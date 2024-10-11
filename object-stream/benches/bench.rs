use criterion::{criterion_group, criterion_main, Criterion};
use object_stream::{from_reader, ser};
use std::io::Cursor;

fn bench(c: &mut Criterion) {
    // let f1 = include_bytes!("E:/Extract/nw live/sharedassets/genericassets/fuelcategory.fueldb");
    // let f2 =
    //     include_bytes!("E:/Extract/nw live/sharedassets/genericassets/playerbaseattributes.pbadb");
    // let f3 =
    //     include_bytes!("E:/Extract/nw live/sharedassets/genericassets/rangedattackdatabase.radb");
    // let mut group = c.benchmark_group("Serialization Speed");

    // let o1 = from_reader(&mut Cursor::new(f1)).unwrap();
    // let o2 = from_reader(&mut Cursor::new(f2)).unwrap();
    // let o3 = from_reader(&mut Cursor::new(f3)).unwrap();

    // group.bench_with_input("Non-serde fuelcategory", &o1, |b, f| {
    //     let mut buf = Vec::with_capacity(f1.len());
    //     b.iter(|| {
    //         f.to_writer(&mut buf).unwrap();
    //     })
    // });
    // group.bench_with_input("Serde fuelcategory", &o1, |b, f| {
    //     let mut buf = Vec::with_capacity(f1.len());
    //     b.iter(|| {
    //         ser::to_writer(f, &mut buf).unwrap();
    //     })
    // });
    // group.bench_with_input("Non-serde playerbase", &o2, |b, f| {
    //     let mut buf = Vec::with_capacity(f2.len());
    //     b.iter(|| {
    //         f.to_writer(&mut buf).unwrap();
    //     })
    // });
    // group.bench_with_input("Serde playerbase", &o2, |b, f| {
    //     let mut buf = Vec::with_capacity(f2.len());
    //     b.iter(|| {
    //         ser::to_writer(f, &mut buf).unwrap();
    //     })
    // });
    // group.bench_with_input("Non-serde rangedattack", &o3, |b, f| {
    //     let mut buf = Vec::with_capacity(f3.len());
    //     b.iter(|| {
    //         f.to_writer(&mut buf).unwrap();
    //     })
    // });
    // group.bench_with_input("Serde rangedattack", &o3, |b, f| {
    //     let mut buf = Vec::with_capacity(f3.len());
    //     b.iter(|| {
    //         ser::to_writer(f, &mut buf).unwrap();
    //     })
    // });
}

criterion_group!(benches, bench,);
criterion_main!(benches);
