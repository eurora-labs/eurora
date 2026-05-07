use std::sync::Arc;

use be_asset::AssetService;
use be_remote_db::DatabaseManager;
use llm_core::LlmConfig;

use crate::llm::{BuildError, Providers};

/// Shared state injected into Axum handlers via `State<Arc<AppState>>`.
pub struct AppState {
    pub db: Arc<DatabaseManager>,
    pub asset_service: Arc<AssetService>,
    pub providers: Providers,
    pub llm_config: Arc<LlmConfig>,
}

impl AppState {
    /// Build [`AppState`] from a resolved [`LlmConfig`].
    ///
    /// The config is held in an [`Arc`] so handlers (e.g. the future
    /// `/llm/info` endpoint) can hand out a redacted view without copying
    /// the underlying provider map.
    pub fn try_new(
        db: Arc<DatabaseManager>,
        asset_service: Arc<AssetService>,
        llm_config: Arc<LlmConfig>,
    ) -> Result<Self, BuildError> {
        let providers = crate::llm::build_providers(&llm_config)?;
        Ok(Self {
            db,
            asset_service,
            providers,
            llm_config,
        })
    }
}
