use std::io::{Read, Result};

use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VShapeC {
    version: u32,
    vertices: Vec<Vector3>,
    #[serde(rename = "metadata")]
    metadata: Vec<MetaData>,
    field: u32,
    field2: u32,
    flags: [u8; 4],
    field3: u32,
}

impl VShapeC {
    pub fn from_reader<R: Read>(mut reader: R) -> Result<Self> {
        let mut buf = [0u8; 4];

        reader.read_exact(&mut buf)?;
        let version = u32::from_le_bytes(buf);

        reader.read_exact(&mut buf)?;
        let len = u32::from_le_bytes(buf);

        let iter = std::iter::repeat_with(|| {
            reader.read_exact(&mut buf).unwrap();
            let x = f32::from_le_bytes(buf);

            reader.read_exact(&mut buf).unwrap();
            let y = f32::from_le_bytes(buf);

            reader.read_exact(&mut buf).unwrap();
            let z = f32::from_le_bytes(buf);

            Vector3(x, y, z)
        })
        .take(len as usize);
        let vertices = Vec::from_iter(iter);

        reader.read_exact(&mut buf)?;
        let len = u32::from_le_bytes(buf);

        let iter = std::iter::repeat_with(|| {
            reader.read_exact(&mut buf).unwrap();
            let key = u32::from_le_bytes(buf);

            reader.read_exact(&mut buf).unwrap();
            let value = u32::from_le_bytes(buf);

            MetaData { key, value }
        })
        .take(len as usize);
        let meta_data = Vec::from_iter(iter);

        reader.read_exact(&mut buf)?;
        let field = u32::from_le_bytes(buf);

        reader.read_exact(&mut buf)?;
        let field2 = u32::from_le_bytes(buf);

        reader.read_exact(&mut buf)?;
        let flags = buf.clone();

        reader.read_exact(&mut buf)?;
        let field3 = u32::from_le_bytes(buf);

        Ok(Self {
            version,
            vertices,
            metadata: meta_data,
            field,
            field2,
            flags,
            field3,
        })
    }
}

#[derive(Debug, Serialize)]
pub struct Vector3(f32, f32, f32);

#[derive(Debug, Serialize)]
pub struct MetaData {
    key: u32,
    value: u32,
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let bytes = [
            0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0x65, 0xC0, 0xBD, 0x42, 0x31, 0x51,
            0xAB, 0xC2, 0x00, 0x00, 0x00, 0x00, 0x83, 0x05, 0xBD, 0x42, 0x50, 0x2B, 0x2F, 0x42,
            0x00, 0x00, 0x00, 0x00, 0xBA, 0xCF, 0x43, 0x42, 0xB5, 0x5E, 0xCD, 0x42, 0x00, 0x00,
            0x00, 0x00, 0xD1, 0x10, 0xBD, 0xC2, 0xA1, 0xAC, 0xC5, 0x42, 0x00, 0x00, 0x00, 0x00,
            0x2F, 0x0F, 0xF3, 0xC2, 0x0B, 0x39, 0x80, 0x41, 0x00, 0x00, 0x00, 0x00, 0xBA, 0x5D,
            0xF2, 0xC2, 0xDD, 0x30, 0x01, 0xC2, 0x00, 0x00, 0x00, 0x00, 0xD1, 0x43, 0x18, 0xC3,
            0xB6, 0xDB, 0x84, 0xC2, 0x00, 0x00, 0x00, 0x00, 0x27, 0x61, 0x16, 0xC3, 0x65, 0xF6,
            0xD4, 0xC2, 0x00, 0x00, 0x00, 0x00, 0x62, 0x59, 0xFD, 0xC2, 0xF2, 0xA5, 0xE6, 0xC2,
            0x00, 0x00, 0x00, 0x00, 0xA4, 0x26, 0xE0, 0xC2, 0x4A, 0xCA, 0xED, 0xC2, 0x00, 0x00,
            0x00, 0x00, 0x4F, 0x3E, 0xC5, 0xC2, 0xD3, 0x5F, 0xEC, 0xC2, 0x00, 0x00, 0x00, 0x00,
            0x70, 0x8D, 0x9C, 0xC2, 0x40, 0x05, 0xF8, 0xC2, 0x00, 0x00, 0x00, 0x00, 0x32, 0x37,
            0x46, 0xC2, 0x5B, 0xA9, 0x08, 0xC3, 0x00, 0x00, 0x00, 0x00, 0x5A, 0xD2, 0xC6, 0xC1,
            0x99, 0x94, 0x08, 0xC3, 0x00, 0x00, 0x00, 0x00, 0x0A, 0xDB, 0x05, 0xC1, 0x2D, 0x38,
            0xFB, 0xC2, 0x00, 0x00, 0x00, 0x00, 0x7D, 0xA5, 0xF2, 0x41, 0x7B, 0x9D, 0xE7, 0xC2,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x96, 0x43, 0x00, 0x00, 0x00, 0x00,
        ];
        let vshapec = VShapeC::from_reader(&mut bytes.as_slice());
        assert!(vshapec.is_ok());

        if let Ok(vshapec) = vshapec {
            assert_eq!(vshapec.version, 0);
            assert_eq!(vshapec.vertices.len(), 16);
        };
    }
}
