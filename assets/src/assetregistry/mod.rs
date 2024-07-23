use crate::common::{AssetIdToInfo, AssetPathToId, LegacyAssetIdToRealAssetId};

pub struct AssetRegistry {
    asset_id_to_info: AssetIdToInfo,
    asset_path_to_id: AssetPathToId,
    legacy_asset_id_to_real_asset_id: LegacyAssetIdToRealAssetId,
}
