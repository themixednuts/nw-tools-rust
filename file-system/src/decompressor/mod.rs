use crate::azcs::{self, is_azcs};
use flate2::Decompress;
use std::io::{self, Cursor, Read, Write};
use zip::{read::ZipFile, CompressionMethod};

pub trait ZipFileExt {
    fn decompress(&mut self, buf: &mut impl Write) -> std::io::Result<u64>;
}

impl ZipFileExt for ZipFile<'_> {
    fn decompress(&mut self, buf: &mut impl Write) -> std::io::Result<u64> {
        decompress_zip(self, buf)
    }
}

pub fn handle_azcs(reader: &mut (impl Read + Unpin), buf: &mut impl Write) -> io::Result<u64> {
    let mut sig = [0; 5];
    reader.read_exact(&mut sig)?;
    // dbg!(&sig);

    if is_azcs(&mut sig) {
        let mut reader = std::io::BufReader::new(azcs::decompress(sig.chain(reader))?);
        // object_stream::parser(&mut reader)
        //     .unwrap()
        //     .to_xml(buf, &fs.uuids, &fs.crcs);
        // let mut reader = object_stream::parser(&mut reader)
        //     .unwrap()
        //     .to_json(&fs.uuids, &fs.crcs);
        // Ok(0)
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
        CompressionMethod::Stored => match handle_azcs(zip, buf) {
            Ok(size) => Ok(size),
            Err(e) => Err(e),
        },
        CompressionMethod::Deflated => {
            let mut bytes = [0; 2];
            zip.read_exact(&mut bytes)?;
            if [0x78, 0xda] == bytes {
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
                        Err(e)
                    }
                }
            } else {
                let mut zip = flate2::read::DeflateDecoder::new(Cursor::new(bytes).chain(zip));
                match handle_azcs(&mut zip, buf) {
                    Ok(size) => Ok(size),
                    Err(e) => {
                        dbg!(bytes);
                        (0..20).for_each(|_| eprintln!("DEFLATED"));
                        Err(e)
                    }
                }
            }
        }
        #[allow(deprecated)]
        CompressionMethod::Unsupported(15) => {
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
            "CompressionMethod not supported",
        )),
    }
}
