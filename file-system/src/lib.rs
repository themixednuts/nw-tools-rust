use pak::{self, PakFile};
use std::{
    io::{self, Cursor},
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
};
use tokio::{fs::File, sync::oneshot};
use walkdir::WalkDir;
use zip::ZipArchive;

#[derive(Debug, Default)]
struct AssetCache {
    inner: Vec<AssetCacheEntry>,
}

#[derive(Debug, Default)]
struct AssetCacheEntry {
    path: PathBuf,
    inner: Vec<PathBuf>,
}

#[derive(Debug)]
pub struct FileSystem {
    assets: PathBuf,
    cache: AssetCache,
}

impl FileSystem {
    pub fn new(assets: &str) -> io::Result<Self> {
        let path = Path::new(assets);
        if !path.is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid directory",
            ));
        }
        Ok(Self {
            assets: path.to_path_buf(),
            cache: AssetCache::default(),
        })
    }

    pub async fn read(&mut self, entry: &str) -> io::Result<Cursor<Vec<u8>>> {
        let entry_path = self.cache.inner.iter().find_map(|cache| {
            cache
                .inner
                .iter()
                .find(|path| path.to_string_lossy() == *entry)
                .map(|_| cache.path.clone())
        });

        let entry = Arc::new(entry.to_string());
        match entry_path {
            Some(path) => {
                let file = File::open(path).await?.into_std().await;
                let (send, recv) = oneshot::channel();
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
                let paks: Vec<_> = WalkDir::new(&mut self.assets)
                    .into_iter()
                    .filter_map(|e| e.ok())
                    .filter(|path| {
                        path.file_type().is_file()
                            && path.path().extension().and_then(|ext| ext.to_str()) == Some("pak")
                            && !self
                                .cache
                                .inner
                                .iter()
                                .any(|cache_entry| cache_entry.path == path.path())
                    })
                    .collect();

                for pak in paks {
                    // dbg!(&pak);
                    let file = File::open(pak.path()).await?.into_std().await;
                    let entry = entry.clone();

                    let (send, recv) = oneshot::channel();
                    rayon::spawn(move || {
                        let archive = Arc::new(RwLock::new(ZipArchive::new(file).unwrap()));
                        let mut found = false;
                        let mut cache_entry = AssetCacheEntry::default();

                        cache_entry.path = pak.path().to_path_buf();
                        let cloned = archive.read().unwrap();
                        let names: Vec<_> = cloned.file_names().into_iter().collect();
                        for name in names {
                            let name_path = PathBuf::from(name);
                            cache_entry.inner.push(name_path);

                            if name == *entry {
                                found = true
                            }
                        }
                        drop(cloned);
                        let found = match found {
                            true => {
                                let cloned = archive.clone();
                                let pak = PakFile::new(cloned)
                                    .entry(&entry)
                                    .expect("something went wrong with getting the entry")
                                    .decompress()
                                    .expect("something went wrong with decompressing");
                                Some(pak)
                            }
                            false => None,
                        };
                        let _ = send.send((found, cache_entry));
                    });

                    let (found, cache_entry) = recv.await.expect("Didnt get the one shot?");
                    self.cache.inner.push(cache_entry);

                    if let Some(found) = found {
                        return Ok(found);
                    }
                }
                Err(io::Error::new(
                    io::ErrorKind::Other,
                    "Entry not found in the paks",
                ))
            }
        }
    }

    pub fn read_sync(&mut self, entry: &str) -> io::Result<Cursor<Vec<u8>>> {
        let entry_path = self.cache.inner.iter().find_map(|cache| {
            cache
                .inner
                .iter()
                .find(|path| path.to_string_lossy() == *entry)
                .map(|_| cache.path.clone())
        });

        match entry_path {
            Some(path) => {
                let file = std::fs::File::open(path)?;
                let archive = Arc::new(RwLock::new(ZipArchive::new(file).unwrap()));
                let pak = PakFile::new(archive).entry(entry).unwrap().decompress()?;
                Ok(pak)
            }
            None => {
                let paks: Vec<_> = WalkDir::new(&mut self.assets)
                    .into_iter()
                    .filter_map(|e| e.ok())
                    .filter(|path| {
                        path.file_type().is_file()
                            && path.path().extension().and_then(|ext| ext.to_str()) == Some("pak")
                            && !self
                                .cache
                                .inner
                                .iter()
                                .any(|cache_entry| cache_entry.path == path.path())
                    })
                    .collect();

                for pak in paks {
                    let file = std::fs::File::open(pak.path())?;

                    let archive = Arc::new(RwLock::new(ZipArchive::new(file).unwrap()));
                    let mut found = false;
                    let mut cache_entry = AssetCacheEntry::default();

                    cache_entry.path = pak.path().to_path_buf();
                    let cloned = archive.read().unwrap();
                    let names: Vec<_> = cloned.file_names().into_iter().collect();
                    for name in names {
                        let name_path = PathBuf::from(name);
                        cache_entry.inner.push(name_path);

                        if name == entry {
                            found = true
                        }
                    }
                    drop(cloned);
                    let found = match found {
                        true => {
                            let pak = PakFile::new(archive)
                                .entry(&entry)
                                .expect("something went wrong with getting the entry")
                                .decompress()
                                .expect("something went wrong with decompressing");
                            Some(pak)
                        }
                        false => None,
                    };

                    self.cache.inner.push(cache_entry);

                    if let Some(found) = found {
                        return Ok(found);
                    }
                }
                Err(io::Error::new(
                    io::ErrorKind::Other,
                    "Entry not found in the paks",
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::Seek;

    use super::*;
    use pak::azcs;

    #[tokio::test]
    async fn it_works() -> io::Result<()> {
        let mut fs = FileSystem::new("E:/Games/Steam/steamapps/common/New World/assets")?;

        let mut reader = fs
            .read("sharedassets/coatlicue/templateworld/regions/r_+00_+00/region.tractmap.tif")
            .await?;
        assert!(azcs::parser(&mut reader).is_err());

        let mut reader = fs
            .read("sharedassets/springboardentitites/datatables/javelindata_affixstats.datasheet")
            .await?;

        assert!(datasheet::parse_datasheet(&mut reader).is_ok());

        let mut reader = fs
            .read("sharedassets/genericassets/playerbaseattributes.pbadb")
            .await?;

        assert!(azcs::parser(&mut reader).is_ok());

        let mut reader = fs
            .read("sharedassets/genericassets/rangedattackdatabase.radb")
            .await?;
        assert!(azcs::parser(&mut reader).is_ok());

        let mut reader = fs
            .read("sharedassets/springboardentitites/datatables/javelindata_achievements.datasheet")
            .await?;
        datasheet::parse_datasheet(&mut reader)?;
        reader.rewind()?;
        datasheet::parse_datasheet_test(&mut reader)?;

        assert!(azcs::parser(&mut reader).is_err());

        //checks to make sure none files returns errors
        let reader = fs
            .read("sharedassets/coatlicue/templateworld/regions/r_+00_+00/region.tractmap")
            .await;
        assert!(reader.is_err());

        Ok(())
    }
}
