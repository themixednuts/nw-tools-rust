mod asset;
mod assetcatalog;
mod assetmanager;
mod assetregistry;
mod common;

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
