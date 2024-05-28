use pak;
use rayon::prelude::*;
use std::{
    collections::HashMap,
    fs::File,
    io::{self, Read},
    path::PathBuf,
    rc::Rc,
};
use walkdir::WalkDir;
use zip::ZipArchive;

pub struct FileSystem {
    index: HashMap<PathBuf, PathBuf>,
}

impl FileSystem {
    pub fn new(dir: &str) -> Self {
        Self {
            index: Self::create_index(dir),
        }
    }
    fn create_index(dir: &str) -> HashMap<PathBuf, PathBuf> {
        let entries: Vec<_> = WalkDir::new(dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|entry| entry.file_type().is_file())
            .collect();

        let map: HashMap<PathBuf, PathBuf> = entries
            .par_iter()
            .filter_map(|entry| {
                let file = File::open(entry.path()).ok()?;
                let mut local_map = HashMap::new();
                if let Ok(archive) = ZipArchive::new(file) {
                    for name in archive.file_names() {
                        local_map.insert(PathBuf::from(name), entry.path().to_path_buf());
                    }
                }
                Some(local_map)
            })
            .reduce(HashMap::new, |mut acc, map| {
                acc.extend(map);
                acc
            });
        map
    }
    pub fn read(&self, entry: &str) -> Result<Box<dyn Read>, io::Error> {
        let in_pak = self.index.get(&PathBuf::from(entry));
        match in_pak {
            Some(file) => {
                let mut zip_file = pak::open(&file.display().to_string()).map_err(|e| {
                    io::Error::new(
                        io::ErrorKind::Other,
                        format!("Failed to open pak file: {}", e),
                    )
                })?;
                let archive = pak::parse(&mut zip_file, entry).map_err(|e| {
                    io::Error::new(
                        io::ErrorKind::Other,
                        format!("Failed to parse archive: {}", e),
                    )
                })?;
                pak::decompress(Rc::new(archive))
            }
            None => Err(io::Error::new(
                io::ErrorKind::Other,
                "Entry not found in the index",
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let fs = FileSystem::new("E:/Games/Steam/steamapps/common/New World/assets");
        let reader = fs.read("sharedassets/genericassets/playerbaseattributes.pbadb");

        match reader {
            Ok(reader) => {
                let mut bytes = reader.bytes();
                while let Some(Ok(byte)) = bytes.next() {
                    println!("Byte: {}", byte);
                }
                assert!(true)
            }
            Err(e) => {
                dbg!(e);
                assert!(false)
            }
        };
    }
}
