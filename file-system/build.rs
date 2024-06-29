// use rayon::iter::{ParallelBridge, ParallelIterator};
// use std::{
//     env,
//     io::Write,
//     path::PathBuf,
//     sync::{Arc, Mutex},
// };
// use walkdir::WalkDir;
// use zip::ZipArchive;

fn main() -> std::io::Result<()> {
    // let nw_dir = match std::env::var("NW_DIR") {
    //     Ok(dir) => PathBuf::from(dir),
    //     _ => find_nw_dir().expect("New World directory not found in $PATH"),
    // };

    // let out_dir = env::var("OUT_DIR").unwrap();
    // let dest_path = PathBuf::from(out_dir).join("assets_path_to_pak.txt");

    // let file = Arc::new(Mutex::new(std::fs::File::create(&dest_path)?));

    // WalkDir::new(nw_dir.join("assets"))
    //     .into_iter()
    //     .filter_map(|e| e.ok())
    //     .filter(|path| {
    //         path.file_type().is_file()
    //             && path.path().extension().and_then(|ext| ext.to_str()) == Some("pak")
    //     })
    //     .par_bridge()
    //     .map(|dir| {
    //         let file = std::fs::File::open(dir.path()).unwrap();
    //         let archive = ZipArchive::new(file).unwrap();
    //         archive
    //             .file_names()
    //             .map(|name| (name.to_owned(), dir.path().to_path_buf()))
    //             .collect::<Vec<(String, PathBuf)>>()
    //     })
    //     .flatten()
    //     .try_for_each(|(asset, pak_path)| {
    //         file.lock().unwrap().write_all(
    //             format!(
    //                 "{},{}\n",
    //                 asset,
    //                 pak_path.to_string_lossy().replace("\\", "/"),
    //             )
    //             .as_bytes(),
    //         )
    //     })?;

    Ok(())
}

// fn find_nw_dir() -> Option<PathBuf> {
//     #[cfg(target_os = "windows")]
//     {
//         let nw_dir = PathBuf::from("C:\\")
//             .join("Program Files (x86)")
//             .join("Steam")
//             .join("steamapps")
//             .join("common")
//             .join("New World");
//         if nw_dir.exists() {
//             return Some(nw_dir);
//         }
//     }

//     None
// }

// let re = Regex::new(r#"(\w+)\s*=\s*"([^"]*)""#).unwrap();
// let uuids = Arc::new(Mutex::new(HashMap::new()));
// let crcs = Arc::new(Mutex::new(HashMap::new()));

// WalkDir::new("E:/Extract/NW/assets")
//     .into_iter()
//     .filter_map(|e| e.ok())
//     .filter(|path| path.file_type().is_file())
//     .par_bridge()
//     .for_each(|file| dir_entry(file, &crcs, &uuids, &re));

// WalkDir::new("E:/Docs/nw-tools-rust/object-stream/resources")
//     .into_iter()
//     .filter_map(|e| e.ok())
//     .filter(|path| path.file_type().is_file())
//     .par_bridge()
//     .for_each(|file| dir_entry(file, &crcs, &uuids, &re));

// std::fs::File::create("uuids.json")
//     .unwrap()
//     .write_all(
//         serde_json::to_string_pretty(&json!(*uuids))
//             .unwrap()
//             .as_bytes(),
//     )
//     .unwrap();
// std::fs::File::create("crcs.json")
//     .unwrap()
//     .write_all(
//         serde_json::to_string_pretty(&json!(*crcs))
//             .unwrap()
//             .as_bytes(),
//     )
//     .unwrap();
// fn dir_entry(
//     file: DirEntry,
//     crcs: &Arc<Mutex<HashMap<u32, String>>>,
//     uuids: &Arc<Mutex<HashMap<String, String>>>,
//     re: &Regex,
// ) {
//     let mut f = std::fs::File::open(file.path()).unwrap();
//     let mut buf = [0; 13];
//     if let Err(_) = f.read_exact(&mut buf) {
//         return;
//     };

//     if &buf != b"<ObjectStream" {
//         return;
//     };

//     println!("Parsing {}", file.file_name().to_string_lossy());

//     let mut contents = vec![];
//     contents.extend_from_slice(&buf);

//     f.read_to_end(&mut contents).unwrap();
//     contents.lines().skip(1).for_each(|line| {
//         let ln = line.unwrap();
//         let fields = ln
//             .trim()
//             .trim_start_matches("<Class ")
//             .trim_end_matches(">")
//             .trim_end_matches("/");

//         let mut name = String::new();
//         for caps in re.captures_iter(fields) {
//             let key = caps.get(1).map_or("", |m| m.as_str());
//             let value = caps.get(2).map_or("", |m| m.as_str());

//             if key == "name" {
//                 name = value.to_owned();
//                 let mut hasher = Hasher::new();
//                 hasher.update(value.to_lowercase().as_bytes());
//                 let lowercase_crc = hasher.finalize();

//                 crcs.lock().unwrap().insert(lowercase_crc, value.to_owned());
//             } else if key == "field" {
//                 let mut hasher = Hasher::new();
//                 hasher.update(value.to_lowercase().as_bytes());
//                 let lowercase_crc = hasher.finalize();

//                 crcs.lock().unwrap().insert(lowercase_crc, value.to_owned());
//             }
//             if key == "type" {
//                 if !&name.is_empty() {
//                     uuids
//                         .lock()
//                         .unwrap()
//                         .insert(value.to_owned(), name.to_owned());
//                 }
//             }
//         }
//     });
// }
