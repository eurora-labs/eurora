use std::sync::Arc;

use be_asset::AssetService as CoreAssetService;

pub struct AppState {
    pub core: Arc<CoreAssetService>,
}

impl AppState {
    pub fn new(core: Arc<CoreAssetService>) -> Self {
        Self { core }
    }
}
