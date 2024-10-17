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

pub struct Distribution {
    pub slices: SlicesData,
    pub gatherables: GatherablesData,
    pub other: Other,
    pub other2: Other,
}

pub struct CompactString {
    len: u8,
    string: Vec<u8>,
}

impl CompactString {
    fn from_reader<R: Read>(value: &mut R) -> Result<Self, MyError> {
        let mut buf = [0u8; 1];
        value.read_exact(&mut buf)?;
        let len = u8::from_le_bytes(buf);
        let mut string = vec![0u8; len as usize];
        value.read_exact(&mut string)?;

        assert_eq!(len as usize, string.len());
        Ok(CompactString { len, string })
    }
}

pub struct SlicesData {
    len: u16,
    slice_names: Vec<CompactString>,
    variant_names: Vec<CompactString>,
}

impl SlicesData {
    fn from_reader<R: Read>(value: &mut R) -> Result<Self, MyError> {
        let mut buf = [0u8; 2];
        value.read_exact(&mut buf)?;

        let len = u16::from_le_bytes(buf);

        let mut slice_names = Vec::with_capacity(len as usize);
        let mut variant_names = Vec::with_capacity(len as usize);

        for _ in 0..len {
            slice_names.push(CompactString::from_reader(value)?);
        }
        for _ in 0..len {
            variant_names.push(CompactString::from_reader(value)?);
        }

        assert_eq!(len as usize, slice_names.len());
        assert_eq!(len as usize, variant_names.len());

        Ok(Self {
            len,
            slice_names,
            variant_names,
        })
    }
}

pub struct GatherablesData {
    len: u32,
    indices: Vec<u16>,
    pos: Vec<Position>,

    // possibly a vector3 instead vector2 for rotation/scale
    field: Vec<u16>,
    field2: Vec<u16>,
    field3: Vec<u8>,
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
        let mut pos = Vec::with_capacity(len as usize);

        for _ in 0..len {
            value.read_exact(&mut buf)?;
            pos.push(Position::try_from(&buf)?);
        }
        assert_eq!(len as usize, pos.len());

        let mut buf = [0u8; 2];
        let mut field = Vec::with_capacity(len as usize);
        let mut field2 = Vec::with_capacity(len as usize);
        for _ in 0..len {
            value.read_exact(&mut buf)?;
            field.push(u16::from_le_bytes(buf));
        }
        for _ in 0..len {
            value.read_exact(&mut buf)?;
            field2.push(u16::from_le_bytes(buf));
        }

        assert_eq!(len as usize, field.len());
        assert_eq!(len as usize, field2.len());

        let mut buf = [0u8; 1];
        let mut field3 = Vec::with_capacity(len as usize);
        for _ in 0..len {
            value.read_exact(&mut buf)?;
            field3.push(u8::from_le_bytes(buf));
        }
        assert_eq!(len as usize, field3.len());

        Ok(Self {
            len,
            indices,
            pos,
            field,
            field2,
            field3,
        })
    }
}

pub struct Other {
    len: u32,
    // possibly a vector3 instead vector2 for rotation/scale
    pos: Vec<Position>,
    field: Vec<u8>,
}

impl Other {
    fn from_reader<R: Read>(value: &mut R) -> Result<Self, MyError> {
        let mut buf = [0u8; 4];
        value.read_exact(&mut buf)?;

        let len = u32::from_le_bytes(buf);
        let mut pos = Vec::with_capacity(len as usize);

        for _ in 0..len {
            value.read_exact(&mut buf)?;
            pos.push(Position::try_from(&buf)?);
        }

        let mut field = Vec::with_capacity(len as usize);

        let mut buf = [0u8; 1];
        for _ in 0..len {
            value.read_exact(&mut buf)?;
            field.push(u8::from_le_bytes(buf));
        }

        Ok(Self { len, pos, field })
    }
}

pub struct Position {
    x: u16,
    y: u16,
}

impl TryFrom<&[u8; 4]> for Position {
    type Error = TryFromSliceError;

    fn try_from(value: &[u8; 4]) -> Result<Self, Self::Error> {
        Ok(Position {
            x: u16::from_le_bytes(value[..2].try_into()?),
            y: u16::from_le_bytes(value[2..].try_into()?),
        })
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
        let other = Other::from_reader(value);
        let Ok(other) = other else {
            println!("Couldn't read otherData");
            return Err(other.err().unwrap());
        };
        let other2 = Other::from_reader(value);
        let Ok(other2) = other2 else {
            println!("Couldn't read other2Data");
            return Err(other2.err().unwrap());
        };

        Ok(Distribution {
            slices,
            gatherables,
            other,
            other2,
        })
    }
}
