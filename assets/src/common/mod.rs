use std::{collections::HashMap, path::PathBuf};
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
