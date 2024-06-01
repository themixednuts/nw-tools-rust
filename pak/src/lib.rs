pub mod azcs;
mod buffers;
pub mod decompressor;
pub mod reader;

use decompressor::Decompressor;
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
    pub fn decompress(&mut self) -> io::Result<Cursor<Vec<u8>>> {
        let archive = self.archive.clone();
        let mut archive_lock = archive.write().unwrap();
        let mut entry = archive_lock.by_index_raw(self.index)?;

        let mut decompressor = Self::get_decompressor(entry.compression())?;
        Ok(Cursor::new(decompressor.decompress(&mut entry)?))
    }

    fn get_decompressor(method: CompressionMethod) -> io::Result<Decompressor> {
        match method {
            CompressionMethod::Stored => Ok(Decompressor::Stored),
            CompressionMethod::Deflated => Ok(Decompressor::Deflated),
            CompressionMethod::Unsupported(15) => Ok(Decompressor::Unsupported),
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
