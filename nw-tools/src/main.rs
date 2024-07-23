mod app;
mod events;
mod resources;

use app::App;
use assets::assetcatalog::AssetCatalog;
use cliclack::{spinner, ProgressBar};
use file_system::FileSystem;
use scopeguard::defer_on_unwind;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};
use tokio::{
    self,
    signal::ctrl_c,
    task::{self},
    time::{self, Duration, Instant},
};
use utils::{format_bytes, format_duration};

#[tokio::main]
async fn main() -> tokio::io::Result<()> {
    let app = App::init();
    // parse_lumberyard_source().await.unwrap();
    // map_resources().await;
    // return Ok(());

    let token = app.notify.clone();
    tokio::spawn(async move {
        ctrl_c().await.unwrap();
        token.cancel();
    });

    start().await?;

    Ok(())
}

async fn start() -> tokio::io::Result<()> {
    // TODO: handle this differently
    let args = tokio::task::spawn_blocking(cli::interactive).await;
    let Ok(Ok(args)) = args else { return Ok(()) };

    let input_path = Arc::new(args.input.as_ref().unwrap());
    assert!(input_path.join("assets").exists());
    let out_path = Arc::new(args.output.as_ref().unwrap());

    let fs = tokio::spawn(async move {
        let pb = cliclack::spinner();
        pb.start("Initializing File System");
        let fs = FileSystem::init().await;
        pb.set_message("File System Initialized");
        pb.set_message("Initializing Asset Catalog");
        let _ = AssetCatalog::init().await.unwrap();
        pb.stop("Asset Catalog Initialized");
        fs
    })
    .await?;

    // tokio::spawn(async move {
    //     let mut file = tokio::fs::File::open("E:/Extract/NW/assets/assetcatalog.xml")
    //         .await
    //         .unwrap()
    //         .into_std()
    //         .await;

    //     let mut out = tokio::fs::File::create("e:\\extract\\assetcatalog.xml")
    //         .await
    //         .unwrap()
    //         .into_std()
    //         .await;

    //     object_stream::parser(&mut file)
    //         .unwrap()
    //         .to_xml(&mut out, &fs.uuids, &fs.crcs);
    // })
    // .await?;

    let len = fs.len() as u64;
    let size = fs.size();

    let multi_pb = Arc::new(cliclack::MultiProgress::new("Decompressing paks"));
    let status_pb = Arc::new(multi_pb.add(spinner()));
    let all = Arc::new(multi_pb.add(ProgressBar::new(len)));
    let stats_pb = Arc::new(multi_pb.add(spinner()));
    let pak_pb = Arc::new(multi_pb.add(spinner()));
    let file_pb = Arc::new(multi_pb.add(spinner()));

    status_pb.start("Processing...");
    all.start("");
    file_pb.start("");
    stats_pb.start("");
    pak_pb.start("");

    let bytes = Arc::new(AtomicU64::new(0));
    let processed = Arc::new(AtomicU64::new(0));

    let processed_clone = Arc::clone(&processed);
    let bytes_cloned = Arc::clone(&bytes);

    // let status_pb_clone = status_pb.clone();
    // defer_on_unwind!(
    //    status_pb_clone.stop("Aborted");
    //    eprintln!("WHYYYYYYY");
    // );

    let start = Instant::now();
    let all_pb = Arc::clone(&all);
    task::spawn(async move {
        let mut interval = time::interval(Duration::from_millis(1000 / 120));

        loop {
            interval.tick().await;
            let elapsed = start.elapsed();
            let bytes_prcocessed = bytes_cloned.load(Ordering::Relaxed);
            let bytes_per_sec = bytes_prcocessed as f64 / elapsed.as_secs_f64();
            let processed_count = processed_clone.load(Ordering::Relaxed);
            let eta = if processed_count < len {
                // let remaining = len - processed_count;
                // let time_per_file = elapsed.as_secs_f64() / (processed_count + 1) as f64;
                // Duration::from_secs_f64(time_per_file * remaining as f64)
                let remaining = size - bytes_prcocessed as u128;
                if bytes_per_sec > 0.0 {
                    Duration::from_secs_f64(remaining as f64 / bytes_per_sec)
                } else {
                    Duration::ZERO
                }
            } else {
                Duration::ZERO
            };

            all_pb.set_message(format!(
                "ETA: {} | Throughput: {}/s | Approx. Size: {}/{}",
                format_duration(eta),
                format_bytes(bytes_per_sec),
                format_bytes(bytes_prcocessed as f64),
                format_bytes(size as f64)
            ));
        }
    });

    // let cancellation_token_handle = cancellation_token.clone();
    // let status_pb_clone = status_pb.clone();
    // tokio::spawn(async move {
    //     cancellation_token_handle.cancelled().await;
    //     status_pb_clone.set_message("Aborting...");
    // });

    let all_pb = Arc::clone(&all);
    let cloned_processed = processed.clone();
    let bytes_cloned = Arc::clone(&bytes);

    // let (term_tx, mut term_rx) = tokio::sync::mpsc::channel(1);
    // let term_tx = Arc::new(term_tx);
    // let cancellation_token_handle = cancellation_token.clone();
    fs.process_all(move |pak, entry, len, active, max, idx, size| {
        // if cancellation_token_handle.is_cancelled() {
        //     return Err(tokio::io::Error::other("task cancelled"));
        // }
        pak_pb.set_message(format!(
            "{} ({idx}/{len})",
            pak.file_name().unwrap().to_str().unwrap()
        ));

        file_pb.set_message(format!("{}", entry.display()));
        let all_pb = Arc::clone(&all_pb);
        all_pb.inc(1);

        bytes_cloned.fetch_add(size, Ordering::Relaxed);
        let processed = Arc::clone(&cloned_processed);
        if (processed.fetch_add(1, Ordering::Relaxed) + 1) == len as u64 {
            // done_tx.blocking_send(true).unwrap();
        };

        stats_pb.set_message(format!(
            "#Tasks: {} | Max Tasks: {} | #Last Bytes Written: {} ",
            active,
            max,
            format_bytes(size as f64),
        ));
        Ok(())
    })
    .await?;

    // let multi_pb_clone = Arc::clone(&multi_pb);
    // tokio::spawn(async move {
    //     let Some(err) = term_rx.recv().await else {
    //         return;
    //     };
    //     cancellation_token_handle.cancel();
    //     multi_pb_clone.error(err.to_string());
    //     panic!("{}", err.to_string());
    // });

    let processed = processed.load(Ordering::Relaxed);
    let bytes_cloned = Arc::clone(&bytes);
    multi_pb.stop();

    cliclack::outro(format!(
        "Processed {}/{} files in {}. Bytes: {}",
        processed,
        len,
        format_duration(start.elapsed()),
        format_bytes(bytes_cloned.load(Ordering::Relaxed) as f64)
    ))?;

    Ok(())
}
