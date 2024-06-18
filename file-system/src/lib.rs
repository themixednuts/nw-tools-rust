use crc32fast::Hasher;
use nucleo_matcher::{Config, Matcher, Utf32Str, Utf32String};
use pak::{self, PakFile};
use pelite::{
    pe::{Pe, PeFile},
    FileMap,
};
use rayon::iter::{ParallelBridge, ParallelIterator};
use std::{
    collections::HashMap,
    io::{self, Cursor},
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
};
use tokio::{fs::File, sync::oneshot};
use walkdir::WalkDir;
use zip::ZipArchive;

// #[derive(Debug, Default)]
// struct AssetCache {
//     inner: Vec<AssetCacheEntry>,
// }

// #[derive(Debug, Default)]
// struct AssetCacheEntry {
//     path: PathBuf,
//     inner: Vec<PathBuf>,
// }

#[derive(Debug, Default)]
pub struct FileSystem {
    dir: PathBuf,
    // cache: AssetCache,
    path_to_pak: HashMap<String, PathBuf>,
    uuids: HashMap<String, String>,
    crcs: HashMap<String, String>,
}

impl FileSystem {
    pub async fn new<P: AsRef<Path>>(dir: P) -> tokio::io::Result<Self> {
        let path = dir.as_ref();
        if !path.is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid directory",
            ));
        }

        let (crcs, uuids) = strings(&path).await?;
        let path_to_pak = create_pak_map(&path);

        Ok(Self {
            dir: path.into(),
            // cache: AssetCache::default(),
            path_to_pak,
            crcs,
            uuids,
        })
    }

    pub async fn read(&mut self, entry: &str) -> tokio::io::Result<Cursor<Vec<u8>>> {
        match self.path_to_pak.get(entry) {
            Some(path) => {
                let file = File::open(path).await?.into_std().await;
                let (send, recv) = oneshot::channel();
                let entry = entry.to_owned();
                rayon::spawn(move || {
                    let archive = Arc::new(RwLock::new(ZipArchive::new(file).unwrap()));
                    let pak = PakFile::new(archive)
                        .entry(&entry)
                        .unwrap()
                        .decompress()
                        .unwrap();
                    let _ = send.send(pak);
                });
                Ok(recv.await.expect(""))
            }
            None => {
                let mut matcher = Matcher::new(Config::DEFAULT.match_paths());
                let paths: Vec<Utf32String> = self
                    .path_to_pak
                    .keys()
                    .into_iter()
                    .filter_map(|path| Some(Utf32String::from(path.as_str())))
                    .collect();
                let mut scores = Vec::with_capacity(paths.len());
                for haystack in &paths {
                    scores.push(matcher.fuzzy_match(
                        haystack.slice(..),
                        Utf32Str::Ascii(entry.to_owned().as_bytes()),
                    ));
                }
                scores.sort_unstable();

                dbg!(&scores[0]);

                Err(io::Error::new(
                    io::ErrorKind::Other,
                    "Entry not found in the paks",
                ))
            }
        }
    }
}

async fn strings<P: AsRef<Path>>(
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
            .map(|(str, _)| {
                (
                    calculate_crc32(&str.to_lowercase()).to_string(),
                    str.to_owned(),
                )
            })
            .collect(),
        uuids,
    ))
}

fn create_pak_map<P: AsRef<Path>>(dir: &P) -> HashMap<String, PathBuf> {
    WalkDir::new(dir.as_ref().join("assets"))
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|path| {
            path.file_type().is_file()
                && path.path().extension().and_then(|ext| ext.to_str()) == Some("pak")
        })
        .par_bridge()
        .map(|dir| {
            let file = std::fs::File::open(dir.path()).unwrap();
            let archive = ZipArchive::new(file).unwrap();
            archive
                .file_names()
                .map(|name| (name.to_owned(), dir.path().to_path_buf()))
                .collect::<Vec<(String, PathBuf)>>()
        })
        .flatten()
        .collect()
}

fn calculate_crc32(string: &str) -> u32 {
    let mut hasher = Hasher::new();
    hasher.update(string.as_bytes());
    hasher.finalize()
}

#[cfg(test)]
mod tests {

    use super::*;
    use object_stream as objectstream;

    #[tokio::test]
    async fn async_read() -> io::Result<()> {
        let mut fs = FileSystem::new("E:/Games/Steam/steamapps/common/New World").await?;

        let mut reader = fs
            .read("sharedassets/coatlicue/templateworld/regions/r_+00_+00/region.tractmap.tif")
            .await?;
        assert!(objectstream::parser(&mut reader).is_err());

        //     let mut reader = fs
        //         .read("sharedassets/springboardentitites/datatables/javelindata_affixstats.datasheet")
        //         .await?;

        //     let datasheet = datasheet::Datasheet::from(&mut reader);
        //     dbg!(&datasheet.name, &datasheet._type);
        //     File::create(".test.yml")
        //         .await
        //         .unwrap()
        //         .write_all_buf(&mut datasheet.to_yaml().as_bytes())
        //         .await
        //         .unwrap();
        //     File::create(".test.csv")
        //         .await
        //         .unwrap()
        //         .write_all_buf(&mut datasheet.to_csv().as_bytes())
        //         .await
        //         .unwrap();
        //     File::create(".test.json")
        //         .await
        //         .unwrap()
        //         .write_all_buf(&mut datasheet.to_json().as_bytes())
        //         .await
        //         .unwrap();

        //     let mut reader = fs
        //         .read("sharedassets/genericassets/playerbaseattributes.pbadb")
        //         .await?;

        //     let stream = objectstream::parser(&mut reader);
        //     assert!(stream.is_ok());
        //     let stream = stream.unwrap();
        //     // dbg!(stream);

        //     let mut reader = fs
        //         .read("sharedassets/genericassets/rangedattackdatabase.radb")
        //         .await?;
        //     assert!(objectstream::parser(&mut reader).is_ok());

        //     let mut reader = fs
        //         .read("sharedassets/springboardentitites/datatables/javelindata_achievements.datasheet")
        //         .await?;
        //     let _ = datasheet::Datasheet::from(&mut reader);

        //     assert!(objectstream::parser(&mut reader).is_err());

        //     //checks to make sure none files returns errors
        //     let reader = fs
        //         .read("sharedassets/coatlicue/templateworld/regions/r_+00_+00/region.tractmap")
        //         .await;
        //     assert!(reader.is_err());

        Ok(())
    }
}
