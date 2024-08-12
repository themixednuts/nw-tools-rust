use crate::{
    azcs::{self, is_azcs},
    FILESYSTEM,
};
use cli::{
    commands::{
        extract::{DatasheetFormat, ObjectStreamFormat},
        Commands,
    },
    ARGS,
};
use datasheet::Datasheet;
use flate2::Decompress;
use object_stream::{from_reader, JSONObjectStream, ObjectStream, XMLObjectStream};
use quick_xml::se::Serializer;
use serde::Serialize;
use std::io::{self, Chain, Cursor, Read, Write};
use zip::{read::ZipFile, CompressionMethod};

pub trait ZipFileExt {
    fn decompress(&mut self, buf: &mut impl Write) -> std::io::Result<u64>;
}

impl ZipFileExt for ZipFile<'_> {
    fn decompress(&mut self, buf: &mut impl Write) -> std::io::Result<u64> {
        decompress_zip(self, buf)
    }
}

enum GameReader<'a, R: Read> {
    Chain(Chain<&'a [u8], &'a mut R>),
    Reader(R),
}

impl<'a, R: Read> Read for GameReader<'a, R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            GameReader::Chain(chain) => chain.read(buf),
            GameReader::Reader(reader) => reader.read(buf),
        }
    }
}
pub fn to_writer(mut reader: impl Read + Unpin, buf: &mut impl Write) -> io::Result<u64> {
    let mut sig = [0; 4];
    reader.read_exact(&mut sig).unwrap();

    if is_azcs(&mut sig) {
        let cursor = Cursor::new(sig.to_owned());
        let reader = azcs::decompress(cursor.chain(reader)).unwrap();
        to_writer_internal(reader, buf)
    } else {
        let cursor = Cursor::new(sig.to_owned());
        let reader = cursor.chain(reader);
        to_writer_internal(reader, buf)
    }
}

fn to_writer_internal<R, W>(mut reader: R, writer: &mut W) -> io::Result<u64>
where
    R: Read,
    W: Write,
{
    let mut sig = [0; 5];
    reader.read_exact(&mut sig)?;
    match &sig {
        // Luac
        [0x04, 0x00, 0x1B, 0x4C, 0x75] => {
            let buf = sig[2..5].to_owned();
            std::io::copy(&mut buf.chain(reader), writer)
        }
        // ObjectStream
        [0x00, 0x00, 0x00, 0x00, 0x03] => {
            let args = &ARGS;
            match &args.command {
                Commands::Extract(e) => {
                    let fmt = &e.objectstream;

                    // early return no serialziation
                    if fmt == &ObjectStreamFormat::BYTES {
                        return std::io::copy(&mut sig.chain(reader), writer);
                    };
                    let hashes = if let Some(fs) = FILESYSTEM.get() {
                        Some(&fs.hashes)
                    } else {
                        None
                    };

                    let Ok(obj_stream) = from_reader(&mut sig.chain(&mut reader), hashes) else {
                        return std::io::copy(&mut sig.chain(&mut reader), writer);
                    };
                    match fmt {
                        ObjectStreamFormat::XML => {
                            let obj_stream = XMLObjectStream::from(obj_stream);
                            let mut buf = String::new();
                            let mut ser = Serializer::new(&mut buf);
                            ser.indent('\t', 2);
                            obj_stream.serialize(ser).unwrap();
                            std::io::copy(&mut buf.as_bytes(), writer)
                        }
                        ObjectStreamFormat::JSON => {
                            let obj_stream = JSONObjectStream::from(obj_stream);
                            let string = serde_json::to_string(&obj_stream)
                                .expect("couldnt parse object stream to json");
                            std::io::copy(&mut string.as_bytes(), writer)
                        }
                        ObjectStreamFormat::JSONPRETTY => {
                            let obj_stream = JSONObjectStream::from(obj_stream);
                            let string = serde_json::to_string_pretty(&obj_stream)
                                .expect("couldnt parse object stream to json");
                            std::io::copy(&mut string.as_bytes(), writer)
                        }
                        _ => std::io::copy(&mut sig.chain(reader), writer),
                    }
                }
            }
        }
        // datasheets
        [0x11, 0x00, 0x00, 0x00, _] => {
            let args = &ARGS;
            match &args.command {
                Commands::Extract(e) => {
                    let fmt = &e.datasheet;

                    // early return no serialziation
                    if fmt == &DatasheetFormat::BYTES {
                        return std::io::copy(&mut sig.chain(reader), writer);
                    };

                    let mut reader = sig.chain(reader);
                    let datasheet = Datasheet::from(&mut reader);

                    match fmt {
                        DatasheetFormat::JSON => {
                            let string = datasheet.to_json_simd(false)?;
                            std::io::copy(&mut string.as_bytes(), writer)
                        }
                        DatasheetFormat::JSONPRETTY => {
                            let string = datasheet.to_json_simd(true)?;
                            std::io::copy(&mut string.as_bytes(), writer)
                        }
                        DatasheetFormat::YAML => {
                            let string = datasheet.to_yaml();
                            std::io::copy(&mut string.as_bytes(), writer)
                        }
                        DatasheetFormat::CSV => {
                            let string = datasheet.to_csv();
                            std::io::copy(&mut string.as_bytes(), writer)
                        }
                        _ => std::io::copy(&mut sig.chain(reader), writer),
                    }
                }
            }
        }
        _ => std::io::copy(&mut sig.chain(reader), writer),
    }
}

#[derive(Debug)]
struct GameFile<'a, R> {
    inner: &'a mut R,
    // pos: usize,
}

// impl<'a, R: Read> Read for GameFile<'a, R> {
//     fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
//         let mut sig = [0; 5];
//         self.inner.read_exact(&mut sig)?;
//     }
// }

pub fn decompress_zip(zip: &mut ZipFile, buf: &mut impl Write) -> io::Result<u64> {
    if zip.size() == 0 {
        return Ok(0);
    }

    match zip.compression() {
        CompressionMethod::Stored => match to_writer(zip, buf) {
            Ok(size) => Ok(size),
            Err(e) => {
                (0..20).for_each(|_| eprintln!("{}", e));
                Err(e)
            }
        },
        CompressionMethod::Deflated => {
            let mut bytes = [0; 2];
            zip.read_exact(&mut bytes)?;
            if [0x78, 0xda] == bytes {
                let mut zip = flate2::read::ZlibDecoder::new_with_decompress(
                    Cursor::new(bytes).chain(zip),
                    Decompress::new(true),
                );
                match to_writer(&mut zip, buf) {
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
                match to_writer(&mut zip, buf) {
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
                Ok(_) => to_writer(&mut Cursor::new(decompressed), buf),
                Err(_) => {
                    dbg!(&decompressed[..5]);
                    (0..20).for_each(|_| eprintln!("oodle"));
                    Err(io::Error::new(
                        io::ErrorKind::Other,
                        format!(
                            "Error with oodle_safe::decompress. Size: {}",
                            decompressed_size
                        ),
                    ))
                }
            }
        }
        _ => Err(io::Error::new(
            io::ErrorKind::Other,
            "CompressionMethod not supported",
        )),
    }
}
