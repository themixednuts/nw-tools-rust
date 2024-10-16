mod app;
mod events;
mod resources;

use app::App;
use assets::assetcatalog::AssetCatalog;
use cli::{
    commands::{test::TestCommands, Commands},
    ARGS,
};
use cliclack::{spinner, ProgressBar};
use distribution::*;
use file_system::{FileSystem, State};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use std::{
    path::PathBuf,
    process::ExitCode,
    sync::{
        atomic::{AtomicU64, AtomicUsize, Ordering},
        Arc, LazyLock, RwLock,
    },
};
use tokio::{
    self,
    signal::ctrl_c,
    task::{self},
    time::{self, Duration, Instant},
};
use tracing::instrument;
use tracing_subscriber::FmtSubscriber;
use utils::{format_bytes, format_duration};

#[tokio::main]
#[instrument]
async fn main() -> tokio::io::Result<ExitCode> {
    let subscriber = FmtSubscriber::builder().finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let app = App::init();

    let token = app.cancel.clone();
    tokio::spawn(async move {
        ctrl_c().await.unwrap();
        token.cancel();
    });

    run().await?;

    Ok(ExitCode::SUCCESS)
}

#[instrument]
async fn run() -> tokio::io::Result<()> {
    match &ARGS.command {
        Commands::Extract(extract) => {
            let cwd = extract.common.input.input.as_ref().unwrap();
            let out = extract.common.output.output.as_ref().unwrap();
            let filter = extract.common.filter.filter.as_ref();
            run_extract(cwd, out, filter).await?
        }
        Commands::Test(test) => match &test.commands {
            TestCommands::Filter { input, filter } => {
                let cwd = input.input.as_ref().unwrap();
                let filter = filter.filter.as_ref();
                run_test_filter(cwd, filter).await?
            }
            TestCommands::Distribution { input } => {
                let cwd = input.input.as_ref().unwrap();
                run_test_distribution(cwd).await?
            }
        },
    };

    Ok(())
}

async fn initialize(
    cwd: &'static PathBuf,
    out: &'static PathBuf,
) -> tokio::io::Result<&'static FileSystem> {
    let pb = cliclack::spinner();
    pb.start("Initializing File System");
    let fs = FileSystem::init(cwd, out, App::handle().cancel.clone()).await;
    pb.stop("File System Initialized");

    let pb = cliclack::spinner();
    pb.start("Initializing Asset Catalog");
    let data = fs.open("assetcatalog.catalog")?;
    let _catalog = AssetCatalog::try_from(data.as_slice())?;
    pb.stop("Asset Catalog Initialized");
    Ok(fs)
}

#[instrument]
async fn run_test_filter(cwd: &'static PathBuf, filter: Option<&String>) -> tokio::io::Result<()> {
    static OUT: LazyLock<PathBuf> = LazyLock::new(|| PathBuf::new());
    let fs = initialize(cwd, &OUT).await?;
    let files = fs.files(filter);
    println!("Filter: {:?}", filter);
    for (file_path, (_full_path, _)) in files {
        println!("File: {}", file_path.display());
    }
    Ok(())
}
#[instrument]
async fn run_test_distribution(cwd: &'static PathBuf) -> tokio::io::Result<()> {
    static OUT: LazyLock<PathBuf> = LazyLock::new(|| PathBuf::new());
    let fs = initialize(cwd, &OUT).await?;
    let files = fs.files(Some(&String::from("**/*.distribution")));

    tokio::task::spawn_blocking(move || {
        let multi = cliclack::ProgressBar::new(files.len() as u64);
        multi.start("Starting Distribution tests.");
        files.par_iter().for_each(|(file_path, (_full_path, _))| {
            let entry = fs.open(file_path).unwrap();
            multi.set_message(format!("{}", file_path.parent().unwrap().display()));
            if let Err(e) = Distribution::from_reader(&mut entry.as_slice()) {
                println!("Distribution Failed: {}\n{}", file_path.display(), e);
            };
            multi.inc(1);
        });
        multi.stop("Distribution Tests Done.");
    })
    .await
    .unwrap();

    Ok(())
}

#[instrument]
async fn run_extract(
    cwd: &'static PathBuf,
    out: &'static PathBuf,
    filter: Option<&String>,
) -> tokio::io::Result<()> {
    let fs = initialize(cwd, out).await?;
    let files = fs.files(filter);
    let len = files.len() as u64;

    let multi_pb = Arc::new(cliclack::MultiProgress::new("Extracting Pak(s)"));
    let all = Arc::new(multi_pb.add(ProgressBar::new(len)));
    let stats_pb = Arc::new(multi_pb.add(spinner()));
    let pak_pb = Arc::new(multi_pb.add(spinner()));
    let file_pb = Arc::new(multi_pb.add(spinner()));

    all.start("");
    stats_pb.start("");
    file_pb.start("");
    pak_pb.start("");

    let bytes = Arc::new(AtomicU64::new(0));
    let processed = Arc::new(AtomicU64::new(0));

    let processed_clone = processed.clone();
    let bytes_cloned = Arc::clone(&bytes);
    let start = Instant::now();
    let all_pb = Arc::clone(&all);
    let state = Arc::new(RwLock::new(State {
        active: Arc::new(AtomicUsize::new(0)),
        max: Arc::new(AtomicUsize::new(0)),
        size: Arc::new(AtomicUsize::new(0)),
    }));
    let state_clone = state.clone();
    let stats_pb_clone = stats_pb.clone();

    task::spawn(async move {
        let mut interval = time::interval(Duration::from_millis(1000 / 10));

        loop {
            if App::handle().cancel.is_cancelled() {
                break;
            };

            interval.tick().await;
            let elapsed = start.elapsed();
            let bytes_prcocessed = bytes_cloned.load(Ordering::Relaxed);
            let bytes_per_sec = bytes_prcocessed as f64 / elapsed.as_secs_f64();
            let processed_count = processed_clone.load(Ordering::Relaxed);
            let eta = if processed_count < len {
                let remaining = len - processed_count;
                let time_per_file = elapsed.as_secs_f64() / (processed_count + 1) as f64;
                Duration::from_secs_f64(time_per_file * remaining as f64)
            } else {
                Duration::ZERO
            };

            let state = state_clone.read().unwrap();

            stats_pb_clone.set_message(format!(
                "#Tasks: {} | Max Tasks: {} | #Last Bytes Written: {} ",
                state.active.load(Ordering::Relaxed),
                state.max.load(Ordering::Relaxed),
                format_bytes(state.size.load(Ordering::Relaxed) as f64),
            ));
            all_pb.set_message(format!(
                "ETA: {} | Throughput: {}/s",
                format_duration(eta),
                format_bytes(bytes_per_sec),
            ));
        }
    });

    let multi_pb_clone = multi_pb.clone();
    tokio::spawn(async move {
        App::handle().cancel.cancelled().await;
        multi_pb_clone.println("Aborting...");
        multi_pb_clone.cancel();
    });

    let all_pb = Arc::clone(&all);
    let cloned_processed = processed.clone();
    let bytes_cloned = Arc::clone(&bytes);
    let file_pb_clone = file_pb.clone();
    let pak_pb_clone = pak_pb.clone();

    fs.all(files, state.clone(), move |pak, entry, len, idx, size| {
        bytes_cloned.fetch_add(size, Ordering::Relaxed);
        all_pb.inc(1);
        pak_pb_clone.set_message(format!(
            "{} ({idx}/{len})",
            pak.file_name().unwrap().to_str().unwrap()
        ));

        file_pb_clone.set_message(format!("{}", entry.display()));

        let processed = Arc::clone(&cloned_processed);
        if (processed.fetch_add(1, Ordering::Relaxed) + 1) == len as u64 {};

        Ok(())
    })
    .await?;

    let all_pb = all.clone();
    let file_pb = file_pb.clone();
    let pak_pb = pak_pb.clone();
    all_pb.stop("");
    stats_pb.stop("");
    file_pb.stop("");
    pak_pb.stop("");
    multi_pb.stop();
    let processed = processed.load(Ordering::Relaxed);
    let bytes_cloned = Arc::clone(&bytes);

    cliclack::outro(format!(
        "Processed {}/{} files in {}. Bytes: {}",
        processed,
        len,
        format_duration(start.elapsed()),
        format_bytes(bytes_cloned.load(Ordering::Relaxed) as f64)
    ))
    .unwrap();
    Ok(())
}
