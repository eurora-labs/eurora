use std::sync::Arc;

use anyhow::Result;
use be_asset::AssetService;
use be_remote_db::DatabaseManager;

use crate::error::ActivityServiceError;

/// Shared state injected into Axum handlers via `State<Arc<AppState>>`.
pub struct AppState {
    pub db: Arc<DatabaseManager>,
    pub asset_service: Arc<AssetService>,
}

impl AppState {
    pub fn new(db: Arc<DatabaseManager>, asset_service: Arc<AssetService>) -> Self {
        tracing::info!("Creating new ActivityService AppState");
        Self { db, asset_service }
    }

    /// Convenience constructor mirroring the gRPC service it replaces.
    /// Reads storage configuration from the environment and wires up an
    /// internal [`AssetService`] from the same database handle.
    pub fn from_env(db: Arc<DatabaseManager>) -> Result<Self> {
        let asset_service =
            AssetService::from_env(db.clone()).map_err(ActivityServiceError::Asset)?;
        Ok(Self::new(db, Arc::new(asset_service)))
    }
}
