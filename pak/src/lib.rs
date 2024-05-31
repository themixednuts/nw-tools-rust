pub mod azcs;
mod buffers;
pub mod reader;

use azcs::is_azcs;
use oodle_safe;
use std::{
    fs::File,
    io::{self, Cursor, Read},
    path::PathBuf,
    sync::{Arc, RwLock},
};
use zip::{CompressionMethod, ZipArchive};

// var azcsSig = []byte{0x41, 0x5a, 0x43, 0x53};
// var luacSig = []byte{0x04, 0x00, 0x1b, 0x4c, 0x75, 0x61};

#[derive(Debug)]
pub struct PakFile {
    archive: Arc<RwLock<ZipArchive<File>>>,
}

impl PakFile {
    pub fn new(archive: Arc<RwLock<ZipArchive<File>>>) -> Self {
        Self { archive }
    }

    pub fn entry<'a>(&'a mut self, path: &str) -> Option<PakFileEntry> {
        let path = PathBuf::from(path);
        let archive_clone = self.archive.clone();
        let cloned = archive_clone.read().ok()?;
        let index = cloned.index_for_path(path)?;
        let pak = PakFileEntry {
            archive: self.archive.clone(),
            index,
        };
        Some(pak)
    }
}

pub struct PakFileEntry {
    archive: Arc<RwLock<ZipArchive<File>>>,
    index: usize,
}

impl PakFileEntry {
    pub fn decompress(&mut self) -> io::Result<Box<dyn Read + Sync + Unpin + Send>> {
        let archive = self.archive.clone();
        let mut archive = archive.write().unwrap();
        let mut entry = archive.by_index_raw(self.index)?;

        match entry.compression() {
            CompressionMethod::Stored => {
                let mut buf = vec![];
                entry.read_to_end(&mut buf)?;
                Ok(Box::new(Cursor::new(buf)))
            }
            CompressionMethod::Deflated | CompressionMethod::Deflate64 => {
                let mut sig = [0; 5];
                entry.read_exact(&mut sig)?;

                match is_azcs(&mut entry, &mut sig) {
                    Ok(header) => azcs::decompress(&mut entry, &header),
                    Err(_) => {
                        let mut buf = vec![];
                        buf.extend_from_slice(&sig);
                        entry.read_to_end(&mut buf)?;
                        Ok(Box::new(Cursor::new(buf)))
                    }
                }
            }
            CompressionMethod::Unsupported(15) => {
                let mut compressed = vec![];
                entry.read_to_end(&mut compressed)?;

                let decompressed_size = entry.size() as usize;
                let mut decompressed = vec![0u8; decompressed_size];

                let result = oodle_safe::decompress(
                    &compressed,
                    &mut decompressed,
                    None,
                    None,
                    None,
                    Some(oodle_safe::DecodeThreadPhase::All),
                );

                match result {
                    Ok(_) => Ok(Box::new(Cursor::new(decompressed))),
                    Err(_) => Err(io::Error::new(
                        io::ErrorKind::Other,
                        format!(
                            "Error with oodle_safe::decompress. Size: {}",
                            decompressed_size
                        ),
                    )),
                }
            }
            _ => Err(io::Error::new(
                io::ErrorKind::Other,
                "Unsupported compression method",
            )),
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_pick() {
        let file =
            File::open("E:/Games/Steam/steamapps/common/New World/assets/DataStrm-part1.pak")
                .unwrap();
        let archive = Arc::new(RwLock::new(ZipArchive::new(file).unwrap()));

        assert!(PakFile::new(archive)
            .entry("coatgen/65e9a962/08qp01_props_32x32_mesh_chunk_63_42_1__18623649790.cgf")
            .is_some())
    }

    #[test]
    fn test_decompress() {
        let file =
            File::open("E:/Games/Steam/steamapps/common/New World/assets/DataStrm-part1.pak")
                .unwrap();
        let archive = Arc::new(RwLock::new(ZipArchive::new(file).unwrap()));

        assert!(PakFile::new(archive)
            .entry("coatgen/65e9a962/08qp01_props_32x32_mesh_chunk_63_42_1__18623649790.cgf")
            .expect("")
            .decompress()
            .is_ok())
    }

    // #[tokio::test]
    // async fn test_process_all() {
    //     let file = "E:/Games/Steam/steamapps/common/New World/assets/DataStrm-part1.pak";

    //     assert!(PakFile::new(file)
    //         .await
    //         .unwrap()
    //         .process_all(|mut entry| async move {
    //             // let mut buf =vec![];
    //             assert!(entry.decompress().await.is_ok());
    //             Ok(())
    //         })
    //         .await
    //         .is_ok())
    // }
}
