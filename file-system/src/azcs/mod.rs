use flate2::{bufread::ZlibDecoder, Decompress};
use std::io::{self, BufReader, Cursor, Read};

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

impl<R: Read> From<&mut R> for Header {
    fn from(value: &mut R) -> Self {
        Self {
            signature: {
                let mut buf = [0; 4];
                value.read_exact(&mut buf).unwrap();
                buf
            },
            compressor_id: {
                let mut buf = [0; 4];
                value.read_exact(&mut buf).unwrap();
                u32::from_be_bytes(buf)
            },
            uncompressed_size: {
                let mut buf = [0; 8];
                value.read_exact(&mut buf).unwrap();
                u64::from_be_bytes(buf)
            },
        }
    }
}

pub fn decompress<R>(mut reader: R) -> io::Result<impl Read + Unpin>
where
    R: Read + Unpin,
{
    let header = { Header::from(&mut reader) };
    match header.compressor_id {
        0x73887d3a => handle_zlib(reader),
        0x72fd505e => Err(io::Error::new(
            io::ErrorKind::Other,
            "zstd is not implemented",
        )),
        _ => {
            dbg!(&header);
            Err(io::Error::new(
                io::ErrorKind::Other,
                format!("Unsupported compressor_id: 0x{:08x}", header.compressor_id),
            ))
        }
    }
}

pub fn is_azcs(sig: &mut [u8; 5]) -> bool {
    (is_compressed(sig) || is_uncompressed(sig)) && sig.starts_with(AZCS_SIGNATURE)
}

pub fn handle_zlib<R>(mut reader: R) -> io::Result<impl Read + Unpin>
where
    R: Read + Unpin,
{
    let mut buf = [0; 4];
    reader.read_exact(&mut buf)?;
    let num_seek_points = u32::from_be_bytes(buf);
    let num_seek_points_size = num_seek_points * 16;

    // let mut compressed = vec![];
    // reader.read_to_end(&mut compressed)?;

    // // Calculate the number of bytes to read for seek points
    // let data_len = compressed.len();
    // if data_len < num_seek_points_size as usize {
    //     return Err(io::Error::new(
    //         io::ErrorKind::Other,
    //         "Invalid compressed data size",
    //     ));
    // }
    // let data_without_seek_points_len = data_len - num_seek_points_size as usize;

    // let data_cursor = Cursor::new(compressed).take(data_without_seek_points_len as u64);
    // ZlibDecoder::new_with_decompress(reader, Decompress::new(true));
    let reader = BufReader::new(reader);
    let zr = ZlibDecoder::new(reader);

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
