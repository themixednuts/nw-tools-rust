use uuid::Uuid;

use crate::common::{self, AssetId};

pub struct Asset {
    id: AssetId,
    name: String,
}

pub trait SerializeContext: Sized {
    type Value;
    fn serialize(&self) -> std::io::Result<Self::Value>;
    fn deserialize(value: &Self::Value) -> std::io::Result<Self>;
}
