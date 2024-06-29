use flate2::bufread::ZlibDecoder;
use std::io::{self, Cursor, Read};

const AZCS_SIGNATURE: &'static [u8; 4] = b"AZCS";

const UNCOMPRESSED_SIGNATURES: [[u8; 5]; 3] = [
    [0x00, 0x00, 0x00, 0x00, 0x03],
    [0x00, 0x00, 0x00, 0x00, 0x02],
    [0x00, 0x00, 0x00, 0x00, 0x01],
];

#[derive(Default, Debug)]
pub struct Header {
    signature: [u8; 4],
    compressor_id: u32,
    uncompressed_size: u64,
}

pub fn decompress<R>(reader: &mut R, header: &Header) -> io::Result<impl Read + Unpin + Send>
where
    R: Read + Unpin,
{
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

pub fn is_azcs<R>(reader: &mut R, buf: &mut [u8; 5]) -> io::Result<Header>
where
    R: Read + Unpin,
{
    match (is_compressed(buf) || is_uncompressed(buf)) && buf.starts_with(AZCS_SIGNATURE) {
        true => {
            let mut header_data = Header::default();
            header_data.signature = *AZCS_SIGNATURE;

            let mut buffer = [0; 4];
            buffer[0] = buf[4];
            reader.read_exact(&mut buffer[1..])?;

            header_data.compressor_id = u32::from_be_bytes(buffer);

            let mut buffer = [0; 8];
            reader.read_exact(&mut buffer)?;
            header_data.uncompressed_size = u64::from_be_bytes(buffer);
            Ok(header_data)
        }
        false => Err(io::Error::new(
            io::ErrorKind::Other,
            "Not an AZCS file stream",
        )),
    }
}

pub fn handle_zlib<R>(reader: &mut R) -> io::Result<impl Read + Unpin + Send>
where
    R: Read + Unpin,
{
    let mut buf = [0; 4];
    reader.read_exact(&mut buf)?;
    let num_seek_points = u32::from_be_bytes(buf);
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

    let data_cursor = Cursor::new(compressed).take(data_without_seek_points_len as u64);
    let zr = ZlibDecoder::new(data_cursor);

    Ok(zr)
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
