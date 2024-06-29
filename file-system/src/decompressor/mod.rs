use crate::azcs::{self, is_azcs};
use std::io::{self, BufReader, Cursor, Read};
use zip::{read::ZipFile, CompressionMethod};

enum Reader<'a> {
    ZipFile(ZipFile<'a>),
    Cursor(Cursor<Vec<u8>>),
}

impl<'a> Read for Reader<'a> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            Reader::ZipFile(reader) => reader.read(buf),
            Reader::Cursor(reader) => reader.read(buf),
        }
    }
}

pub trait ZipFileExt {
    fn decompress(self, buf: &mut Vec<u8>) -> std::io::Result<u64>;
}

impl ZipFileExt for ZipFile<'_> {
    fn decompress(mut self, buf: &mut Vec<u8>) -> std::io::Result<u64> {
        let mut reader = match self.compression() {
            CompressionMethod::Stored | CompressionMethod::Deflated => Reader::ZipFile(self),
            #[allow(deprecated)]
            CompressionMethod::Unsupported(15) => {
                let mut compressed = vec![];
                std::io::copy(&mut self, &mut compressed)?;

                let decompressed_size = self.size() as usize;
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
                    Ok(_) => Reader::Cursor(Cursor::new(decompressed)),
                    Err(_) => {
                        return Err(io::Error::new(
                            io::ErrorKind::Other,
                            format!(
                                "Error with oodle_safe::decompress. Size: {}",
                                decompressed_size
                            ),
                        ))
                    }
                }
            }
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    "CompressionMethod not supported",
                ))
            }
        };

        let mut sig = [0; 5];
        reader.read_exact(&mut sig)?;

        match is_azcs(&mut reader, &mut sig) {
            Ok(header) => {
                let mut reader = azcs::decompress(&mut reader, &header)?;
                std::io::copy(&mut reader, buf)
            }
            Err(_) => {
                let sig = Cursor::new(sig);
                let mut chained = sig.chain(reader);
                std::io::copy(&mut chained, buf)
            }
        }
    }
}
