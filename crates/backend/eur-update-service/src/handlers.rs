//! HTTP request handlers for the update service

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
};
use std::sync::Arc;
use tracing::{debug, info, instrument, warn};

use crate::error::error_to_http_response;
use crate::service::AppState;
use crate::types::UpdateParams;

/// Handler for the update endpoint
#[instrument(skip(state), fields(
    channel = %params.channel,
    target_arch = %params.target_arch,
    current_version = %params.current_version
))]
pub async fn check_update_handler(
    State(state): State<Arc<AppState>>,
    Path(params): Path<UpdateParams>,
) -> Response {
    info!(
        "Processing update request: channel={}, target_arch={}, current_version={}",
        params.channel, params.target_arch, params.current_version
    );

    let start_time = std::time::Instant::now();

    match state
        .check_for_update(
            &params.channel,
            &params.target_arch,
            &params.current_version,
        )
        .await
    {
        Ok(Some(update)) => {
            let duration = start_time.elapsed();
            info!(
                "Update available: version {} (processed in {:?})",
                update.version, duration
            );
            debug!(
                "Update response: signature_length={}, notes_length={}, url_length={}",
                update.signature.len(),
                update.notes.len(),
                update.url.len()
            );
            (StatusCode::OK, Json(update)).into_response()
        }
        Ok(None) => {
            let duration = start_time.elapsed();
            info!("No update available (processed in {:?})", duration);
            // Return 204 No Content with empty body as per RFC 7231
            // This is the correct way to indicate "no update available" to Tauri updater
            StatusCode::NO_CONTENT.into_response()
        }
        Err(e) => {
            let duration = start_time.elapsed();
            warn!("Update check failed after {:?}: {}", duration, e);
            let (status, error_response) = error_to_http_response(&e);
            (status, error_response).into_response()
        }
    }
}
