extern crate criterion;
use criterion::{
    async_executor::AsyncExecutor, black_box, criterion_group, criterion_main, Criterion,
};
use file_system::FileSystem;
use tokio;

fn bench_sync(c: &mut Criterion) {
    let mut c = Criterion::default().sample_size(10);
    let mut group = c.benchmark_group("file_sytem");
    group.bench_function("sync", |b| {
        b.iter(|| {
            let mut fs =
                FileSystem::new("E:/Games/Steam/steamapps/common/New World/assets").unwrap();
            fs.read_sync("sharedassets/genericassets/playerbaseattributes.pbadb")
                .unwrap();
        })
    });

    group.bench_function("async", |b| {
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

criterion_group!(benches, bench_sync);
criterion_main!(benches);
