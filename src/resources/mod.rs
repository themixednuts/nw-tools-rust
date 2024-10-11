use std::{
    collections::HashMap,
    io::{BufRead, Read, Write},
    path::Path,
    sync::{Arc, Mutex},
};

use crc32fast::Hasher;
use futures::FutureExt;
use rayon::prelude::*;
use regex::{self, Regex};
use serde_json::json;
use tokio::{io::AsyncReadExt, runtime::Handle, task};
use walkdir::{DirEntry, WalkDir};

pub async fn map_resources() {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let (done_tx, done_rx) = tokio::sync::oneshot::channel();

    let tx = Arc::new(tx);
    let task = task::spawn(async move {
        let re: Regex = Regex::new(r#"(\w+)\s*=\s*"([^"]*)""#).unwrap();
        let mut uuids = HashMap::new();
        let mut crcs = HashMap::new();

        let mut done_rx = done_rx.fuse();

        loop {
            tokio::select! {
                _ = &mut done_rx => {
                    rx.close();
                },
               res = rx.recv() => {
                   let Some((file, contents)) = res else {
                       break;
                   };
                     parse(file, contents, &mut crcs, &mut uuids, &re)
               },
            }
        }
        (uuids, crcs)
    });

    let tx1 = tx.clone();
    WalkDir::new("E:/Extract/nw/assets")
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|path| path.file_type().is_file())
        .par_bridge()
        .for_each(|file| {
            let Some(contents) = dir_entry(&file) else {
                return;
            };
            tx1.send((file, contents)).unwrap();
        });

    let tx2 = tx.clone();
    WalkDir::new("E:/Docs/nw-tools-rust/file-system/resources")
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|path| path.file_type().is_file())
        .par_bridge()
        .for_each(|file| {
            let Some(contents) = dir_entry(&file) else {
                return;
            };
            tx2.send((file, contents)).unwrap();
        });

    done_tx.send(()).unwrap();
    let (uuids, crcs) = task.await.unwrap();

    let uuid_path = Path::new("E:/docs/nw-tools-rust").join("uuids.json");
    let crcs_path = Path::new("E:/docs/nw-tools-rust").join("crcs.json");

    std::fs::File::create(uuid_path)
        .unwrap()
        .write_all(
            serde_json::to_string_pretty(&json!(uuids))
                .unwrap()
                .as_bytes(),
        )
        .unwrap();
    std::fs::File::create(crcs_path)
        .unwrap()
        .write_all(
            serde_json::to_string_pretty(&json!(crcs))
                .unwrap()
                .as_bytes(),
        )
        .unwrap();
}

fn dir_entry(file: &DirEntry) -> Option<Vec<u8>> {
    let f = std::fs::File::open(file.path()).unwrap();
    let mut f = std::io::BufReader::new(f);
    let mut buf = [0; 13];
    if let Err(_) = f.read_exact(&mut buf) {
        return None;
    };

    if &buf != b"<ObjectStream" {
        return None;
    };

    let mut contents = vec![];
    contents.extend_from_slice(&buf);

    f.read_to_end(&mut contents).unwrap();
    Some(contents)
}

fn parse(
    file: DirEntry,
    contents: Vec<u8>,
    crcs: &mut HashMap<u32, String>,
    uuids: &mut HashMap<String, String>,
    re: &Regex,
) {
    println!("Parsing {}", file.file_name().to_string_lossy());
    contents.lines().skip(1).for_each(|line| {
        let ln = line.unwrap();
        let fields = ln
            .trim()
            .trim_start_matches("<Class ")
            .trim_end_matches(">")
            .trim_end_matches("/");

        let mut name = String::new();
        for caps in re.captures_iter(fields) {
            let key = caps.get(1).map_or("", |m| m.as_str());
            let value = caps.get(2).map_or("", |m| m.as_str());

            if key == "name" {
                name = value.to_owned();
                let mut hasher = Hasher::new();
                hasher.update(value.to_lowercase().as_bytes());
                let crc = hasher.finalize();

                crcs.insert(crc, value.to_owned());
            } else if key == "field" {
                let mut hasher = Hasher::new();
                hasher.update(value.to_lowercase().as_bytes());
                let crc = hasher.finalize();

                crcs.insert(crc, value.to_owned());
            }
            if key == "type" {
                if !&name.is_empty() {
                    uuids.insert(value.to_owned(), name.to_owned());
                }
            }
        }
    });
}
