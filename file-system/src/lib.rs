use decompressor::{decompress_zip, ZipFileExt};
use futures::{Future, StreamExt};
use nucleo_matcher::{Config, Matcher, Utf32Str, Utf32String};
use pelite::{
    pe::{Pe, PeFile},
    FileMap,
};
use rayon::prelude::*;
use std::sync::OnceLock;
use std::{
    collections::HashMap,
    io::{self, BufReader, Cursor},
    path::{Path, PathBuf},
    sync::{atomic::AtomicUsize, Arc, RwLock},
};
use tokio::{
    fs::File,
    io::{AsyncRead, AsyncSeek},
    runtime::Handle,
};
use tokio_stream::wrappers::ReceiverStream;
use utils::crc32;
use walkdir::WalkDir;
use zip::read::ZipArchive;

pub mod azcs;
pub mod decompressor;

static INSTANCE: OnceLock<FileSystem> = OnceLock::new();

#[derive(Debug, Default)]
pub struct FileSystem {
    root_dir: PathBuf,
    out_dir: PathBuf,
    path_to_pak: HashMap<PathBuf, (PathBuf, String)>,
    uuids: HashMap<String, String>,
    crcs: HashMap<String, String>,
    len: usize,
    size: u128,
}

impl FileSystem {
    pub async fn init<P>(root: P, out_dir: P) -> tokio::io::Result<&'static FileSystem>
    where
        P: AsRef<Path>,
    {
        let path = root.as_ref();
        if !path.is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid directory",
            ));
        }
        match INSTANCE.get() {
            Some(fs) => Ok(fs),
            None => {
                let (crcs, uuids) = parse_strings(&path).await?;
                let (path_to_pak, len, size) = create_pak_map(&path);
                let fs = FileSystem {
                    root_dir: path.into(),
                    out_dir: out_dir.as_ref().to_path_buf(),
                    path_to_pak,
                    crcs,
                    uuids,
                    len,
                    size,
                };
                match INSTANCE.set(fs) {
                    Ok(_) => {
                        let Some(fs) = INSTANCE.get() else {
                            return Err(io::Error::new(
                                io::ErrorKind::InvalidInput,
                                "Invalid directory",
                            ));
                        };
                        Ok(fs)
                    }
                    Err(_) => Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "Invalid directory",
                    )),
                }
            }
        }
    }

    // pub async fn get(
    //     &self,
    //     entry: &str,
    // ) -> tokio::io::Result<impl AsyncRead + AsyncSeek + Unpin + Sync + Send> {
    //     match self.path_to_pak.get(entry) {
    //         Some(path) => {
    //             let file = File::open(path).await?.into_std().await;
    //             let buf_read = BufReader::new(file);
    //             let mut archive = ZipArchive::new(buf_read).unwrap();
    //             let index = archive.index_for_path(entry).unwrap();
    //             let entry = archive.by_index_raw(index).unwrap();
    //             let mut buf = vec![];
    //             entry.decompress(&mut buf).unwrap();
    //             let reader = tokio::io::BufReader::new(Cursor::new(buf));
    //             Ok(reader)
    //         }
    //         None => {
    //             let mut matcher = Matcher::new(Config::DEFAULT.match_paths());
    //             let paths: Vec<Utf32String> = self
    //                 .path_to_pak
    //                 .keys()
    //                 .into_iter()
    //                 .filter_map(|path| Some(Utf32String::from(path.as_str())))
    //                 .collect();
    //             let mut scores = Vec::with_capacity(paths.len());
    //             for haystack in &paths {
    //                 scores.push(matcher.fuzzy_match(
    //                     haystack.slice(..),
    //                     Utf32Str::Ascii(entry.to_owned().as_bytes()),
    //                 ));
    //             }
    //             scores.sort_unstable();

    //             dbg!(&scores[0]);

    //             Err(io::Error::new(
    //                 io::ErrorKind::Other,
    //                 "Entry not found in the paks",
    //             ))
    //         }
    //     }
    // }

    pub async fn process_all<F>(&self, cb: F) -> tokio::io::Result<()>
    where
        F: Fn(Arc<PathBuf>, Arc<PathBuf>, usize, usize, usize, usize, u64) -> io::Result<()>
            + Send
            + Sync
            + Clone
            + 'static,
    {
        let (tx, rx) = tokio::sync::mpsc::channel(1000);
        let mut paks: HashMap<PathBuf, Vec<(PathBuf, String)>> = HashMap::new();
        self.path_to_pak.keys().for_each(|entry| {
            let path = self.path_to_pak.get(entry).unwrap();
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
        let tx = Arc::new(tx);
        let out_dir = Arc::new(self.out_dir.to_owned());

        let handle = Handle::current();
        for (pak_path, entries) in paks.into_iter() {
            let file = tokio::fs::File::open(&pak_path).await?;
            let file = BufReader::new(file.into_std().await);
            let archive = Arc::new(RwLock::new(ZipArchive::new(file).unwrap()));
            let len = entries.len();
            let pak_path = Arc::new(pak_path);
            let active_threads = active_threads.clone();
            let max_threads = max_threads.clone();
            let cb = cb.clone();

            entries.into_par_iter().for_each(|(entry, name)| {
                active_threads.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                let entry = Arc::new(entry);
                let pak_path = Arc::clone(&pak_path);
                let archive = Arc::clone(&archive);
                let mut write_archive = archive.write().unwrap();

                let index = write_archive.index_for_path(&*entry).unwrap();
                let mut zip = write_archive.by_index_raw(index).unwrap();

                let active_threads_clone = active_threads.clone();
                let max_threads_clone = max_threads.clone();
                let cb = cb.clone();

                let path = out_dir.join(entry.to_path_buf());
                std::fs::create_dir_all(&path.parent().unwrap()).unwrap();
                let mut file = std::fs::File::create(&path).unwrap();
                let bytes = decompress_zip(&mut zip, &mut file).unwrap();

                let join = handle.spawn_blocking(move || {
                    let active_threads = active_threads_clone.clone();
                    let max_threads = max_threads_clone.clone();
                    cb(
                        entry,
                        pak_path,
                        len,
                        active_threads.load(std::sync::atomic::Ordering::Relaxed),
                        max_threads.load(std::sync::atomic::Ordering::Relaxed),
                        index,
                        bytes,
                    )
                    .unwrap();
                });
                if let Ok(permit) = tx.try_reserve() {
                    permit.send(join);
                }

                let active_threads = active_threads.clone();
                let max_threads = max_threads.clone();
                let c = active_threads.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
                if c > max_threads.load(std::sync::atomic::Ordering::Relaxed) {
                    max_threads.store(c, std::sync::atomic::Ordering::Relaxed);
                }
            });
        }

        let mut stream = ReceiverStream::new(rx);
        while let Some(s) = stream.next().await {
            s.await.unwrap();
        }
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn size(&self) -> u128 {
        self.size
    }
}

async fn parse_strings<P: AsRef<Path>>(
    dir: &P,
) -> io::Result<(HashMap<String, String>, HashMap<String, String>)> {
    let path = dir.as_ref().join("Bin64/NewWorld.exe");

    let file_map = FileMap::open(&path)?;
    let pe = PeFile::from_bytes(&file_map).expect(&format!(
        "Couldn't create a PeFile from the file map for {}",
        path.display()
    ));

    let mut strings: Vec<(String, usize)> = Vec::new();
    let mut uuids = HashMap::new();

    for section in pe.section_headers() {
        if !section.Name.starts_with(b".rdata") {
            continue;
        }

        if let Ok(data) = pe.get_section_bytes(section) {
            let rva = section.VirtualAddress;
            let offset = pe.rva_to_file_offset(rva).unwrap();

            let mut string = String::new();
            let mut string_offset = offset;

            for (i, chunk) in data.chunks(4).enumerate() {
                if let Ok(str) = std::str::from_utf8(chunk) {
                    string.push_str(str);
                    if string.contains('\0') && string.is_ascii() {
                        string.retain(|c| !c.is_control());
                        if string.len() > 4 && string.chars().all(|c| c.is_ascii_graphic()) {
                            if let Ok(_) = uuid::Uuid::try_parse(&string) {
                                if let Some((last, offset)) = strings.last() {
                                    if uuid::Uuid::try_parse(last).is_err()
                                        && (offset + last.len() + 8 <= string_offset)
                                    {
                                        uuids.insert(string.clone(), last.clone());
                                    }
                                }
                            } else {
                                strings.push((string.clone(), string_offset));
                            }
                        }
                        string.clear();
                        string_offset = offset + (i + 1) * 4;
                    }
                } else {
                    string.clear();
                    string_offset = offset + (i + 1) * 4;
                };
            }
        }
    }

    Ok((
        strings
            .iter()
            .map(|(str, _)| (crc32(&str.to_lowercase()).to_string(), str.to_owned()))
            .collect(),
        uuids,
    ))
}

fn create_pak_map<P: AsRef<Path>>(path: &P) -> (HashMap<PathBuf, (PathBuf, String)>, usize, u128) {
    let size = Arc::new(0);
    let len = Arc::new(AtomicUsize::new(0));

    let assets_dir = Arc::new(path.as_ref().join("assets").to_path_buf());
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
                let mut size = size.clone();
                let file = std::fs::File::open(dir.path()).unwrap();
                let archive = ZipArchive::new(file).unwrap();

                if let Some(s) = archive.decompressed_size() {
                    let size = Arc::make_mut(&mut size);
                    *size += s;
                };

                // dbg!(&dir);
                let file_names = archive
                    .file_names()
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
                file_names
            })
            .flatten()
            .collect(),
        len.load(std::sync::atomic::Ordering::Relaxed),
        *size,
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
