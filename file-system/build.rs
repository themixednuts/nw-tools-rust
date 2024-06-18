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
