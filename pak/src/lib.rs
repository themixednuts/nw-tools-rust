pub mod azcs;
pub mod reader;

use async_trait::async_trait;
use azcs::is_azcs;
use futures::stream::{self, StreamExt};
use oodle_safe;
use std::fs::File;
use std::io::{prelude::*, Cursor};
use std::sync::Arc;
use tokio::{self, spawn};
use tokio::io::{self, AsyncRead, AsyncReadExt};
use tokio::sync::Mutex;
use tokio::task::spawn_blocking;
use zip::result::ZipResult;
use zip::{read::ZipFile, CompressionMethod, ZipArchive};

// var azcsSig = []byte{0x41, 0x5a, 0x43, 0x53};
// var luacSig = []byte{0x04, 0x00, 0x1b, 0x4c, 0x75, 0x61};

pub fn open(path: &str) -> ZipResult<ZipArchive<File>> {
    let file = File::open(path)?;
    ZipArchive::new(file)
}

#[derive(Debug)]
pub struct Pak {
    archive: ZipArchive<File>,
}

impl Pak {
    pub fn new(archive: ZipArchive<File>) -> Self {
        Self { archive }
    }

    pub fn pick<'a>(&'a mut self, path: &str) -> Option<PakFile<'a>> {
        let index = self.archive.index_for_path(path)?;
        Some(PakFile {
            archive: &mut self.archive,
            index,
        })
    }

    pub async fn process_all<'a, F, Fut>(&mut self, mut processor: F) -> io::Result<()>
    where
        F: FnMut(PakFile) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = io::Result<()>> + Send,
    {
        let num_files = self.archive.len();
        let mut handles = vec![];

        for i in 0..num_files {
            let handle = spawn(move {
                processor(PakFile {
                    archive: &mut self.archive,
                    index: i,
                })
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.await?;
        }

        Ok(())
    }
}

pub struct PakFile<'a> {
    archive: &'a mut ZipArchive<File>,
    index: usize,
}

impl<'a> PakFile<'a> {
    pub fn decompress(&mut self) -> io::Result<Box<dyn Read>> {
        let mut entry = self.archive.by_index_raw(self.index)?;
        match entry.compression() {
            CompressionMethod::Stored | CompressionMethod::Deflated => {
                let mut sig = [0; 4];
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
                        format!("Error with oodle_safe::decompress. Size: {decompressed_size}"),
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
    fn test_open() {
        let file = open("E:/Games/Steam/steamapps/common/New World/assets/DataStrm-part1.pak");
        assert!(file.is_ok());
    }

    #[test]
    fn test_pick() {
        let file =
            open("E:/Games/Steam/steamapps/common/New World/assets/DataStrm-part1.pak").unwrap();

        assert!(Pak::new(file)
            .pick("coatgen/65e9a962/08qp01_props_32x32_mesh_chunk_63_42_1__18623649790.cgf")
            .is_some())
    }

    #[test]
    fn test_decompress() {
        let file =
            open("E:/Games/Steam/steamapps/common/New World/assets/DataStrm-part1.pak").unwrap();

        assert!(Pak::new(file)
            .pick("coatgen/65e9a962/08qp01_props_32x32_mesh_chunk_63_42_1__18623649790.cgf")
            .unwrap()
            .decompress()
            .is_ok())
    }

    #[tokio::test]
    async fn test_process_all() {
        let file =
            open("E:/Games/Steam/steamapps/common/New World/assets/DataStrm-part1.pak").unwrap();

        Pak::new(file)
            .process_all(|mut entry| async move {
                let mut buf = vec![];
                entry.decompress().unwrap().read_to_end(&mut buf);
                Ok(())
            })
            .await
            .unwrap();
    }
}
