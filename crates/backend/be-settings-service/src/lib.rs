//! HTTP settings service.
//!
//! Exposes a small Axum router under `/settings` that the desktop, mobile,
//! and web clients call to read and write their per-user cloud-synced
//! settings blob. Authentication and Casbin authorization are applied by
//! the surrounding `be-authz` middleware in `be-monolith`; this crate only
//! assumes that a verified [`be_auth_core::Claims`] has been inserted into
//! request extensions by the time a handler runs.
//!
//! ## Endpoints
//!
//! | Method | Path        | Outcome                                                  |
//! |--------|-------------|----------------------------------------------------------|
//! | GET    | `/settings` | `200 GetSettingsResponse` or `404` (first-run upload).   |
//! | PUT    | `/settings` | `200 PutSettingsAcceptedResponse` or `409 PutSettingsConflictResponse`. |
//! | DELETE | `/settings` | `204` (idempotent). Used by "reset cloud settings" UI.   |
//!
//! ## Server is blob-opaque
//!
//! The settings document is stored verbatim as `serde_json::Value` and is
//! never parsed by this crate. The server owns the indexed
//! `(user_id, schema_version, updated_at)` metadata; the body shape is
//! the client's contract. Optimistic concurrency lives in the database
//! layer (see [`be_remote_db::DatabaseManager::upsert_user_settings`]);
//! this crate is a thin translator that surfaces the three
//! [`be_remote_db::UpsertOutcome`] variants as appropriate HTTP responses.

mod error;
mod handlers;
mod response;

use std::sync::Arc;

use axum::{Router, routing::get};
use be_remote_db::DatabaseManager;
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;

pub use error::{SettingsErrorResponse, SettingsResult, SettingsServiceError};
pub use response::PutOutcomeResponse;

/// Shared state injected into Axum handlers via `State<Arc<AppState>>`.
pub struct AppState {
    pub db: Arc<DatabaseManager>,
}

impl AppState {
    pub fn new(db: Arc<DatabaseManager>) -> Self {
        Self { db }
    }
}

/// Build the settings router with the supplied dependencies.
///
/// Returns the bare router; the caller is expected to apply the
/// cross-cutting layers (CORS, body limit, auth middleware) at the
/// monolith level so all REST services share the same outer pipeline.
pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route(
            "/settings",
            get(handlers::get_settings)
                .put(handlers::put_settings)
                .delete(handlers::delete_settings),
        )
        .layer(ServiceBuilder::new().layer(TraceLayer::new_for_http()))
        .with_state(state)
}

/// Wire up application state and return the router ready to merge into
/// the monolith HTTP pipeline.
pub fn init_settings_service(db: Arc<DatabaseManager>) -> Router {
    tracing::debug!("Initializing settings service");
    create_router(Arc::new(AppState::new(db)))
}
