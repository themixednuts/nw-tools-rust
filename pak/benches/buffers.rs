extern crate criterion;

use std::{
    fs::File,
    io::{Cursor, Read},
};

use pak::{self, azcs};

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use zip::ZipArchive;

fn stream(c: &mut Criterion) {
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

    c.bench_function("streams", |b| {
        b.iter(|| {
            let mut reader = Cursor::new(&file_data);
            reader.read_exact(&mut buffer).unwrap();

            let header = azcs::is_azcs(&mut reader, &mut buffer).unwrap();
            let mut buffer = vec![];
            azcs::decompress(black_box(&mut reader), black_box(&header))
                .unwrap()
                .read_to_end(&mut buffer)
                .unwrap();
            azcs::parser(&mut Cursor::new(buffer)).unwrap();
        })
    });
}

criterion_group!(benches, stream);
criterion_main!(benches);
