use std::io::{self, Cursor, Read};

use zip::read::ZipFile;

use crate::azcs::{self, is_azcs};

pub enum Decompressor {
    Stored,
    Deflated,
    Unsupported,
}

impl Decompressor {
    pub fn decompress(&mut self, entry: &mut ZipFile) -> io::Result<Vec<u8>> {
        let reader = match self {
            Self::Stored | Self::Deflated => {
                let mut buf = vec![];
                entry.read_to_end(&mut buf)?;
                Ok(buf)
            }
            Self::Unsupported => {
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
                    Ok(_) => Ok(decompressed),
                    Err(_) => Err(io::Error::new(
                        io::ErrorKind::Other,
                        format!(
                            "Error with oodle_safe::decompress. Size: {}",
                            decompressed_size
                        ),
                    )),
                }
            }
        }?;

        let mut reader = Cursor::new(reader);
        let mut sig = [0; 5];
        reader.read_exact(&mut sig)?;

        match is_azcs(&mut reader, &mut sig) {
            Ok(header) => {
                // dbg!("Decompressing AZCS");

                let mut buf = vec![];
                let mut reader = azcs::decompress(&mut reader, &header)?;
                reader.read_to_end(&mut buf)?;
                Ok(buf)
            }
            Err(_) => {
                let mut buf = vec![];
                buf.extend_from_slice(&sig);
                reader.read_to_end(&mut buf)?;
                Ok(buf)
            }
        }
    }
}
