use crate::{
    azcs::{self, is_azcs},
    file_type, FileType, FILESYSTEM,
};
use cli::common::{datasheet::DatasheetFormat, objectstream::ObjectStreamFormat};
use datasheet::Datasheet;
use flate2::Decompress;
use object_stream::{from_reader, JSONObjectStream, XMLObjectStream};
use quick_xml::se::Serializer;
use serde::Serialize;
use std::io::{self, Cursor, Read, Write};
use zip::{read::ZipFile, CompressionMethod};

pub trait ZipFileExt {
    fn decompress(
        &mut self,
        buf: &mut impl Write,
    ) -> std::io::Result<(u64, FileType, Option<Metadata>)>;
}

impl ZipFileExt for ZipFile<'_> {
    fn decompress(
        &mut self,
        buf: &mut impl Write,
    ) -> std::io::Result<(u64, FileType, Option<Metadata>)> {
        decompress_zip(self, buf)
    }
}

pub fn to_writer(
    mut reader: impl Read + Unpin,
    buf: &mut impl Write,
) -> io::Result<(u64, FileType, Option<Metadata>)> {
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

pub enum Metadata {
    Datasheet(Datasheet),
}

// TODO: refactor this, should really be two different things
fn to_writer_internal<R, W>(
    mut reader: R,
    writer: &mut W,
) -> io::Result<(u64, FileType, Option<Metadata>)>
where
    R: Read,
    W: Write,
{
    let mut sig = [0; 5];
    reader.read_exact(&mut sig)?;
    let file_type = file_type(&sig)?;
    let mut extra = None;

    let size = match &file_type {
        FileType::Luac => {
            let buf = sig[2..5].to_owned();
            std::io::copy(&mut buf.chain(reader), writer)
        }
        FileType::ObjectStream(fmt) => {
            // early return no serialziation
            if **fmt == ObjectStreamFormat::BYTES {
                return Ok((
                    std::io::copy(&mut sig.chain(reader), writer)?,
                    file_type,
                    None,
                ));
            };
            let hashes = if let Some(fs) = FILESYSTEM.get() {
                Some(&fs.hashes)
            } else {
                None
            };

            let Ok(obj_stream) = from_reader(&mut sig.chain(&mut reader), hashes) else {
                return Ok((
                    std::io::copy(&mut sig.chain(reader), writer)?,
                    file_type,
                    None,
                ));
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
                ObjectStreamFormat::PRETTY => {
                    let obj_stream = JSONObjectStream::from(obj_stream);
                    let string = serde_json::to_string_pretty(&obj_stream)
                        .expect("couldnt parse object stream to json");
                    std::io::copy(&mut string.as_bytes(), writer)
                }
                _ => std::io::copy(&mut sig.chain(reader), writer),
            }
        }
        FileType::Datasheet(fmt) => {
            // early return no serialziation
            let mut reader = sig.chain(reader);
            let datasheet = Datasheet::from(&mut reader);

            if **fmt == DatasheetFormat::BYTES {
                return Ok((
                    std::io::copy(&mut sig.chain(reader), writer)?,
                    file_type,
                    Some(Metadata::Datasheet(datasheet.to_owned())),
                ));
            };

            extra = Some(Metadata::Datasheet(datasheet.to_owned()));

            match fmt {
                DatasheetFormat::MINI => {
                    let string = datasheet.to_json_simd(false)?;
                    std::io::copy(&mut string.as_bytes(), writer)
                }
                DatasheetFormat::PRETTY => {
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
                DatasheetFormat::BYTES => std::io::copy(&mut sig.chain(reader), writer),
                DatasheetFormat::XML => todo!(),
                DatasheetFormat::SQL => {
                    let string = datasheet.to_sql();
                    std::io::copy(&mut string.as_bytes(), writer)
                }
            }
        }
        _ => std::io::copy(&mut sig.chain(reader), writer),
    }?;

    Ok((size, file_type, extra))
}

pub fn decompress_zip(
    zip: &mut ZipFile,
    buf: &mut impl Write,
) -> io::Result<(u64, FileType, Option<Metadata>)> {
    if zip.size() == 0 {
        return Ok((0, FileType::Other, None));
    }

    match zip.compression() {
        CompressionMethod::Stored => match to_writer(zip, buf) {
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
                match to_writer(&mut zip, buf) {
                    Ok(size) => Ok(size),
                    Err(e) => Err(e),
                }
            } else {
                let mut zip = flate2::read::DeflateDecoder::new(Cursor::new(bytes).chain(zip));
                match to_writer(&mut zip, buf) {
                    Ok(size) => Ok(size),
                    Err(e) => Err(e),
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
