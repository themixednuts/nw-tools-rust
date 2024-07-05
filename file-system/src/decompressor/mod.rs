use crate::azcs::{self, is_azcs, Header};
use flate2::Decompress;
use regex::bytes;
use std::io::{self, BufReader, Cursor, Read, Write};
use zip::{read::ZipFile, unstable::stream::ZipStreamReader, CompressionMethod};

enum Reader<'a> {
    ZipFile(&'a mut ZipFile<'a>),
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

pub trait ZipFileExt<'a> {
    fn decompress(&mut self, buf: &mut std::io::BufWriter<impl Write>) -> std::io::Result<u64>;
}

impl<'a> ZipFileExt<'a> for ZipFile<'a> {
    fn decompress(&mut self, buf: &mut std::io::BufWriter<impl Write>) -> std::io::Result<u64> {
        decompress_zip(self, buf)
    }
}

fn handle_azcs(mut reader: &mut (impl Read + Unpin), buf: &mut impl Write) -> io::Result<u64> {
    let mut sig = [0; 5];
    reader.read_exact(&mut sig)?;
    // dbg!(&sig);

    if is_azcs(&mut sig) {
        let mut reader = std::io::BufReader::new(azcs::decompress(&mut reader)?);
        std::io::copy(&mut reader, buf)
    } else {
        let sig = Cursor::new(sig);
        let mut chained = std::io::BufReader::new(sig.chain(reader));
        std::io::copy(&mut chained, buf)
    }
}

pub fn decompress_zip(zip: &mut ZipFile, buf: &mut impl Write) -> io::Result<u64> {
    if zip.size() == 0 {
        return Ok(0);
    }

    match zip.compression() {
        CompressionMethod::Stored => {
            // eprintln!("STORED");
            handle_azcs(zip, buf)
        }
        CompressionMethod::Deflated => {
            let mut bytes = [0; 2];
            zip.read_exact(&mut bytes)?;
            if &[0x78, 0xda] == &bytes {
                let mut zip = flate2::read::ZlibDecoder::new_with_decompress(
                    Cursor::new(bytes).chain(zip),
                    Decompress::new(true),
                );
                match handle_azcs(&mut zip, buf) {
                    Ok(size) => Ok(size),
                    Err(e) => {
                        (0..20).for_each(|_| {
                            dbg!(bytes);
                            eprintln!("ZLIB DEFLATED")
                        });
                        // dbg!(&zip_decoder.total_in(), &zip_decoder.total_out());
                        Err(e)
                    }
                }
            } else {
                let mut zip = flate2::read::DeflateDecoder::new(Cursor::new(bytes).chain(zip));
                match handle_azcs(&mut zip, buf) {
                    Ok(size) => Ok(size),
                    Err(e) => {
                        dbg!(bytes);
                        // (0..20).for_each(|_| eprintln!("DEFLATED"));
                        // dbg!(&zip_decoder.total_in(), &zip_decoder.total_out());
                        Err(e)
                    }
                }
            }
        }
        #[allow(deprecated)]
        CompressionMethod::Unsupported(15) => {
            // eprintln!("OODLE");
            let mut compressed = vec![];
            std::io::copy(zip, &mut compressed)?;

            let decompressed_size = zip.size() as usize;
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
                Ok(_) => handle_azcs(&mut Cursor::new(decompressed), buf),
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
    }
}
