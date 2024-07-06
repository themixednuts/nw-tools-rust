mod app;
mod cli;
mod events;

use clap::Parser;
use cliclack::{spinner, ProgressBar};
use file_system::{decompressor::decompress_zip, FileSystem};
use scopeguard::defer_on_unwind;
use std::{
    borrow::Borrow,
    io::{self, Cursor, Read, Seek, Write},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};
use tokio::{
    self,
    runtime::Handle,
    signal::ctrl_c,
    task::{self, JoinHandle},
    time::{self, Duration, Instant},
};
use tokio_util::sync::CancellationToken;
use utils::{format_bytes, format_duration, race};
use zip::read::ZipFile;

#[tokio::main]
async fn main() -> tokio::io::Result<()> {
    let cancellation_token = CancellationToken::new();

    let cancellation_token_handle = cancellation_token.clone();
    tokio::spawn(async move {
        ctrl_c().await.unwrap();
        cancellation_token_handle.cancel();
    });

    let cancellation_token_handle = cancellation_token.clone();
    start(cancellation_token_handle).await?;

    Ok(())
}

async fn start(cancellation_token: CancellationToken) -> tokio::io::Result<()> {
    let cancellation_token_handle = cancellation_token.clone();

    let args = tokio::task::spawn_blocking(move || {
        if std::env::args().len() > 1 {
            return Ok(cli::Args::parse());
        } else {
            match cli::interactive() {
                Ok(args) => Ok(args),
                Err(e) => Err(e),
            }
        }
    })
    .await;

    let Ok(Ok(args)) = args else { return Ok(()) };

    let input_path = Arc::new(args.input.unwrap());
    assert!(input_path.join("assets").exists());
    let out_path = Arc::new(args.output.unwrap());

    let fs = tokio::spawn(async move {
        let pb = cliclack::spinner();
        pb.start("Initializing File System");
        let fs = FileSystem::init(&input_path.as_ref(), &out_path.as_ref()).await;
        // pb.set_message("Initializing Asset Catalog");
        // let mut data = fs.get("assetcatalog.catalog").await.unwrap();
        // let _ = AssetCatalog::new(&mut data).await.unwrap();
        pb.stop("File System Initialized");
        fs
    })
    .await?;

    let len = fs.len() as u64;

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
        let mut interval = time::interval(Duration::from_millis(1000 / 120)); // 60 FPS

        loop {
            interval.tick().await;
            let elapsed = start.elapsed();
            let bytes_per_sec = bytes_cloned.load(Ordering::Relaxed) as f64 / elapsed.as_secs_f64();
            let processed_count = processed_clone.load(Ordering::Relaxed);
            let eta = if processed_count < len {
                let remaining = len - processed_count;
                let time_per_file = elapsed.as_secs_f64() / (processed_count + 1) as f64;
                Duration::from_secs_f64(time_per_file * remaining as f64)
            } else {
                Duration::ZERO
            };

            all_pb.set_message(format!(
                "ETA: {} | Throughput: {}/s",
                format_duration(eta),
                format_bytes(bytes_per_sec),
            ));
        }
    });

    let cancellation_token_handle = cancellation_token.clone();
    let status_pb_clone = status_pb.clone();
    tokio::spawn(async move {
        cancellation_token_handle.cancelled().await;
        status_pb_clone.set_message("Aborting...");
    });

    let all_pb = Arc::clone(&all);
    let cloned_processed = processed.clone();
    let bytes_cloned = Arc::clone(&bytes);

    // let (term_tx, mut term_rx) = tokio::sync::mpsc::channel(1);
    // let term_tx = Arc::new(term_tx);
    let cancellation_token_handle = cancellation_token.clone();
    fs.process_all(move |pak, entry, len, active, max, idx, size| {
        if cancellation_token_handle.is_cancelled() {
            return Err(tokio::io::Error::other("task cancelled"));
        }
        pak_pb.set_message(format!(
            "{} ({idx}/{len})",
            pak.file_name().unwrap().to_str().unwrap()
        ));

        file_pb.set_message(format!("{}", entry.display()));
        let all_pb = Arc::clone(&all_pb);

        let processed = Arc::clone(&cloned_processed);
        all_pb.inc(1);

        bytes_cloned.fetch_add(size, Ordering::Relaxed);
        if (processed.fetch_add(1, Ordering::Relaxed) + 1) == len as u64 {
            // done_tx.blocking_send(true).unwrap();
        };

        stats_pb.set_message(format!(
            "#Processing Threads: {} | Max Threads: {} | #Last Bytes Written: {} ",
            active,
            max,
            format_bytes(size as f64),
        ));
        Ok(())
    })
    .await?;

    // match join {
    //     Ok(res) => {
    //         if let Err(e) = res.await {
    //             dbg!(e);
    //         }
    //     }
    //     Err(e) => {
    //         dbg!(e);
    //     }
    // }
    let cancellation_token_handle = cancellation_token.clone();
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
