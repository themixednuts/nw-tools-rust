use serde::Serialize;
use std::io;
use std::{array::TryFromSliceError, io::Read};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MyError {
    #[error("IO error: {0}")]
    IO(#[from] io::Error),

    #[error("Slice conversion error: {0}")]
    TryFromSliceError(#[from] std::array::TryFromSliceError),
}

#[derive(Debug, Serialize)]
pub struct Distribution {
    #[serde(flatten)]
    pub slices: SlicesData,
    #[serde(flatten)]
    pub gatherables: GatherablesData,
    pub unknown1: Unknown,
    pub unknown2: Unknown,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SlicesData {
    slices: Vec<String>,
    variants: Vec<String>,
}

impl SlicesData {
    fn from_reader<R: Read>(value: &mut R) -> Result<Self, MyError> {
        let mut buf = [0u8; 2];
        value.read_exact(&mut buf)?;

        let len = u16::from_le_bytes(buf);

        let mut slices = Vec::with_capacity(len as usize);
        let mut variants = Vec::with_capacity(len as usize);

        for _ in 0..len {
            let mut buf = [0u8; 1];
            value.read_exact(&mut buf)?;
            let len = u8::from_le_bytes(buf);
            let mut string = vec![0; len as usize];
            value.read_exact(&mut string)?;
            assert_eq!(len as usize, string.len());

            let string = String::from_utf8(string).unwrap();
            slices.push(string);
        }
        for _ in 0..len {
            let mut buf = [0u8; 1];
            value.read_exact(&mut buf)?;
            let len = u8::from_le_bytes(buf);
            let mut string = vec![0; len as usize];
            value.read_exact(&mut string)?;
            assert_eq!(len as usize, string.len());

            let string = String::from_utf8(string).unwrap();
            variants.push(string);
        }

        assert_eq!(len as usize, slices.len());
        assert_eq!(len as usize, variants.len());

        Ok(Self { slices, variants })
    }
}

#[derive(Debug, Serialize)]
pub struct GatherablesData {
    indices: Vec<u16>,
    positions: Vec<Position>,

    // possibly a vector3 instead vector2 for rotation/scale
    extra1: Vec<u16>,
    extra2: Vec<u16>,
    extra3: Vec<u8>,
}

impl GatherablesData {
    fn from_reader<R: Read>(value: &mut R) -> Result<Self, MyError> {
        let mut buf = [0u8; 4];
        value.read_exact(&mut buf)?;
        let len = u32::from_le_bytes(buf);

        let mut indices = Vec::with_capacity(len as usize);
        let mut buf = [0u8; 2];
        for _ in 0..len {
            value.read_exact(&mut buf)?;
            indices.push(u16::from_le_bytes(buf));
        }

        assert_eq!(len as usize, indices.len());

        let mut buf = [0u8; 4];
        let mut positions = Vec::with_capacity(len as usize);

        for _ in 0..len {
            value.read_exact(&mut buf)?;
            positions.push(Position::try_from(&buf)?);
        }
        assert_eq!(len as usize, positions.len());

        let mut buf = [0u8; 2];
        let mut extra1 = Vec::with_capacity(len as usize);
        let mut extra2 = Vec::with_capacity(len as usize);
        for _ in 0..len {
            value.read_exact(&mut buf)?;
            extra1.push(u16::from_le_bytes(buf));
        }
        for _ in 0..len {
            value.read_exact(&mut buf)?;
            extra2.push(u16::from_le_bytes(buf));
        }

        assert_eq!(len as usize, extra1.len());
        assert_eq!(len as usize, extra2.len());

        let mut buf = [0u8; 1];
        let mut extra3 = Vec::with_capacity(len as usize);
        for _ in 0..len {
            value.read_exact(&mut buf)?;
            extra3.push(u8::from_le_bytes(buf));
        }
        assert_eq!(len as usize, extra3.len());

        Ok(Self {
            indices,
            positions,
            extra1,
            extra2,
            extra3,
        })
    }
}

#[derive(Debug, Serialize)]
pub struct Unknown {
    // possibly a vector3 instead vector2 for rotation/scale
    positions: Vec<Position>,
    extra: Vec<u8>,
}

impl Unknown {
    fn from_reader<R: Read>(value: &mut R) -> Result<Self, MyError> {
        let mut buf = [0u8; 4];
        value.read_exact(&mut buf)?;

        let len = u32::from_le_bytes(buf);
        let mut positions = Vec::with_capacity(len as usize);

        for _ in 0..len {
            value.read_exact(&mut buf)?;
            positions.push(Position::try_from(&buf)?);
        }

        let mut extra = Vec::with_capacity(len as usize);

        let mut buf = [0u8; 1];
        for _ in 0..len {
            value.read_exact(&mut buf)?;
            extra.push(u8::from_le_bytes(buf));
        }

        Ok(Self { positions, extra })
    }
}

#[derive(Debug, Serialize)]
pub struct Position(u16, u16);

impl TryFrom<&[u8; 4]> for Position {
    type Error = TryFromSliceError;

    fn try_from(value: &[u8; 4]) -> Result<Self, Self::Error> {
        Ok(Position(
            u16::from_le_bytes(value[..2].try_into()?),
            u16::from_le_bytes(value[2..].try_into()?),
        ))
    }
}

impl Distribution {
    pub fn from_reader<R: Read>(value: &mut R) -> Result<Self, MyError> {
        let slices = SlicesData::from_reader(value);
        let Ok(slices) = slices else {
            println!("Couldn't read SlicesData");
            return Err(slices.err().unwrap());
        };
        let gatherables = GatherablesData::from_reader(value);
        let Ok(gatherables) = gatherables else {
            println!("Couldn't read gatherablesData");
            return Err(gatherables.err().unwrap());
        };
        let other = Unknown::from_reader(value);
        let Ok(other) = other else {
            println!("Couldn't read otherData");
            return Err(other.err().unwrap());
        };
        let other2 = Unknown::from_reader(value);
        let Ok(other2) = other2 else {
            println!("Couldn't read other2Data");
            return Err(other2.err().unwrap());
        };

        Ok(Distribution {
            slices,
            gatherables,
            unknown1: other,
            unknown2: other2,
        })
    }
}
