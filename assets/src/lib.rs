pub mod assetcatalog;
use assetcatalog::AssetCatalog;
use tokio::io::{AsyncRead, AsyncSeek};

#[derive(Debug, Default)]
pub struct AssetManager {
    asset_catalog: AssetCatalog,
}

impl AssetManager {
    async fn new<R>(mut data: R) -> Self
    where
        R: AsyncRead + AsyncSeek + Unpin + Sync,
    {
        Self {
            asset_catalog: AssetCatalog::new(&mut data).await.unwrap(),
        }
    }
}

#[cfg(test)]
mod test {
    use std::io::Cursor;

    use super::*;
    use tokio;

    // #[tokio::test]
    // async fn test() {
    //     let catalog = include_bytes!("E:/Extract/NW Live/assetcatalog.catalog");
    //     let cursor = Cursor::new(catalog);

    //     let asset_manager = AssetManager::new(cursor).await;
    // }
}
