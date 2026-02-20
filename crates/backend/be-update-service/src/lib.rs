//! Eurora Update Service
//!
//! A Tauri-compatible update service that serves application updates from AWS S3.
//! Supports multiple channels (nightly, release, beta) and cross-platform builds.
//!
//! Also provides browser extension version checking for Firefox, Chrome, and Safari
//! with support for release and nightly channels.

use std::sync::Arc;

use anyhow::{Context, Result};
use axum::{Router, routing::get};
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;
use tracing::debug;

pub mod analytics;
pub mod error;
pub mod handlers;
pub mod service;
pub mod types;
pub mod utils;

use service::AppState;

pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/releases/{channel}", get(handlers::get_release_handler))
        .route(
            "/releases/{channel}/{target_arch}/{current_version}",
            get(handlers::check_update_handler),
        )
        .route(
            "/releases/{channel}/{target_arch}/{current_version}/{bundle_type}",
            get(handlers::check_update_with_bundle_type_handler),
        )
        .route(
            "/download/{channel}/{target_arch}",
            get(handlers::download_handler),
        )
        .route(
            "/download/{channel}/{target_arch}/{bundle_type}",
            get(handlers::download_with_bundle_type_handler),
        )
        .route(
            "/extensions/{channel}",
            get(handlers::get_extension_release_handler),
        )
        .layer(ServiceBuilder::new().layer(TraceLayer::new_for_http()))
        .with_state(state)
}

pub async fn init_update_service(bucket_name: String) -> Result<Router> {
    debug!("Initializing update service with bucket: {}", bucket_name);

    let state = Arc::new(
        AppState::new(bucket_name)
            .await
            .context("Failed to create application state")?,
    );

    Ok(create_router(state))
}

pub use error::{ErrorResponse, UpdateServiceError};
pub use types::{
    BrowserExtensionInfo, BrowserType, DownloadParams, DownloadWithBundleTypeParams,
    ExtensionChannel, ExtensionReleaseParams, ExtensionReleaseResponse, PlatformInfo,
    ReleaseInfoResponse, ReleaseParams, UpdateParams, UpdateResponse, UpdateWithBundleTypeParams,
};
