mod cli;
use clap::Parser;
use cliclack::{spinner, ProgressBar};
use file_system::FileSystem;
use futures::{future::join_all, StreamExt};
use scopeguard::{defer, defer_on_unwind};
use std::{
    borrow::BorrowMut,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};
use tokio::{
    self, pin,
    runtime::Handle,
    signal::ctrl_c,
    task::{self, JoinHandle},
    time::{self, Duration, Instant},
};
use tokio_stream::wrappers::ReceiverStream;
use tokio_util::{compat::FuturesAsyncReadCompatExt, sync::CancellationToken};
use utils::{format_bytes, format_duration, race};

#[tokio::main]
async fn main() -> tokio::io::Result<()> {
    let (tx, mut rx) = tokio::sync::oneshot::channel();
    let cancellation_token = CancellationToken::new();

    let cancellation_token_handle = cancellation_token.clone();
    tokio::spawn(async move {
        ctrl_c().await.unwrap();
        cancellation_token_handle.cancel();
    });

    let cancellation_token_handle = cancellation_token.clone();
    let main_handle = tokio::spawn(async move {
        if let Err(e) = start(cancellation_token_handle).await {
            if let Err(_) = tx.send(e.to_string()) {};
        }
    });
    main_handle.await?;

    // println!("Exiting main");
    Ok(())
}

async fn start(cancellation_token: CancellationToken) -> tokio::io::Result<()> {
    let cancellation_token_handle = cancellation_token.clone();

    let args = tokio::task::spawn_blocking(move || {
        if std::env::args().len() > 1 {
            return cli::Args::parse();
        } else {
            cli::interactive().unwrap()
        }
    })
    .await;

    let Ok(args) = args else { return Ok(()) };
    let input_path = Arc::new(args.input.unwrap());
    let out_path = Arc::new(args.output.unwrap());

    let fs = tokio::spawn(async move {
        let pb = cliclack::spinner();
        pb.start("Initializing File System");
        let p = input_path.to_path_buf();
        let fs = race(FileSystem::init(&p), &cancellation_token_handle)
            .await
            .unwrap()
            .unwrap();
        // pb.set_message("Initializing Asset Catalog");
        // let mut data = fs.get("assetcatalog.catalog").await.unwrap();
        // let _ = AssetCatalog::new(&mut data).await.unwrap();
        pb.stop("File System Initialized");
        fs
    })
    .await?;

    let len = fs.len() as u64;

    let multi_pb = Arc::new(cliclack::MultiProgress::new("Decompressing paks"));
    let all = Arc::new(multi_pb.add(ProgressBar::new(len)));
    all.start("");
    let stats_pb = Arc::new(multi_pb.add(spinner()));
    let pak_pb = Arc::new(multi_pb.add(spinner()));
    let file_pb = Arc::new(multi_pb.add(spinner()));

    let bytes = Arc::new(AtomicU64::new(0));
    let processed = Arc::new(AtomicU64::new(0));
    file_pb.start("");
    stats_pb.start("");
    pak_pb.start("");

    let current_pak = Arc::new(tokio::sync::Mutex::new(String::new()));
    let current_pak_count = Arc::new(AtomicU64::new(0));

    let all_pb = Arc::clone(&all);

    let processed_clone = Arc::clone(&processed);
    let bytes_cloned = Arc::clone(&bytes);

    let start = Instant::now();
    defer_on_unwind!(
        eprintln!(
        "Processed {}/{} files in {}. Bytes: {}",
        processed.load(Ordering::Relaxed),
        len,
        format_duration(start.elapsed()),
        format_bytes(bytes.load(Ordering::Relaxed) as f64),
        );
    );

    let cancellation_token_handle = cancellation_token.clone();
    task::spawn(async move {
        let mut interval = time::interval(Duration::from_millis(1000 / 1)); // 60 FPS

        loop {
            let fut = async {
                interval.tick().await;
                let elapsed = start.elapsed();
                let bytes_per_sec =
                    bytes_cloned.load(Ordering::Relaxed) as f64 / elapsed.as_secs_f64();
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
                    format_bytes(bytes_per_sec)
                ));
            };
            if let None = race(fut, &cancellation_token_handle).await {
                break;
            };
        }
    });

    let cloned_processed = processed.clone();
    let all_pb = Arc::clone(&all);
    let bytes_cloned = Arc::clone(&bytes);

    let (error_tx, mut error_rx) = tokio::sync::mpsc::channel(1);
    let (tx, mut rx) = tokio::sync::mpsc::channel(fs.len());
    let (done_tx, mut done_rx) = tokio::sync::mpsc::channel(1);
    let tx = Arc::new(tx);
    let error_tx = Arc::new(error_tx);
    let done_tx = Arc::new(done_tx);

    let tx_clone = tx.clone();
    let cancellation_token_handle = cancellation_token.clone();
    let tasks_count = Arc::new(AtomicU64::new(0));
    let handle = Handle::current();
    let fut = fs.process_all(move |data, entry, path, pak_len, active, max| {
        let file_path = out_path.join(&*entry);
        let all_pb = Arc::clone(&all_pb);
        let stats_pb = Arc::clone(&stats_pb);
        let file_pb = Arc::clone(&file_pb);
        let pak_pb = Arc::clone(&pak_pb);
        let processed = Arc::clone(&cloned_processed);
        let bytes = Arc::clone(&bytes_cloned);
        let current_pak = Arc::clone(&current_pak);
        let current_pak_count = Arc::clone(&current_pak_count);
        let cancellation_token_handle = cancellation_token.clone();
        let tx = tx.clone();
        let done_tx = done_tx.clone();
        let error_tx_clone = error_tx.clone();
        let tasks_count = tasks_count.clone();
        let cancellation_token_handle2 = cancellation_token.clone();

        let join = handle.spawn(async move {
            let fut = async move {
                let error_tx_clone = error_tx_clone.clone();
                let Some(pak_name) = path.file_name().and_then(|str| Some(str.to_string_lossy()))
                else {
                    if let Ok(permit) = error_tx_clone.reserve().await {
                        permit.send("No file Name".to_string())
                    };
                    return;
                };
                let mut current_pak = current_pak.lock().await;
                if *current_pak != pak_name {
                    *current_pak = pak_name.to_string();
                    current_pak_count.store(0, Ordering::Relaxed);
                }
                current_pak_count.fetch_add(1, Ordering::Relaxed);
                pak_pb.set_message(format!(
                    "{pak_name} ({}/{pak_len})",
                    current_pak_count.load(Ordering::Relaxed)
                ));
                file_pb.set_message(format!("{}", &entry));

                let tasks_count = tasks_count.clone();
                let error_tx_clone = error_tx_clone.clone();
                let processed = Arc::clone(&processed);
                let cancellation_token_handle = cancellation_token_handle.clone();
                let all_pb = Arc::clone(&all_pb);
                let handle = Handle::current();
                tasks_count.fetch_add(1, Ordering::Relaxed);
                let join = handle.spawn(async move {
                    let fut = async move {
                        let Some(parent) = file_path.parent() else {
                            // if let Ok(_permit) = error_tx_clone.reserve().await {
                            //     error_tx_clone
                            //         .send(format!("Couldn't get {} parent", file_path.display()))
                            //         .await
                            //         .unwrap();
                            // }
                            return;
                        };
                        match tokio::fs::create_dir_all(parent).await {
                            Ok(_) => {}
                            Err(e) => {
                                // if let Ok(_permit) = error_tx_clone.reserve().await {
                                //     error_tx_clone.send(e.to_string()).await.unwrap();
                                // }
                                return;
                            }
                        };
                        let file = match tokio::fs::File::create(&file_path).await {
                            Ok(file) => file,
                            Err(e) => {
                                // if let Ok(_permit) = error_tx_clone.reserve().await {
                                //     error_tx_clone.send(e.to_string()).await.unwrap();
                                // }
                                return;
                            }
                        };

                        let mut data = futures::io::Cursor::new(data).compat();
                        let mut file = tokio::io::BufWriter::new(file);

                        let bytes_size = match tokio::io::copy(&mut data, &mut file).await {
                            Ok(b) => b,
                            Err(e) => {
                                // if let Ok(_permit) = error_tx_clone.reserve().await {
                                //     error_tx_clone.send(e.to_string()).await.unwrap();
                                // }
                                return;
                            }
                        };
                        all_pb.inc(1);

                        bytes.fetch_add(bytes_size, Ordering::Relaxed);
                        if (processed.fetch_add(1, Ordering::Relaxed) + 1) == len {
                            done_tx.send(true).await.unwrap();
                        };

                        stats_pb.set_message(format!(
                            "Active Processing Threads: {} | Max Processing Threads: {} | Active Writing Tasks: {} | Last Bytes Written: {} ",
                            active,
                            max,
                            tasks_count.fetch_sub(1, Ordering::Relaxed),
                            format_bytes(bytes_size as f64),
                        ));
                    };
                    let cancellation_token_handle = cancellation_token_handle.clone();
                    race(fut, cancellation_token_handle).await;
                });
                let tx = tx.clone();
                if let Ok(permit) = tx.reserve().await {
                    permit.send(join);
                };
            };

            let cancellation_token_handle2 = cancellation_token_handle2.clone();
            race(fut, cancellation_token_handle2).await;
        });

        if let Ok(permit) = tx_clone.try_reserve() {
            permit.send(join);
        };
    });

    let multi_pb_clone = Arc::clone(&multi_pb);
    let mut join_handles = Vec::new();

    pin!(fut);

    let cancellation_token_handle = cancellation_token_handle.clone();
    loop {
        tokio::select! {
            biased;
            _ = cancellation_token_handle.cancelled() => {
                multi_pb_clone.error("Aborted");
                break;
            },
            Some(r) = error_rx.recv() => {
                cancellation_token_handle.cancel();
                multi_pb_clone.error(&r);
                break;
            },
            s = rx.recv() => {
                match s {
                    Some(s) => {
                        join_handles.push(s);
                    },
                    None => {
                        multi_pb_clone.stop();
                        break;
                    }
                }
            },
            Some(_) = done_rx.recv() => {
               rx.close();
            },
            r = &mut fut => {
                if let Err(e) = r {
                    multi_pb_clone.error(&e);
                    break;
                };
            },
        }
    }
    join_all(join_handles).await;

    let processed = processed.load(Ordering::Relaxed);

    let bytes_cloned = Arc::clone(&bytes);
    cliclack::outro(format!(
        "Processed {}/{} files in {}. Bytes: {}",
        processed,
        len,
        format_duration(start.elapsed()),
        format_bytes(bytes_cloned.load(Ordering::Relaxed) as f64)
    ))?;

    Ok(())
}
