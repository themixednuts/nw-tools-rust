extern crate criterion;
use std::{
    collections::HashMap,
    io::{Cursor, Read, Seek},
    path::PathBuf,
};

use criterion::{criterion_group, criterion_main, Criterion};
use file_system::FileSystem;
use futures::{self, future::join_all, FutureExt, StreamExt};
use rayon::iter::{IntoParallelIterator, ParallelBridge, ParallelIterator};
use tokio::{self, fs::File};
use walkdir::WalkDir;
use zip::ZipArchive;

fn bench_sync(c: &mut Criterion) {
    let mut group = c.benchmark_group("file_sytem");
    let mut fs = FileSystem::new("E:/Games/Steam/steamapps/common/New World/assets").unwrap();

    group.sample_size(10).bench_function("async", |b| {
        b.to_async(tokio::runtime::Runtime::new().unwrap())
            .iter(|| async {
                let mut fs =
                    FileSystem::new("E:/Games/Steam/steamapps/common/New World/assets").unwrap();
                fs.read("sharedassets/genericassets/playerbaseattributes.pbadb")
                    .await
                    .unwrap();
            })
    });
}

fn index(c: &mut Criterion) {
    c.bench_function("pak index", |b| {
        b.iter(|| {
            let files = WalkDir::new("E:/Games/Steam/steamapps/common/New World/assets")
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|path| {
                    path.file_type().is_file()
                        && path.path().extension().and_then(|ext| ext.to_str()) == Some("pak")
                })
                .par_bridge()
                .map(|dir| {
                    let file = std::fs::File::open(dir.path()).unwrap();
                    let archive = ZipArchive::new(file).unwrap();
                    archive
                        .file_names()
                        .map(|name| (name.to_owned(), dir.path().to_path_buf()))
                        .collect::<Vec<(String, PathBuf)>>()
                })
                .collect::<Vec<_>>();
        });
    });
}

// fn to_json(c: &mut Criterion) {
//     let mut fs = FileSystem::new("E:/Games/Steam/steamapps/common/New World/assets").unwrap();
//     let mut reader = fs
//         .read_sync("sharedassets/springboardentitites/datatables/javelindata_affixstats.datasheet")
//         .unwrap();

//     let mut buffer = vec![];
//     reader.read_to_end(&mut buffer).unwrap();
//     let mut buffer = Cursor::new(buffer);
//     let datasheet = datasheet::Datasheet::from(&mut buffer);
//     c.bench_function("to_json", |b| {
//         b.iter(|| {
//             datasheet.to_json();
//         });
//     });
// }

// fn to_json_simd(c: &mut Criterion) {
//     let mut fs = FileSystem::new("E:/Games/Steam/steamapps/common/New World/assets").unwrap();
//     let mut reader = fs
//         .read_sync("sharedassets/springboardentitites/datatables/javelindata_affixstats.datasheet")
//         .unwrap();

//     let mut buffer = vec![];
//     reader.read_to_end(&mut buffer).unwrap();
//     let mut buffer = Cursor::new(buffer);
//     let datasheet = datasheet::Datasheet::from(&mut buffer);
//     c.bench_function("to_json_simd", |b| {
//         b.iter(|| {
//             datasheet.to_json_simd().unwrap();
//         });
//     });
// }

// fn to_csv(c: &mut Criterion) {
//     let mut fs = FileSystem::new("E:/Games/Steam/steamapps/common/New World/assets").unwrap();
//     let mut reader = fs
//         .read_sync("sharedassets/springboardentitites/datatables/javelindata_affixstats.datasheet")
//         .unwrap();

//     let mut buffer = vec![];
//     reader.read_to_end(&mut buffer).unwrap();
//     let mut buffer = Cursor::new(buffer);
//     let datasheet = datasheet::Datasheet::from(&mut buffer);
//     c.bench_function("to_csv", |b| {
//         b.iter(|| {
//             datasheet.to_csv();
//         });
//     });
// }
// fn to_yaml(c: &mut Criterion) {
//     let mut fs = FileSystem::new("E:/Games/Steam/steamapps/common/New World/assets").unwrap();
//     let mut reader = fs
//         .read_sync("sharedassets/springboardentitites/datatables/javelindata_affixstats.datasheet")
//         .unwrap();

//     let mut buffer = vec![];
//     reader.read_to_end(&mut buffer).unwrap();
//     let mut buffer = Cursor::new(buffer);
//     let datasheet = datasheet::Datasheet::from(&mut buffer);
//     c.bench_function("to_yaml", |b| {
//         b.iter(|| {
//             datasheet.to_yaml();
//         });
//     });
// }

criterion_group!(
    benches,
    index,
    // parse,
    // to_json,
    // to_json_simd,
    // to_csv,
    // to_yaml
);
criterion_main!(benches);
