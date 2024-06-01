extern crate criterion;
use std::io::{Cursor, Read, Seek};

use criterion::{criterion_group, criterion_main, Criterion};
use file_system::FileSystem;
use tokio;

fn bench_sync(c: &mut Criterion) {
    let mut group = c.benchmark_group("file_sytem");
    let mut fs = FileSystem::new("E:/Games/Steam/steamapps/common/New World/assets").unwrap();
    group.sample_size(10).bench_function("sync", |b| {
        b.iter(|| {
            fs.read_sync("sharedassets/genericassets/playerbaseattributes.pbadb")
                .unwrap();
        })
    });

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

fn bench_datasheet(c: &mut Criterion) {
    let mut group = c.benchmark_group("datasheet");

    let mut fs = FileSystem::new("E:/Games/Steam/steamapps/common/New World/assets").unwrap();
    let mut reader = fs
        .read_sync("sharedassets/springboardentitites/datatables/javelindata_affixstats.datasheet")
        .unwrap();

    let mut buffer = vec![];
    reader.read_to_end(&mut buffer).unwrap();
    let mut buffer = Cursor::new(buffer);

    group.bench_function("normal", |b| {
        b.iter(|| {
            buffer.rewind().unwrap();
            datasheet::parse_datasheet(&mut buffer).unwrap();
        });
    });
    buffer.rewind().unwrap();

    dbg!(buffer.stream_position().unwrap());
    group.bench_function("no cursor", |b| {
        b.iter(|| {
            buffer.rewind().unwrap();
            datasheet::parse_datasheet_test(&mut buffer).unwrap();
        });
    });
}

criterion_group!(benches, bench_datasheet);
criterion_main!(benches);
