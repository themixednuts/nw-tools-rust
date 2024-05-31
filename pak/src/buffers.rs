#[cfg(bench)]
use std::{
    collections::HashMap,
    io::{self, Read},
};

#[cfg(bench)]
#[derive(Default)]
pub struct BufferPool {
    buffers: HashMap<usize, Vec<u8>>,
}

#[cfg(bench)]
impl BufferPool {
    pub fn new() -> Self {
        Self {
            buffers: HashMap::new(),
        }
    }

    pub fn get<R: Read>(&mut self, mut reader: R, size: usize) -> io::Result<Vec<u8>> {
        if let Some(buffer) = self.buffers.get_mut(&size) {
            buffer.clear();
            buffer.resize(size, 0);
            reader.read_exact(buffer)?;
            Ok(buffer.clone())
        } else {
            let mut buffer = vec![0; size];
            reader.read_exact(&mut buffer)?;
            self.buffers.insert(size, buffer.clone());
            Ok(buffer)
        }
    }

    pub fn get_as<T>(&mut self, reader: &mut impl Read, size: usize) -> io::Result<T>
    where
        T: FromBeBytes,
    {
        let bytes = self.get(reader, size)?;
        Ok(T::from_be_bytes(&bytes))
    }
}

#[cfg(bench)]
pub trait FromBeBytes {
    fn from_be_bytes(bytes: &[u8]) -> Self;
}

#[cfg(bench)]
macro_rules! impl_from_be_bytes {
    ($t:ty) => {
        impl FromBeBytes for $t {
            fn from_be_bytes(bytes: &[u8]) -> Self {
                <$t>::from_be_bytes(bytes.try_into().unwrap())
            }
        }
    };
}

#[cfg(bench)]
impl_from_be_bytes!(u8);
#[cfg(bench)]
impl_from_be_bytes!(u16);
#[cfg(bench)]
impl_from_be_bytes!(u32);
#[cfg(bench)]
impl_from_be_bytes!(u64);
