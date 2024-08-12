use std::{
    collections::HashMap,
    io,
    path::{Path, PathBuf},
    sync::OnceLock,
};

use file_system::{FileSystem, FILESYSTEM};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncSeek, AsyncSeekExt};
use uuid::{self, Uuid};
use uuid_simd::UuidExt;

use crate::common::{AssetId, AssetInfo};

static CATALOG: OnceLock<AssetCatalog> = OnceLock::new();

const SIGNATURE: &[u8; 4] = b"RASC";
const VERSION_OFFSET: u8 = 0x000004;
const SIZE_OFFSET: u8 = 0x000008;
const FIELD_4: u8 = 0x00000C;
const GUID_OFFSET: u8 = 0x000010;
const ASSET_TYPE_OFFSET: u8 = 0x000014;
const DIR_OFFSET: u8 = 0x000018;
const FILE_NAME_OFFSET: u8 = 0x00001C;
const SIZE_OFFSET_2: u8 = 0x000020;
const NUM_ASSET_ID_TO_INFO: u8 = 0x000024;
const ASSET_ID_TO_INFO_OFFSET: u8 = 0x000028;

#[derive(Debug, Default)]
pub struct AssetCatalog {
    version: u32,
    asset_infos: Vec<AssetInfo>,
    asset_id_index: HashMap<AssetId, usize>,
    relative_path_index: HashMap<PathBuf, usize>,
    // asset_path_to_id: HashMap<Uuid, AssetId>,
    // asset_dependencies: HashMap<AssetId, Vec<ProductDependancy>>,
}

impl AssetCatalog {
    pub async fn init() -> tokio::io::Result<&'static Self> {
        let fs = FILESYSTEM.get().unwrap();
        let mut data = fs.open("assetcatalog.catalog").await?;

        let mut buf = [0u8; 4];
        data.read_exact(&mut buf)
            .await
            .expect("Error reading asset catalog signature bytes");

        assert_eq!(
            &buf,
            SIGNATURE,
            "Incorrect signature bytes. {:?} does not match {:?}.",
            std::str::from_utf8(&buf),
            std::str::from_utf8(SIGNATURE)
        );

        let version = {
            data.read_exact(&mut buf).await?;
            u32::from_le_bytes(buf)
        };

        let size = {
            data.read_exact(&mut buf).await?;
            u32::from_le_bytes(buf)
        };

        // Field 4
        data.seek(tokio::io::SeekFrom::Current(4)).await?;

        let guid_offset = {
            data.read_exact(&mut buf).await?;
            u32::from_le_bytes(buf)
        };
        let asset_type_offset = {
            data.read_exact(&mut buf).await?;
            u32::from_le_bytes(buf)
        };
        let dir_data_offset = {
            data.read_exact(&mut buf).await?;
            u32::from_le_bytes(buf)
        };
        let file_name_data_offset = {
            data.read_exact(&mut buf).await?;
            u32::from_le_bytes(buf)
        };

        // size 2 assert
        assert_eq!(size, {
            data.read_exact(&mut buf).await?;
            u32::from_le_bytes(buf)
        });

        let asset_id_to_info_num = {
            data.read_exact(&mut buf).await?;
            u32::from_le_bytes(buf)
        };

        let mut asset_infos: Vec<AssetInfo> = Vec::with_capacity(asset_id_to_info_num as usize);
        let mut relative_path_index: HashMap<PathBuf, usize> =
            HashMap::with_capacity(asset_id_to_info_num as usize);
        let mut asset_id_index: HashMap<AssetId, usize> =
            HashMap::with_capacity(asset_id_to_info_num as usize);

        let asset_id_to_info_ref_size = std::mem::size_of::<AssetIdToInfoRef>() as u64;
        let mut asset_id_to_info_data = vec![0u8; asset_id_to_info_num as usize];
        data.read_exact(&mut asset_id_to_info_data).await?;

        let mut guid_data = vec![0u8; 16 * asset_id_to_info_num as usize];
        data.seek(tokio::io::SeekFrom::Start(guid_offset as u64))
            .await?;
        data.read_exact(&mut guid_data).await?;

        let guid_data = guid_data.chunks_exact(16).collect::<Vec<_>>();
        assert_eq!(asset_id_to_info_num as usize, guid_data.len());

        let mut asset_type_data = vec![0u8; 16 * asset_id_to_info_num as usize];
        data.seek(tokio::io::SeekFrom::Start(asset_type_offset as u64))
            .await?;
        data.read_exact(&mut asset_type_data).await?;

        let asset_tye_data = asset_type_data.chunks_exact(16).collect::<Vec<_>>();
        assert_eq!(asset_id_to_info_num as usize, asset_tye_data.len());

        let dir_data_size = file_name_data_offset - dir_data_offset;
        let mut dir_data = vec![0u8; dir_data_size as usize];
        data.seek(tokio::io::SeekFrom::Start(dir_data_offset as u64))
            .await?;
        data.read_exact(&mut dir_data).await?;

        let current = data
            .seek(tokio::io::SeekFrom::Start(file_name_data_offset as u64))
            .await?;
        let end = data.seek(tokio::io::SeekFrom::End(0)).await?;
        let file_name_data_size = end - current;
        let mut file_name_data = vec![0u8; file_name_data_size as usize];
        data.seek(tokio::io::SeekFrom::Start(file_name_data_offset as u64))
            .await?;
        data.read_exact(&mut file_name_data).await?;

        for (idx, id_to_info_ref) in asset_id_to_info_data
            .chunks_exact(asset_id_to_info_ref_size as usize)
            .enumerate()
        {
            let mut chunks = id_to_info_ref.chunks_exact(4);

            let guid_index =
                u32::from_le_bytes(chunks.next().unwrap().try_into().expect("msg")) as usize;
            let sub_id = u32::from_le_bytes(chunks.next().unwrap().try_into().expect("msg"));
            assert_eq!(
                guid_index,
                u32::from_le_bytes(chunks.next().unwrap().try_into().expect("msg")) as usize
            );

            assert_eq!(
                sub_id,
                u32::from_le_bytes(chunks.next().unwrap().try_into().expect("msg"))
            );
            let asset_type_index =
                u32::from_le_bytes(chunks.next().unwrap().try_into().expect("msg")) as usize;
            let _field_6 = u32::from_le_bytes(chunks.next().unwrap().try_into().expect("msg"));
            let size_bytes = u32::from_le_bytes(chunks.next().unwrap().try_into().expect("msg"));
            let _field_8 = u32::from_le_bytes(chunks.next().unwrap().try_into().expect("msg"));
            let dir_offset =
                u32::from_le_bytes(chunks.next().unwrap().try_into().expect("msg")) as usize;
            let file_name_offset =
                u32::from_le_bytes(chunks.next().unwrap().try_into().expect("msg")) as usize;

            let asset_id = AssetId {
                guid: Uuid::from_bytes(guid_data[guid_index].try_into().expect("msg")),
                sub_id,
            };

            let dir_null_byte_index = dir_data[dir_offset..]
                .iter()
                .position(|&byte| byte == 0)
                .map(|pos| pos + dir_offset)
                .unwrap_or(dir_data.len());

            let file_name_null_byte_index = file_name_data[file_name_offset..]
                .iter()
                .position(|&byte| byte == 0)
                .map(|pos| pos + file_name_offset)
                .unwrap_or(file_name_data.len());

            let path = PathBuf::from(
                String::from_utf8_lossy(&dir_data[dir_offset..dir_null_byte_index]).as_ref(),
            )
            .join(
                String::from_utf8_lossy(
                    &file_name_data[file_name_offset..file_name_null_byte_index],
                )
                .as_ref(),
            );

            let asset_info = AssetInfo {
                asset_id,
                asset_type: Uuid::from_bytes(
                    asset_tye_data[asset_type_index].try_into().expect("msg"),
                ),
                size_bytes,
                relative_path: path.to_owned(),
            };

            asset_infos.push(asset_info);
            asset_id_index.insert(asset_id, idx);
            relative_path_index.insert(path, idx);
        }

        Ok(CATALOG.get_or_init(move || Self {
            version,
            asset_infos,
            asset_id_index,
            relative_path_index,
        }))
    }

    pub fn get_asset_info_by_id<T>(&'static self, id: T) -> io::Result<&AssetInfo>
    where
        T: AsRef<AssetId>,
    {
        let idx = self
            .asset_id_index
            .get(id.as_ref())
            .ok_or(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Not found",
            ))?;

        let asset_info = self.asset_infos.get(*idx).ok_or(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "No AssetInfo found for path",
        ))?;

        Ok(asset_info)
    }

    pub fn get_asset_info_by_path<P>(&'static self, path: P) -> io::Result<&AssetInfo>
    where
        P: AsRef<Path>,
    {
        let idx = self
            .relative_path_index
            .get(path.as_ref())
            .ok_or(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Not found",
            ))?;

        let asset_info = self.asset_infos.get(*idx).ok_or(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "No AssetInfo found for path",
        ))?;

        Ok(asset_info)
    }
}

struct AssetIdToInfoRef {
    guid_index: u32,
    sub_id: u32,
    guid_index_2: u32,
    sub_id_2: u32,
    asset_type_index: u32,
    field_6: u32,
    size: u32,
    field_8: u32,
    dir_offset: u32,
    file_name_offset: u32,
}

struct AssetPathToIdRef {
    asset_type_index: u32,
    guid_index: u32,
    sub_id: u32,
}

struct LegacyAssetIdToRealAssetIdRef {
    legacy_guid_index: u32,
    legacy_sub_id: u32,
    real_guid_index: u32,
    real_sub_id: u32,
}

#[cfg(test)]
mod test {
    use super::*;
    use std::io::Cursor;
    use tokio;

    #[tokio::test]
    async fn test() {
        let catalog = include_bytes!("E:/Extract/NW Live/assetcatalog.catalog");
        let mut cursor = Cursor::new(catalog);

        let asset_catalog = AssetCatalog::init().await;
        assert!(asset_catalog.is_ok());
        // dbg!(asset_catalog.unwrap());
    }
}
