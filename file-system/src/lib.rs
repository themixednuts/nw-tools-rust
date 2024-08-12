use cli::commands::Commands;
use cli::ARGS;
use core::panic;
use decompressor::ZipFileExt;
use memmap2::Mmap;
use std::fmt::Debug;
// use memmap2::Mmap;
use pelite::pe::{Pe, PeFile};
use pelite::FileMap;
use rayon::{prelude::*, ThreadPoolBuilder};
use std::io::{self, Cursor};
use std::sync::{atomic::Ordering, Mutex, OnceLock};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    str::FromStr,
    sync::{atomic::AtomicUsize, Arc},
};
use tokio::{
    fs::File,
    io::{AsyncRead, AsyncSeek},
    runtime::Handle,
};
use tokio_util::sync::CancellationToken;
use utils::{crc32, lumberyard::LumberyardSource};
use uuid::Uuid;
use walkdir::WalkDir;
use zip::read::ZipArchive;

pub mod azcs;
pub mod decompressor;
mod pak;

pub static FILESYSTEM: OnceLock<FileSystem> = OnceLock::new();

#[derive(Debug)]
pub struct FileSystem {
    root_dir: &'static PathBuf,
    out_dir: &'static PathBuf,
    path_to_pak: HashMap<PathBuf, (PathBuf, String)>,
    pub hashes: LumberyardSource,
    len: usize,
    size: u128,
    cancel: CancellationToken,
}

impl FileSystem {
    pub async fn init(token: CancellationToken) -> &'static FileSystem {
        let args = &ARGS;
        let handle = Handle::current();
        let root_dir = match &args.command {
            Commands::Extract(extract) => extract.input.as_ref().unwrap(),
        };
        let out_dir = match &args.command {
            Commands::Extract(extract) => extract.output.as_ref().unwrap(),
        };

        tokio::task::spawn_blocking(move || {
            FILESYSTEM.get_or_init(|| {
                if !root_dir.is_dir() {
                    panic!("Not a correct directory");
                }
                let hashes = handle.block_on(async { parse_strings(&root_dir).await.unwrap() });
                let (path_to_pak, len, size) = create_pak_map(&root_dir);
                FileSystem {
                    root_dir,
                    out_dir,
                    path_to_pak,
                    len,
                    size,
                    hashes,
                    cancel: token,
                }
            })
        })
        .await
        .unwrap()
    }

    pub async fn open<P>(
        &'static self,
        entry: P,
    ) -> tokio::io::Result<impl AsyncRead + AsyncSeek + Unpin + Sync + Send>
    where
        P: AsRef<Path>,
    {
        match self.path_to_pak.get(entry.as_ref()) {
            Some((path, _str)) => {
                let file = File::open(path).await?.into_std().await;
                let mut archive = ZipArchive::new(file).unwrap();
                let index = archive.index_for_path(entry).unwrap();
                let mut entry = archive.by_index_raw(index).unwrap();
                let mut buf = vec![];
                entry.decompress(&mut buf).unwrap();
                let reader = Cursor::new(buf);
                Ok(reader)
            }
            None => {
                // let mut matcher = Matcher::new(Config::DEFAULT.match_paths());
                // let paths: Vec<Utf32String> = self
                //     .path_to_pak
                //     .keys()
                //     .into_iter()
                //     .filter_map(|path| Some(Utf32String::from(path.as_str())))
                //     .collect();
                // let mut scores = Vec::with_capacity(paths.len());
                // for haystack in &paths {
                //     scores.push(matcher.fuzzy_match(
                //         haystack.slice(..),
                //         Utf32Str::Ascii(entry.to_owned().as_bytes()),
                //     ));
                // }
                // scores.sort_unstable();

                // dbg!(&scores[0]);

                Err(io::Error::new(
                    io::ErrorKind::Other,
                    "Entry not found in the paks",
                ))
            }
        }
    }

    pub async fn process_all<F>(&'static self, cb: F) -> tokio::io::Result<()>
    where
        F: Fn(Arc<PathBuf>, PathBuf, usize, usize, usize, usize, u64) -> io::Result<()>
            + Send
            + Sync
            + Clone
            + 'static,
    {
        let mut paks: HashMap<PathBuf, Vec<(PathBuf, String)>> = HashMap::new();
        self.path_to_pak.keys().for_each(|entry| {
            let path = self.path_to_pak.get(entry).unwrap();

            // dbg!(&path);
            paks.entry(path.0.clone())
                .or_default()
                .push((entry.clone(), path.1.to_owned()));
        });

        let mut paks: Vec<(PathBuf, Vec<(PathBuf, String)>)> = paks.into_iter().collect();
        paks.par_sort_unstable_by(|(s, _), (s2, _)| {
            natord::compare(
                s.file_stem().expect("msg").to_str().expect("msg"),
                s2.file_stem().expect("msg").to_str().expect("msg"),
            )
        });

        let cb = Arc::new(cb);
        let active_threads = Arc::new(AtomicUsize::new(0));
        let max_threads = Arc::new(AtomicUsize::new(0));
        let out_dir = Arc::new(self.out_dir.to_owned());

        tokio::task::spawn_blocking(move || {
            let pool = ThreadPoolBuilder::new().build().unwrap();
            pool.scope(|p| {
                paks.into_par_iter().for_each(|(pak_path, entries)| {
                    let pak_path = Arc::new(pak_path);
                    let len = entries.len();
                    let idx = Arc::new(AtomicUsize::new(0));
                    let file = std::fs::OpenOptions::new()
                        .read(true)
                        // .write(true)
                        .open(pak_path.as_ref())
                        .unwrap();
                    let mmap = unsafe {
                        Mmap::map(&file).expect("couldn't map file")
                        // .make_mut()
                        // .expect("couldnt make mut")
                    };

                    let archive = Arc::new(Mutex::new(ZipArchive::new(Cursor::new(mmap)).unwrap()));

                    for (entry, name) in entries {
                        if self.cancel.is_cancelled() {
                            return;
                        }
                        let out_dir = out_dir.clone();
                        let idx = idx.clone();
                        let active_threads = active_threads.clone();
                        let max_threads = max_threads.clone();
                        let cb = cb.clone();
                        let archive = archive.clone();
                        let pak_path = pak_path.clone();
                        // let mmap = mmap.clone();

                        p.spawn(move |_| {
                            if self.cancel.is_cancelled() {
                                return;
                            }
                            let c = active_threads.fetch_add(1, Ordering::Relaxed) + 1;
                            max_threads.fetch_max(c, Ordering::Relaxed);

                            let mut archive = archive.lock().unwrap();
                            let index = archive.index_for_path(name).unwrap();
                            let mut zip = archive.by_index_raw(index).unwrap();

                            let path = out_dir.join(entry.to_path_buf());
                            let Some(parent) = path.parent() else { return };

                            std::fs::create_dir_all(parent).expect("failed to create directory");
                            let mut file = std::fs::File::create(&path).unwrap();
                            let Ok(bytes) = zip.decompress(&mut file) else {
                                (0..20).for_each(|_| {
                                    dbg!(&path);
                                });
                                self.cancel.cancel();
                                return;
                            };

                            // drop(zip);
                            // drop(archive);

                            if let Err(e) = cb(
                                pak_path,
                                entry,
                                len,
                                active_threads.fetch_sub(1, Ordering::Relaxed),
                                max_threads.load(Ordering::Relaxed),
                                idx.fetch_add(1, Ordering::Relaxed) + 1,
                                bytes,
                            ) {
                                if e.to_string() != "task cancelled" {
                                    self.cancel.cancel();
                                }
                            }
                        });
                    }
                });
            });
        })
        .await
        .unwrap();

        Ok(())
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn size(&self) -> u128 {
        self.size
    }
}

async fn parse_strings<P: AsRef<Path>>(dir: &P) -> io::Result<LumberyardSource> {
    let uuids: HashMap<Uuid, String> =
        serde_json::from_str(include_str!("../../uuids.json")).unwrap();
    let crcs: HashMap<u32, String> = serde_json::from_str(include_str!("../../crcs.json")).unwrap();
    let mut ly: LumberyardSource = serde_json::from_str(include_str!("../../ly.json")).unwrap();
    ly.crcs.extend(crcs);
    ly.uuids.extend(uuids);

    let path = dir.as_ref().join("Bin64/NewWorld.exe");

    let file_map = FileMap::open(&path).unwrap();
    let pe = PeFile::from_bytes(&file_map).expect(&format!(
        "Couldn't create a PeFile from the file map for {}",
        path.display()
    ));

    let mut strings: Vec<(String, usize)> = Vec::new();

    for section in pe.section_headers() {
        if !section.Name.starts_with(b".rdata") {
            continue;
        }

        if let Ok(data) = pe.get_section_bytes(section) {
            let rva = section.VirtualAddress;
            let offset = pe.rva_to_file_offset(rva).unwrap();

            // let mut string = String::new();
            let mut string_offset = offset;

            let mut vec = vec![];

            for (i, &chunk) in data.iter().enumerate() {
                if chunk == b'\0' {
                    if !vec.is_empty() {
                        if let Ok(str) = std::str::from_utf8(&vec) {
                            // if str == "Max Instanced SlayerScript State Count" {
                            //     println!("{str}");
                            // }
                            let string = str.trim_end_matches('\0').to_string();
                            if string.len() > 4
                                && string
                                    .chars()
                                    .all(|c| c.is_ascii_graphic() || c.is_ascii_whitespace())
                            {
                                if let Ok(uuid) = Uuid::try_parse(&string) {
                                    if let Some((last, last_offset)) = strings.last() {
                                        if Uuid::try_parse(last).is_err()
                                            && (string_offset - last_offset - last.len() <= 8)
                                        {
                                            ly.uuids.entry(uuid).or_insert_with(|| last.clone());
                                        }
                                    }
                                } else {
                                    strings.push((string, string_offset));
                                }
                            }
                        }

                        vec.clear();
                    }
                    string_offset = offset + i + 1;
                } else {
                    vec.push(chunk);
                }
            }
        }
    }
    strings.iter().for_each(|(str, _)| {
        let crc = crc32(&str.to_lowercase());
        ly.crcs.entry(crc).or_insert_with(|| str.to_owned());
    });
    Ok(ly)
}

fn create_pak_map<P: AsRef<Path>>(path: &P) -> (HashMap<PathBuf, (PathBuf, String)>, usize, u128) {
    let size = Arc::new(Mutex::new(0u128));
    let len = Arc::new(AtomicUsize::new(0));

    let assets_dir = path.as_ref().join("assets").to_path_buf();
    (
        WalkDir::new(assets_dir.to_path_buf())
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|path| {
                path.file_type().is_file()
                    && path.path().extension().and_then(|ext| ext.to_str()) == Some("pak")
            })
            .par_bridge()
            .map(|dir| {
                let size = size.clone();

                let file = std::fs::File::open(dir.path()).unwrap();
                let mmap = unsafe { Mmap::map(&file).expect("couldn't map file") };
                drop(file);
                let mmap = Cursor::new(mmap);
                let mut archive = ZipArchive::new(mmap).unwrap();

                // dbg!(&dir);
                let file_names = archive
                    .file_names()
                    .filter(|name| {
                        if let Some(filter) = match &ARGS.command {
                            cli::commands::Commands::Extract(ext) => &ext.filter,
                        } {
                            filter.is_match(name)
                        } else {
                            true
                        }
                    })
                    .map(|name| {
                        let full_name = dir
                            .to_owned()
                            .into_path()
                            .strip_prefix(assets_dir.to_path_buf())
                            .unwrap()
                            .to_path_buf()
                            .parent()
                            .unwrap()
                            .join(name);

                        (full_name, (dir.path().to_path_buf(), name.to_owned()))
                    })
                    .collect::<Vec<(PathBuf, (PathBuf, String))>>();
                len.fetch_add(file_names.len(), std::sync::atomic::Ordering::Relaxed);

                match &ARGS.command {
                    Commands::Extract(ext) => {
                        if ext.filter.is_some() {
                            let mut _size = 0;
                            for (_, (_, name)) in &file_names {
                                let index = archive.index_for_name(&name).unwrap();
                                let file = archive.by_index_raw(index).unwrap();
                                _size += file.size();
                            }
                            let mut size = size.lock().unwrap();
                            *size += _size as u128;
                        } else {
                            let mut guard = size.lock().unwrap();
                            *guard += archive.decompressed_size().unwrap();
                        }
                    }
                }
                file_names
            })
            .flatten()
            .collect(),
        len.load(std::sync::atomic::Ordering::Relaxed),
        {
            let size_guard = size.lock().unwrap();
            *size_guard
        },
    )
}

#[cfg(test)]
mod tests {

    use super::*;

    // #[tokio::test(flavor = "multi_thread")]
    // async fn async_read() -> tokio::io::Result<()> {
    //     let mut fs = FileSystem::new("E:/Games/Steam/steamapps/common/New World").await?;

    //     let mut reader = fs
    //         .get("sharedassets/coatlicue/templateworld/regions/r_+00_+00/region.tractmap.tif")
    //         .await?;
    //     assert!(objectstream::parser(&mut reader).is_err());

    //     Ok(())
    // }

    #[test]
    fn pak_map() {
        let root = "C:/Program Files (x86)/Steam/steamapps/common/New World";
        create_pak_map(&root);
    }
}
