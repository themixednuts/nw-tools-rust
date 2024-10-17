use criterion::{criterion_group, criterion_main, Criterion};
// use file_system::FileSystem;
// use std::io::Cursor;
// use tokio;

fn bench(c: &mut Criterion) {
    // c.bench_function("catalog", |b| {
    //     let catalog = include_bytes!("E:/Extract/NW Live/assetcatalog.catalog");

    //     let runtime = tokio::runtime::Runtime::new().unwrap();
    //     let handle = runtime.handle();
    //     b.to_async(runtime).iter(|| async move {
    //         let mut cursor = Cursor::new(catalog);
    //     })
    // });

    let len = 10000;

    c.bench_function("loop", |b| {
        b.iter(|| {
            let mut buf: Vec<String> = Vec::with_capacity(len);

            for _ in 0..len {
                buf.push(String::from("i"))
            }
        });
    });
    c.bench_function("iter", |b| {
        b.iter(|| {
            let iter = std::iter::repeat_with(|| String::from("i")).take(len);

            let vec = Vec::from_iter(iter);
        });
    });
}

criterion_group!(benches, bench);
criterion_main!(benches);
