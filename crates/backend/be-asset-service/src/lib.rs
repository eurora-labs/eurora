pub mod error;
pub mod handlers;
pub mod service;

use std::sync::Arc;

use axum::{Router, extract::DefaultBodyLimit, routing::post};
use be_asset::AssetService as CoreAssetService;
use tower_http::trace::TraceLayer;

pub use error::{AssetServiceError, ErrorResponse};
pub use service::AppState;

/// Maximum request body size for asset uploads. Sized to comfortably hold a
/// base64-encoded 1 GiB binary plus surrounding JSON envelope overhead
/// (base64 inflation factor ≈ 4/3).
const MAX_ASSET_REQUEST_SIZE: usize = 1500 * 1024 * 1024;

pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/v1/assets", post(handlers::create_asset_handler))
        .layer(DefaultBodyLimit::max(MAX_ASSET_REQUEST_SIZE))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

pub fn init_asset_service(core: Arc<CoreAssetService>) -> Router {
    create_router(Arc::new(AppState::new(core)))
}
