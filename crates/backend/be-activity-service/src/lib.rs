//! HTTP activity service.
//!
//! Exposes a small Axum router under `/activities` that the desktop app
//! talks to via JSON. Authentication and Casbin authorization are applied
//! by the surrounding `be-authz` middleware in `be-monolith`; this crate
//! only assumes that a verified [`be_auth_core::Claims`] has been inserted
//! into request extensions by the time a handler runs.

pub mod analytics;
mod auth;
mod error;
mod handlers;
mod service;

use std::sync::Arc;

use anyhow::Result;
use axum::{Router, routing::get};
use be_asset::AssetService;
use be_remote_db::DatabaseManager;
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;

pub use error::{ActivityResult, ActivityServiceError};
pub use service::AppState;

/// Build the activity router with the supplied dependencies.
///
/// Returns the bare router; the caller is expected to apply the cross-cutting
/// layers (CORS, body limit, auth middleware) at the monolith level so all
/// REST services share the same outer pipeline.
pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route(
            "/activities",
            get(handlers::list_activities).post(handlers::insert_activity),
        )
        .layer(ServiceBuilder::new().layer(TraceLayer::new_for_http()))
        .with_state(state)
}

/// Convenience constructor mirroring `be-payment-service::init_payment_service`
/// and `be-update-service::init_update_service`. Wires up application state
/// and returns the router ready to merge into the monolith HTTP pipeline.
pub fn init_activity_service(
    db: Arc<DatabaseManager>,
    asset_service: Arc<AssetService>,
) -> Result<Router> {
    tracing::debug!("Initializing activity service");
    let state = Arc::new(AppState::new(db, asset_service));
    Ok(create_router(state))
}
