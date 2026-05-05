use std::sync::Arc;

use be_asset::AssetService;
use be_remote_db::DatabaseManager;

use crate::llm::Providers;

/// Shared state injected into Axum handlers via `State<Arc<AppState>>`.
pub struct AppState {
    pub db: Arc<DatabaseManager>,
    pub asset_service: Arc<AssetService>,
    pub providers: Providers,
}

impl AppState {
    pub fn new(db: Arc<DatabaseManager>, asset_service: Arc<AssetService>) -> Self {
        tracing::info!("Creating new ThreadService AppState");
        let providers = crate::llm::build_providers();
        Self {
            db,
            asset_service,
            providers,
        }
    }
}
