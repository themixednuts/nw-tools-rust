use std::{
    collections::HashMap,
    io::{BufRead, Read, Write},
    sync::{Arc, Mutex},
};

use crc32fast::Hasher;
use rayon::iter::{ParallelBridge, ParallelIterator};
use regex::Regex;
use serde_json::json;
use walkdir::{DirEntry, WalkDir};

fn dir_entry(
    file: DirEntry,
    crcs: &Arc<Mutex<HashMap<u32, String>>>,
    uuids: &Arc<Mutex<HashMap<String, String>>>,
    re: &Regex,
) {
    let mut f = std::fs::File::open(file.path()).unwrap();
    let mut buf = [0; 13];
    if let Err(_) = f.read_exact(&mut buf) {
        return;
    };

    if &buf != b"<ObjectStream" {
        return;
    };

    println!("Parsing {}", file.file_name().to_string_lossy());

    let mut contents = vec![];
    contents.extend_from_slice(&buf);

    f.read_to_end(&mut contents).unwrap();
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
                let lowercase_crc = hasher.finalize();

                crcs.lock().unwrap().insert(lowercase_crc, value.to_owned());
            } else if key == "field" {
                let mut hasher = Hasher::new();
                hasher.update(value.to_lowercase().as_bytes());
                let lowercase_crc = hasher.finalize();

                crcs.lock().unwrap().insert(lowercase_crc, value.to_owned());
            }
            if key == "type" {
                if !&name.is_empty() {
                    uuids
                        .lock()
                        .unwrap()
                        .insert(value.to_owned(), name.to_owned());
                }
            }
        }
    });
}

fn main() {
    let re = Regex::new(r#"(\w+)\s*=\s*"([^"]*)""#).unwrap();
    let uuids = Arc::new(Mutex::new(HashMap::new()));
    let crcs = Arc::new(Mutex::new(HashMap::new()));

    WalkDir::new("E:/Extract/NW/assets")
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|path| path.file_type().is_file())
        .par_bridge()
        .for_each(|file| dir_entry(file, &crcs, &uuids, &re));

    WalkDir::new("E:/Docs/nw-tools-rust/object-stream/resources")
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|path| path.file_type().is_file())
        .par_bridge()
        .for_each(|file| dir_entry(file, &crcs, &uuids, &re));

    std::fs::File::create("uuids.json")
        .unwrap()
        .write_all(
            serde_json::to_string_pretty(&json!(*uuids))
                .unwrap()
                .as_bytes(),
        )
        .unwrap();
    std::fs::File::create("crcs.json")
        .unwrap()
        .write_all(
            serde_json::to_string_pretty(&json!(*crcs))
                .unwrap()
                .as_bytes(),
        )
        .unwrap();
}
