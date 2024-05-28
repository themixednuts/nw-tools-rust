use byteorder::{self, BigEndian, ReadBytesExt};
use flate2;
use flate2::read::ZlibDecoder;
use oodle_safe;
use std::error::Error;
use std::io::{prelude::*, Cursor};
use std::rc::Rc;
use std::{fs::File, io};
use zip::{read::ZipFile, CompressionMethod, ZipArchive};

// var azcsSig = []byte{0x41, 0x5a, 0x43, 0x53}
// var luacSig = []byte{0x04, 0x00, 0x1b, 0x4c, 0x75, 0x61}

const SIGNATURE: &'static [u8] = b"AZCS";

struct Header {
    signature: String,
    compressor_id: u32,
    uncompressed_size: u64,
}

impl Default for Header {
    fn default() -> Self {
        Self {
            signature: String::new(),
            compressor_id: 0,
            uncompressed_size: 0,
        }
    }
}

pub fn open(path: &str) -> Result<ZipArchive<File>, zip::result::ZipError> {
    let file = File::open(path).expect("Open file");
    ZipArchive::new(file)
}

pub fn parse<'a>(
    file: &'a mut zip::ZipArchive<File>,
    path: &str,
) -> Result<ZipFile<'a>, Box<dyn Error>> {
    let idx = file.index_for_path(path).unwrap();
    let entry = file.by_index_raw(idx)?;
    if !entry.is_file() {
        return Err("Not a file entry".into());
    }
    Ok(entry)
}

pub fn decompress(entry: Rc<ZipFile<'_>>) -> Result<Box<dyn Read>, io::Error> {
    let mut entry = Rc::try_unwrap(entry)
        .map_err(|_| io::Error::new(io::ErrorKind::Other, "Failed to unwrap Rc"))?;

    match entry.compression() {
        CompressionMethod::Stored | CompressionMethod::Deflated => {
            let mut buf = vec![0; 4]; // Read the first four bytes
            entry.read_exact(&mut buf)?;

            if buf == SIGNATURE {
                // println!("this is azcs");
                let mut header_data = Header::default();
                header_data.signature = String::from_utf8_lossy(&buf).to_string();
                header_data.compressor_id = read_uint32(&mut entry)?;
                header_data.uncompressed_size = read_uint64(&mut entry)?;

                match header_data.compressor_id {
                    0x73887d3a => handle_zlib(&mut entry),
                    0x72fd505e => Err(io::Error::new(
                        io::ErrorKind::Other,
                        "zstd is not implemented",
                    )),
                    _ => Err(io::Error::new(
                        io::ErrorKind::Other,
                        format!(
                            "unsupported compressorId: 0x{:08x}",
                            header_data.compressor_id
                        ),
                    )),
                }?;
            }

            let mut buf = vec![];
            entry.read_to_end(&mut buf)?;

            Ok(Box::new(Cursor::new(buf)))
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

fn read_uint32(entry: &mut ZipFile<'_>) -> Result<u32, io::Error> {
    let mut buf = [0u8; 4];
    entry.read_exact(&mut buf)?;
    Ok(u32::from_be_bytes(buf))
}

fn read_uint64(entry: &mut ZipFile<'_>) -> Result<u64, io::Error> {
    let mut buf = [0u8; 8];
    entry.read_exact(&mut buf)?;
    Ok(u64::from_be_bytes(buf))
}

fn handle_zlib(entry: &mut ZipFile<'_>) -> Result<Box<dyn Read>, io::Error> {
    let mut buf = [0u8; 4];
    entry.read_exact(&mut buf)?;

    let num_seek_points = (&buf[..]).read_u32::<BigEndian>()?;
    let num_seek_points_size = num_seek_points * 16;

    let mut compressed = vec![];
    entry.read_to_end(&mut compressed)?;
    println!("Compressed data (bytes): {:?}", compressed);

    // Calculate the number of bytes to read for seek points
    let data_len = compressed.len();
    if data_len < num_seek_points_size as usize {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "Invalid compressed data size",
        ));
    }

    let data_without_seek_points_len = data_len - num_seek_points_size as usize;

    // Create a cursor over the relevant portion of the data
    let data_cursor = Cursor::new(compressed);
    let zr = ZlibDecoder::new(data_cursor.take(data_without_seek_points_len as u64));

    // Return the zlib decoder wrapped in a Box<dyn Read>
    Ok(Box::new(zr))
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
    fn test_parse() {
        let mut file =
            open("E:/Games/Steam/steamapps/common/New World/assets/DataStrm-part1.pak").unwrap();
        let archive = parse(
            &mut file,
            "coatgen/65e9a962/08qp01_props_32x32_mesh_chunk_63_42_1__18623649790.cgf",
        );

        assert!(archive.is_ok())
    }

    #[test]
    fn test_decompress() {
        let mut file =
            open("E:/Games/Steam/steamapps/common/New World/assets/DataStrm-part1.pak").unwrap();
        let archive = parse(
            &mut file,
            "coatgen/65e9a962/08qp01_props_32x32_mesh_chunk_63_42_1__18623649790.cgf",
        )
        .unwrap();

        let reader = decompress(Rc::new(archive));

        assert!(reader.is_ok())
    }
}
