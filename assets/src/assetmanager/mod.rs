use crate::assetcatalog::AssetCatalog;
use std::{path::Path, sync::OnceLock};

pub static MANAGER: OnceLock<AssetManager> = OnceLock::new();

#[derive(Debug)]
pub struct AssetManager {
    m_catalog: &'static AssetCatalog,
    m_handlers: Vec<()>,
    m_assets: Vec<()>,
    m_assets_containers: Vec<()>,
}

impl AssetManager {
    async fn init() -> &'static Self {
        let manager = Self {
            m_catalog: AssetCatalog::init().await.unwrap(),
        };

        MANAGER.get_or_init(move || manager)
    }

    async fn load<P>(&self, path: P) -> std::io::Result<()>
    where
        P: AsRef<Path>,
    {
        // self.catalog.get_asset(path.as_ref()).unwrap();
        Ok(())
    }
}
