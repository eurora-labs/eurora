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
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::debug;

pub mod error;
pub mod handlers;
pub mod service;
pub mod types;
pub mod utils;

use service::AppState;

/// Create the axum router
pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        // Desktop app update routes
        .route("/releases/{channel}", get(handlers::get_release_handler))
        .route(
            "/releases/{channel}/{target_arch}/{current_version}",
            get(handlers::check_update_handler),
        )
        .route(
            "/releases/{channel}/{target_arch}/{current_version}/{bundle_type}",
            get(handlers::check_update_with_bundle_type_handler),
        )
        // Browser extension routes
        // GET /extensions/{channel} - returns all browsers for a channel
        .route(
            "/extensions/{channel}",
            get(handlers::get_extension_release_handler),
        )
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CorsLayer::permissive()),
        )
        .with_state(state)
}

/// Initialize the update service and return the router
pub async fn init_update_service(bucket_name: String) -> Result<Router> {
    debug!("Initializing update service with bucket: {}", bucket_name);

    // Create application state
    let state = Arc::new(
        AppState::new(bucket_name)
            .await
            .context("Failed to create application state")?,
    );

    // Create and return router
    Ok(create_router(state))
}

// Re-export commonly used types
pub use error::{ErrorResponse, NoUpdateResponse, UpdateServiceError};
pub use types::{
    BrowserExtensionInfo, BrowserType, ExtensionChannel, ExtensionReleaseParams,
    ExtensionReleaseResponse, PlatformInfo, ReleaseInfoResponse, ReleaseParams, UpdateParams,
    UpdateResponse, UpdateWithBundleTypeParams,
};

// Re-export handler for direct use
pub use handlers::get_extension_release_handler;
