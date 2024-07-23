use assets::assetcatalog::AssetCatalog;
use criterion::{criterion_group, criterion_main, Criterion};
use std::io::Cursor;

fn bench(c: &mut Criterion) {
    c.bench_function("catalog", |b| {
        let catalog = include_bytes!("E:/Extract/NW Live/assetcatalog.catalog");

        b.to_async(tokio::runtime::Runtime::new().unwrap())
            .iter(|| async move {
                let mut cursor = Cursor::new(catalog);
                // AssetCatalog::init(&mut cursor).await.unwrap();
            })
    });
}

criterion_group!(benches, bench,);
criterion_main!(benches);
