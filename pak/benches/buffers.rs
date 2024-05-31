extern crate criterion;

use std::{
    fs::File,
    io::{Cursor, Read},
};

use pak::{
    self,
    azcs::{self, is_azcs},
    PakFile,
};

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use zip::ZipArchive;

fn without_bufferpool(c: &mut Criterion) {
    let file =
        File::open("E:/Games/Steam/steamapps/common/New World/assets/SharedDataStrm-part8.pak")
            .unwrap();
    let mut archive = ZipArchive::new(file).unwrap();
    let target_name = "sharedassets/genericassets/playerbaseattributes.pbadb";

    let target_index = archive
        .file_names()
        .enumerate()
        .find_map(
            |(idx, name)| {
                if name == target_name {
                    Some(idx)
                } else {
                    None
                }
            },
        )
        .unwrap_or_else(|| panic!("Entry '{}' not found", target_name));

    // Now you can access the file by its index

    let mut buffer = [0; 5];
    let mut file_data = vec![];
    archive
        .by_index_raw(target_index)
        .unwrap()
        .read_to_end(&mut file_data)
        .unwrap();

    c.bench_function("without_bufferpool", |b| {
        b.iter(|| {
            let mut reader = Cursor::new(&file_data);
            reader.read_exact(&mut buffer).unwrap();

            let header = azcs::is_azcs(&mut reader, &mut buffer).unwrap();
            azcs::parser(
                &mut azcs::decompress(black_box(&mut reader), black_box(&header)).unwrap(),
            )
            .unwrap();
        })
    });
}
fn with_bufferpool(c: &mut Criterion) {
    let file =
        File::open("E:/Games/Steam/steamapps/common/New World/assets/SharedDataStrm-part8.pak")
            .unwrap();
    let mut archive = ZipArchive::new(file).unwrap();
    let target_name = "sharedassets/genericassets/playerbaseattributes.pbadb";

    let target_index = archive
        .file_names()
        .enumerate()
        .find_map(
            |(idx, name)| {
                if name == target_name {
                    Some(idx)
                } else {
                    None
                }
            },
        )
        .unwrap_or_else(|| panic!("Entry '{}' not found", target_name));

    // Now you can access the file by its index

    let mut buffer = [0; 5];
    let mut file_data = vec![];
    archive
        .by_index_raw(target_index)
        .unwrap()
        .read_to_end(&mut file_data)
        .unwrap();
    c.bench_function("with_bufferpool", |b| {
        b.iter(|| {
            let mut reader = Cursor::new(&file_data);
            reader.read_exact(&mut buffer).unwrap();

            let header = azcs::is_azcs(&mut reader, &mut buffer).unwrap();
            azcs::parser(
                &mut azcs::decompress(black_box(&mut reader), black_box(&header)).unwrap(),
            )
            .unwrap();
        });
    });
}

criterion_group!(benches, without_bufferpool, with_bufferpool);
criterion_main!(benches);
