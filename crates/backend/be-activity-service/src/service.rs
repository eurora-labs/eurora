use std::sync::Arc;

use be_asset::AssetService;
use be_remote_db::DatabaseManager;

/// Shared state injected into Axum handlers via `State<Arc<AppState>>`.
pub struct AppState {
    pub db: Arc<DatabaseManager>,
    pub asset_service: Arc<AssetService>,
}

impl AppState {
    pub fn new(db: Arc<DatabaseManager>, asset_service: Arc<AssetService>) -> Self {
        Self { db, asset_service }
    }
}
