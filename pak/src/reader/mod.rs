use std::io::{self, Read};

pub trait ReadExt: Read {
    fn read_u8(&mut self) -> io::Result<u8> {
        let mut buf = [0; 1];
        self.read_exact(&mut buf)?;
        Ok(buf[0])
    }

    fn read_u16(&mut self) -> io::Result<u16> {
        let mut buf = [0; 2];
        self.read_exact(&mut buf)?;
        Ok(u16::from_be_bytes(buf))
    }

    fn read_u32(&mut self) -> io::Result<u32> {
        let mut buf = [0; 4];
        self.read_exact(&mut buf)?;
        Ok(u32::from_be_bytes(buf))
    }

    fn read_u64(&mut self) -> io::Result<u64> {
        let mut buf = [0; 8];
        self.read_exact(&mut buf)?;
        Ok(u64::from_be_bytes(buf))
    }

    fn read_bytes(&mut self, count: usize) -> io::Result<Vec<u8>> {
        let mut buf = vec![0; count];
        self.read_exact(&mut buf)?;
        Ok(buf)
    }
}

impl<T: Read> ReadExt for T {}
