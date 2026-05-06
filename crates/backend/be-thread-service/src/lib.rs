//! HTTP + WebSocket thread service.
//!
//! Exposes an Axum router under `/threads` for CRUD, message-tree, and
//! search endpoints, plus a WebSocket upgrade at `/threads/{id}/chat` for
//! streaming chat. Authentication and Casbin authorization are applied by
//! the surrounding `be-authz` middleware in `be-monolith`; this crate only
//! assumes that a verified [`be_auth_core::Claims`] has been inserted into
//! request extensions by the time a handler runs.
//!
//! Token gating for the two cost-bearing endpoints (`POST /threads/{id}/title`
//! and the chat WebSocket) is also enforced by `be-authz` ahead of dispatch
//! — handlers in this crate trust that gating has already passed.

mod agent_loop;
mod conversion;
mod describe_image_tool;
mod error;
mod handlers;
mod llm;
mod message_projection;
mod preliminary;
mod service;
mod tools;

use std::sync::Arc;

use axum::Router;
use axum::routing::{get, post};
use be_asset::AssetService;
use be_remote_db::DatabaseManager;
use tower_http::trace::TraceLayer;

pub use error::{ThreadServiceError, ThreadServiceResult};
pub use service::AppState;

/// Build the thread router with the supplied dependencies.
///
/// Returns the bare router; the caller is expected to apply the cross-cutting
/// layers (CORS, body limit, auth middleware, token gating) at the monolith
/// level so all REST services share the same outer pipeline.
pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route(
            "/threads",
            post(handlers::threads::create_thread).get(handlers::threads::list_threads),
        )
        .route(
            "/threads/{thread_id}",
            get(handlers::threads::get_thread).delete(handlers::threads::delete_thread),
        )
        .route(
            "/threads/{thread_id}/title",
            post(handlers::threads::generate_thread_title),
        )
        .route(
            "/threads/{thread_id}/messages",
            get(handlers::messages::get_messages),
        )
        .route(
            "/threads/{thread_id}/messages/switch-branch",
            post(handlers::messages::switch_branch),
        )
        .route("/threads/{thread_id}/chat", get(handlers::chat::chat_ws))
        .route("/threads/search", get(handlers::search::search_threads))
        .route(
            "/threads/messages/search",
            get(handlers::search::search_messages),
        )
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

/// Wire up application state and return the router ready to merge into the
/// monolith HTTP pipeline.
pub fn init_thread_service(db: Arc<DatabaseManager>, asset_service: Arc<AssetService>) -> Router {
    tracing::debug!("Initializing thread service");
    create_router(Arc::new(AppState::new(db, asset_service)))
}
