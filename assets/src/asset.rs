use std::io::Read;

pub enum EDataType {
    XML,
    JSON,
    BINARY,
}

pub trait SerializeContext {
    fn serialize<R: Read>(data: &mut R, _type: EDataType) -> usize;
}

pub struct IntAsset {}
