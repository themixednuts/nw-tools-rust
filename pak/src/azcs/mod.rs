use crate::reader::ReadExt;
use flate2::bufread::ZlibDecoder;
use std::io::{self, Cursor, Error, Read};
use uuid::Uuid;

#[cfg(bench)]
use crate::buffers::BufferPool;

const AZCS_SIGNATURE: &'static [u8; 4] = b"AZCS";
const ST_BINARYFLAG_MASK: u8 = 0xF8;
const ST_BINARY_VALUE_SIZE_MASK: u8 = 0x07;
const ST_BINARYFLAG_ELEMENT_HEADER: u8 = 1 << 3;
const ST_BINARYFLAG_HAS_VALUE: u8 = 1 << 4;
const ST_BINARYFLAG_EXTRA_SIZE_FIELD: u8 = 1 << 5;
const ST_BINARYFLAG_HAS_NAME: u8 = 1 << 6;
const ST_BINARYFLAG_HAS_VERSION: u8 = 1 << 7;
const ST_BINARYFLAG_ELEMENT_END: u8 = 0;

const UNCOMPRESSED_SIGNATURES: [[u8; 5]; 3] = [
    [0x00, 0x00, 0x00, 0x00, 0x03],
    [0x00, 0x00, 0x00, 0x00, 0x02],
    [0x00, 0x00, 0x00, 0x00, 0x01],
];

#[derive(Default, Debug)]
pub struct Element {
    name_crc: Option<u32>,
    version: Option<u8>,
    element_type: Uuid,
    specialized_type: Option<Uuid>,
    data_size: Option<u32>,
    data: Vec<u8>,
    elements: Vec<Element>,
}

#[derive(Default, Debug)]
pub struct Stream {
    version: u32,
    elements: Vec<Element>,
}

#[derive(Debug)]
struct EOE;

impl std::fmt::Display for EOE {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "End of Elements")
    }
}

impl std::error::Error for EOE {}

#[derive(Default, Debug)]
pub struct Header {
    signature: [u8; 4],
    compressor_id: u32,
    uncompressed_size: u64,
}

pub fn parser<R: Read + Sync + Unpin>(reader: &mut R) -> io::Result<Stream> {
    let stream_tag = reader.read_u8()?;

    if stream_tag != 0x00 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "not a valid stream",
        ));
    }

    let version = reader.read_u32()?;
    let mut stream = Stream {
        version,
        elements: vec![],
    };

    loop {
        match read_element(reader, &stream) {
            Ok(element) => stream.elements.push(element),
            Err(e) if e.is::<EOE>() => return Ok(stream),
            Err(e) => return Err(io::Error::new(io::ErrorKind::Other, format!("{e}"))),
        }
    }
}

#[cfg(bench)]
pub fn parser_withpool<R: Read + Sync + Unpin>(reader: &mut R) -> io::Result<Stream> {
    let stream_tag = reader.read_u8()?;

    if stream_tag != 0x00 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "not a valid stream",
        ));
    }

    let version = reader.read_u32()?;
    let mut stream = Stream {
        version,
        elements: vec![],
    };

    let mut pool = BufferPool::new();
    loop {
        match read_element_pool(reader, &stream, &mut pool) {
            Ok(element) => stream.elements.push(element),
            Err(e) if e.is::<EOE>() => return Ok(stream),
            Err(e) => return Err(io::Error::new(io::ErrorKind::Other, format!("{e}"))),
        }
    }
}

pub fn decompress<R: Read + Unpin>(
    reader: &mut R,
    header: &Header,
) -> io::Result<Box<dyn Read + Sync + Unpin + Send>> {
    match header.compressor_id {
        0x73887d3a => handle_zlib(reader),
        0x72fd505e => Err(io::Error::new(
            io::ErrorKind::Other,
            "zstd is not implemented",
        )),
        _ => Err(io::Error::new(
            io::ErrorKind::Other,
            format!("unsupported compressorId: 0x{:08x}", header.compressor_id),
        )),
    }
}

pub fn is_azcs<R: Read>(reader: &mut R, buf: &mut [u8; 5]) -> io::Result<Header> {
    match (is_compressed(buf) || is_uncompressed(buf)) && buf.starts_with(AZCS_SIGNATURE) {
        true => {
            let mut header_data = Header::default();
            header_data.signature = *AZCS_SIGNATURE;

            let mut buffer = [0; 4];
            buffer[0] = buf[4];
            reader.read_exact(&mut buffer[1..])?;

            header_data.compressor_id = u32::from_be_bytes(buffer);
            header_data.uncompressed_size = reader.read_u64()?;
            Ok(header_data)
        }
        false => Err(io::Error::new(
            io::ErrorKind::Other,
            "Not an AZCS file stream",
        )),
    }
}

fn read_element<R: Read>(
    reader: &mut R,
    stream: &Stream,
) -> Result<Element, Box<dyn std::error::Error>> {
    let mut element = Element::default();
    let flags = reader.read_u8()?;

    if flags == ST_BINARYFLAG_ELEMENT_END {
        return Err(Box::new(EOE));
    }

    if flags & ST_BINARYFLAG_HAS_NAME > 0 {
        let name_crc = reader.read_u32()?;
        element.name_crc = Some(name_crc);
    }

    if flags & ST_BINARYFLAG_HAS_VERSION > 0 {
        let version = reader.read_u8()?;
        element.version = Some(version);
    }

    let type_data = reader.read_bytes(16)?;
    element.element_type = Uuid::from_slice(&type_data)?;

    if stream.version == 2 {
        let specialized_type_data = reader.read_bytes(16)?;
        element.specialized_type = Some(Uuid::from_slice(&specialized_type_data)?);
    }

    if flags & ST_BINARYFLAG_HAS_VALUE > 0 {
        let value_bytes = flags & ST_BINARY_VALUE_SIZE_MASK;
        element.data_size = if flags & ST_BINARYFLAG_EXTRA_SIZE_FIELD > 0 {
            match value_bytes {
                1 => Some(reader.read_u8()? as u32),
                2 => Some(reader.read_u16()? as u32),
                4 => Some(reader.read_u32()?),
                _ => {
                    return Err(Box::new(io::Error::new(
                        io::ErrorKind::Other,
                        "unsupported valueBytes",
                    )))
                }
            }
        } else {
            Some(value_bytes as u32)
        };
    }

    if let Some(data_size) = element.data_size {
        element.data = reader.read_bytes(data_size as usize)?;
    }

    loop {
        match read_element(reader, stream) {
            Ok(child_element) => element.elements.push(child_element),
            Err(e) if e.is::<EOE>() => return Ok(element),
            Err(e) => return Err(e),
        }
    }
}

#[cfg(bench)]
fn read_element_pool<R: Read>(
    reader: &mut R,
    stream: &Stream,
    pool: &mut BufferPool,
) -> Result<Element, Box<dyn std::error::Error>> {
    let mut element = Element::default();

    let flags: u8 = pool.get_as(reader, 1)?;

    if flags == ST_BINARYFLAG_ELEMENT_END {
        return Err(Box::new(EOE));
    }

    if flags & ST_BINARYFLAG_HAS_NAME > 0 {
        element.name_crc = Some(pool.get_as(reader, 4)?);
    }

    if flags & ST_BINARYFLAG_HAS_VERSION > 0 {
        element.version = Some(pool.get_as(reader, 1)?);
    }

    let type_data = reader.read_bytes(16)?;
    element.element_type = Uuid::from_slice(&type_data)?;

    if stream.version == 2 {
        let specialized_type_data = reader.read_bytes(16)?;
        element.specialized_type = Some(Uuid::from_slice(&specialized_type_data)?);
    }

    if flags & ST_BINARYFLAG_HAS_VALUE > 0 {
        let value_bytes = flags & ST_BINARY_VALUE_SIZE_MASK;
        element.data_size = if flags & ST_BINARYFLAG_EXTRA_SIZE_FIELD > 0 {
            match value_bytes {
                1 => Some(pool.get_as::<u8>(reader, 1)? as u32),
                2 => Some(pool.get_as::<u16>(reader, 1)? as u32),
                4 => Some(pool.get_as::<u32>(reader, 1)?),
                _ => {
                    return Err(Box::new(io::Error::new(
                        io::ErrorKind::Other,
                        "unsupported valueBytes",
                    )))
                }
            }
        } else {
            Some(value_bytes as u32)
        };
    }

    if let Some(data_size) = element.data_size {
        element.data = reader.read_bytes(data_size as usize)?;
    }

    loop {
        match read_element_pool(reader, stream, pool) {
            Ok(child_element) => element.elements.push(child_element),
            Err(e) if e.is::<EOE>() => return Ok(element),
            Err(e) => return Err(e),
        }
    }
}

pub fn handle_zlib<R: Read + Unpin>(
    reader: &mut R,
) -> io::Result<Box<dyn Read + Sync + Unpin + Send>> {
    let num_seek_points = reader.read_u32()?;
    let num_seek_points_size = num_seek_points * 16;

    let mut compressed = vec![];
    reader.read_to_end(&mut compressed)?;

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

    let reader = Box::new(zr);
    Ok(reader)
}

pub fn is_uncompressed(data: &[u8]) -> bool {
    for &uncompressed_signature in UNCOMPRESSED_SIGNATURES.iter() {
        if data.len() >= uncompressed_signature.len() && data.starts_with(&uncompressed_signature) {
            return true;
        }
    }
    false
}

pub fn is_compressed(data: &[u8]) -> bool {
    if data.len() < AZCS_SIGNATURE.len() {
        return false;
    }

    data.starts_with(AZCS_SIGNATURE)
}
