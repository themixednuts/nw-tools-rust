use criterion::{criterion_group, criterion_main, Criterion};
use file_system::FileSystem;
use std::io::Cursor;
use tokio;

fn bench(c: &mut Criterion) {
    // c.bench_function("catalog", |b| {
    //     let catalog = include_bytes!("E:/Extract/NW Live/assetcatalog.catalog");

    //     let runtime = tokio::runtime::Runtime::new().unwrap();
    //     let handle = runtime.handle();
    //     b.to_async(runtime).iter(|| async move {
    //         let mut cursor = Cursor::new(catalog);
    //     })
    // });
}

criterion_group!(benches, bench,);
criterion_main!(benches);
