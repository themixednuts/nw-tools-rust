use cli::commands::Commands;
use cli::common::{
    datasheet::{DatasheetFormat, DatasheetOutputMode},
    objectstream::ObjectStreamFormat,
};
use cli::ARGS;
use core::panic;
use decompressor::{Metadata, ZipFileExt};
use localization::Localization;
use memmap2::Mmap;
use regex::Regex;
use simd_json::prelude::ArrayTrait;
use std::collections::HashSet;
use std::fmt::Debug;
use std::sync::RwLock;
// use memmap2::Mmap;
use pelite::pe::{Pe, PeFile};
use pelite::FileMap;
use rayon::{prelude::*, ThreadPoolBuilder};
use std::io::{self, BufReader, Cursor, Write};
use std::sync::{atomic::Ordering, Mutex, OnceLock};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
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
    cwd: &'static PathBuf,
    out_dir: &'static PathBuf,
    path_to_pak: HashMap<PathBuf, (PathBuf, String)>,
    pub hashes: LumberyardSource,
    cancel: CancellationToken,
}

impl FileSystem {
    pub async fn init(
        cwd: &'static PathBuf,
        out_dir: &'static PathBuf,
        token: CancellationToken,
    ) -> &'static FileSystem {
        let handle = Handle::current();

        tokio::task::spawn_blocking(move || {
            FILESYSTEM.get_or_init(|| {
                if !cwd.is_dir() {
                    panic!("Not a correct directory");
                }
                let hashes = handle.block_on(async { parse_strings(&cwd).await.unwrap() });
                let path_to_pak = map(&cwd);
                FileSystem {
                    cwd,
                    out_dir,
                    path_to_pak,
                    hashes,
                    cancel: token,
                }
            })
        })
        .await
        .unwrap()
    }
    pub async fn load_localization(&'static self, locale: &str) -> HashMap<String, Option<String>> {
        let locale_path = PathBuf::from(format!("localization/{}", locale));
        let files = self
            .path_to_pak
            .iter()
            .filter(|(_, (_, name))| name.starts_with(locale_path.to_str().unwrap()))
            .map(|(_, v)| v)
            .collect::<Vec<_>>();

        files
            .iter()
            .map(|(path, _)| path)
            .collect::<HashSet<_>>()
            .par_iter()
            .map(|path| {
                let file = std::fs::File::open(path).unwrap();
                let mut archive = ZipArchive::new(file).unwrap();

                files
                    .iter()
                    .map(|(_path, name)| {
                        let Some(idx) = archive.index_for_name(name) else {
                            return None;
                        };

                        let mut entry = archive.by_index_raw(idx).unwrap();
                        let mut buf = Vec::with_capacity(entry.size() as usize);
                        entry.decompress(&mut buf).unwrap();

                        let locale =
                            match std::panic::catch_unwind(|| Localization::from(Cursor::new(buf)))
                            {
                                Ok(v) => v,
                                Err(_) => panic!("File Name: {}\n", name),
                            };

                        Some(HashMap::from(locale))
                    })
                    .filter_map(|v| v)
                    .flatten()
                    .collect::<HashMap<_, _>>()
            })
            .flatten()
            .collect::<HashMap<_, _>>()
    }

    pub fn files(
        &'static self,
        regex: Option<&Regex>,
    ) -> HashMap<&'static PathBuf, &'static (PathBuf, String)> {
        self.path_to_pak
            .iter()
            .filter(|(name, _)| {
                if let Some(filter) = regex {
                    filter.is_match(name.to_str().unwrap())
                } else {
                    true
                }
            })
            .collect()
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

    pub async fn all<F>(
        &'static self,
        map: HashMap<&'static PathBuf, &'static (PathBuf, String)>,
        state: Arc<RwLock<State>>,
        cb: F,
    ) -> tokio::io::Result<()>
    where
        F: Fn(Arc<&PathBuf>, &PathBuf, usize, usize, u64) -> io::Result<()>
            + Send
            + Sync
            + Clone
            + 'static,
    {
        let mut paks: HashMap<&PathBuf, Vec<(&PathBuf, &str)>> = HashMap::new();
        map.iter().for_each(|(entry, path)| {
            paks.entry(&path.0).or_default().push((&entry, &path.1));
        });

        let mut paks: Vec<(&PathBuf, Vec<(&PathBuf, &str)>)> = paks.into_iter().collect();
        paks.par_sort_unstable_by(|(s, _), (s2, _)| {
            natord::compare(
                s.file_stem().expect("msg").to_str().expect("msg"),
                s2.file_stem().expect("msg").to_str().expect("msg"),
            )
        });

        let cb = Arc::new(cb);
        let out_dir = Arc::new(self.out_dir.to_owned());

        if let Err(e) = tokio::task::spawn_blocking(move || {
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
                        let cb = cb.clone();
                        let archive = archive.clone();
                        let pak_path = pak_path.clone();
                        let state = state.clone();
                        // let mmap = mmap.clone();

                        p.spawn(move |_| {
                            if self.cancel.is_cancelled() {
                                return;
                            }

                            let state = state.read().unwrap();

                            let c = state.active.fetch_add(1, Ordering::Relaxed) + 1;
                            state.max.fetch_max(c, Ordering::Relaxed);

                            let Ok(mut archive) = archive.lock() else {
                                self.cancel.cancel();
                                return;
                            };
                            let index = archive.index_for_path(name).unwrap();
                            let mut zip = archive.by_index_raw(index).unwrap();

                            let path = out_dir.join(entry.to_path_buf());

                            let mut buf = Vec::with_capacity(zip.size() as usize);
                            let (bytes, file_type, metadata) = match zip.decompress(&mut buf) {
                                Ok(res) => res,
                                Err(_) => {
                                    self.cancel.cancel();
                                    return;
                                }
                            };
                            let path = handle_extension(file_type, path, metadata);
                            let Some(parent) = path.parent() else { return };
                            std::fs::create_dir_all(parent).expect("failed to create directory");
                            let mut file = std::fs::File::create(&path).unwrap();

                            std::io::copy(&mut Cursor::new(buf), &mut file).unwrap();

                            state.active.fetch_sub(1, Ordering::Relaxed);
                            state.max.load(Ordering::Relaxed);
                            state.size.store(bytes as usize, Ordering::Relaxed);

                            if let Err(_) = cb(
                                pak_path,
                                entry,
                                len,
                                idx.fetch_add(1, Ordering::Relaxed) + 1,
                                bytes,
                            ) {
                                self.cancel.cancel();
                                return;
                            }
                        });
                    }
                });
            });
        })
        .await
        {
            self.cancel.cancel();
            return Err(tokio::io::Error::other(e));
        };

        Ok(())
    }
}

pub struct State {
    pub active: Arc<AtomicUsize>,
    pub max: Arc<AtomicUsize>,
    pub size: Arc<AtomicUsize>,
}

fn handle_extension(file_type: FileType, mut path: PathBuf, meta: Option<Metadata>) -> PathBuf {
    let ext = path.extension().unwrap().to_os_string();
    match file_type {
        FileType::ObjectStream(fmt) => match fmt {
            ObjectStreamFormat::XML => {
                if ext != "xml" {
                    // std::fs::rename(&path, path.with_extension("xml")).unwrap();
                    path.set_extension("xml");
                }
            }
            ObjectStreamFormat::JSON | ObjectStreamFormat::PRETTY => {
                if ext != "json" {
                    // std::fs::rename(&path, path.with_extension("json")).unwrap();
                    path.set_extension("json");
                }
            }
            _ => {}
        },
        FileType::Datasheet(fmt) => {
            match &ARGS.command {
                Commands::Extract(extract) => {
                    if extract.datasheet.datasheet_filenames == DatasheetOutputMode::TYPENAME {
                        if let Some(meta) = &meta {
                            match meta {
                                Metadata::Datasheet(datasheet) => {
                                    let datatable_root = path
                                        .ancestors()
                                        .find(|p| p.ends_with("datatables"))
                                        .unwrap()
                                        .to_path_buf();

                                    path = datatable_root;
                                    path = path
                                        .join(format!("{}/{}", datasheet._type, datasheet.name));
                                    path.set_extension(&ext);
                                }
                            }
                        }
                    }
                }
            };
            match fmt {
                DatasheetFormat::BYTES => {}
                DatasheetFormat::XML => {
                    if ext != "xml" {
                        // std::fs::rename(&path, path.with_extension("xml")).unwrap();
                        path.set_extension("xml");
                    }
                }
                DatasheetFormat::MINI | DatasheetFormat::PRETTY => {
                    if ext != "json" {
                        // std::fs::rename(&path, path.with_extension("json")).unwrap();
                        path.set_extension("json");
                    }
                    if let Some(meta) = &meta {
                        match meta {
                            Metadata::Datasheet(datasheet) => {
                                let Some(parent) = path.parent() else {
                                    panic!("hmm")
                                };
                                std::fs::create_dir_all(parent)
                                    .expect("failed to create directory");

                                // let mut schema =
                                //     schemars::schema_for_value!(datasheet.json_value());
                                // schema.schema.metadata().title = Some(datasheet._type.to_owned());
                                // schema.schema.metadata().id = Some(datasheet.name.to_owned());

                                let stem = path.file_stem().unwrap();
                                let mut schema_path = path.with_file_name(stem);
                                schema_path.set_extension("meta.json");
                                let mut file = std::fs::File::create(schema_path).unwrap();
                                file.write_all(
                                    &simd_json::to_vec_pretty(&datasheet.meta()).unwrap(),
                                )
                                .unwrap();
                                // datasheet.to_json_simd(pretty)
                            }
                        }
                    }
                }
                DatasheetFormat::CSV => {
                    if ext != "csv" {
                        // std::fs::rename(&path, path.with_extension("csv")).unwrap();
                        path.set_extension("csv");
                    }
                }
                DatasheetFormat::YAML => {
                    if ext != "yaml" {
                        // std::fs::rename(&path, path.with_extension("yaml")).unwrap();
                        path.set_extension("yaml");
                    }
                }
                DatasheetFormat::SQL => {
                    if ext != "sql" {
                        // std::fs::rename(&path, path.with_extension("yaml")).unwrap();
                        path.set_extension("sql");
                    }
                }
            }
        }
        _ => {}
    };
    path
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

fn map<P: AsRef<Path>>(path: &P) -> HashMap<PathBuf, (PathBuf, String)> {
    let assets_dir = path.as_ref().join("assets").to_path_buf();
    WalkDir::new(assets_dir.to_path_buf())
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|path| {
            path.file_type().is_file()
                && path.path().extension().and_then(|ext| ext.to_str()) == Some("pak")
        })
        .par_bridge()
        .map(|dir| {
            let file = std::fs::File::open(dir.path()).unwrap();
            let mmap = unsafe { Mmap::map(&file).expect("couldn't map file") };
            drop(file);
            let mmap = Cursor::new(mmap);
            let archive = ZipArchive::new(mmap).unwrap();

            // dbg!(&dir);
            // let file_names: Vec<String> = archive.file_names().map(|n| n.to_owned()).collect();
            let file_names = archive
                .file_names()
                // .iter()
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
                    // let idx = archive.index_for_name(name).unwrap();
                    // let size = archive.by_index_raw(idx).unwrap().size() as usize;

                    (full_name, (dir.path().to_path_buf(), name.to_string()))
                })
                .collect::<Vec<(PathBuf, (PathBuf, String))>>();

            file_names
        })
        .flatten()
        .collect()
}

#[derive(Default, Debug)]
pub enum FileType {
    Luac,
    ObjectStream(&'static ObjectStreamFormat),
    Datasheet(&'static DatasheetFormat),
    #[default]
    Other,
}

pub fn file_type(sig: &[u8; 5]) -> io::Result<FileType> {
    let _type = match sig {
        [0x04, 0x00, 0x1B, 0x4C, 0x75] => FileType::Luac,
        [0x00, 0x00, 0x00, 0x00, 0x03] => match &ARGS.command {
            Commands::Extract(extract) => {
                FileType::ObjectStream(&extract.objectstream.objectstream)
            }
        },
        [0x11, 0x00, 0x00, 0x00, _] => match &ARGS.command {
            Commands::Extract(extract) => FileType::Datasheet(&extract.datasheet.datasheet),
        },
        _ => FileType::default(),
    };

    Ok(_type)
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
        map(&root);
    }
}
