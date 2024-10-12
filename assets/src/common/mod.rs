use std::{array::TryFromSliceError, collections::HashMap, path::PathBuf};
use uuid::Uuid;

pub struct ProductDependancy {
    asset_id: AssetId,
    flags: u8,
}

#[derive(Eq, Hash, PartialEq, Clone, Copy, Debug)]
pub struct AssetId {
    pub guid: Uuid,
    pub sub_id: u32,
}

impl AssetId {}

impl TryFrom<&[u8]> for AssetId {
    type Error = TryFromSliceError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        Ok(Self {
            guid: Uuid::from_bytes(value[0..16].try_into()?),
            sub_id: u32::from_be_bytes(value[16..].try_into()?),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AssetInfo {
    pub asset_id: AssetId,
    pub asset_type: Uuid,
    pub relative_path: PathBuf,
    pub size_bytes: u32,
}

pub type AssetIdToInfo = HashMap<AssetId, AssetInfo>;
pub type AssetPathToId = HashMap<Uuid, AssetId>;
pub type LegacyAssetIdToRealAssetId = HashMap<AssetId, AssetId>;
pub type AssetDependancies = HashMap<AssetId, Vec<ProductDependancy>>;
