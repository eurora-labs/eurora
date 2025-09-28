//! HTTP request handlers for the update service

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
};
use tracing::{debug, info, instrument, warn};

use crate::{error::error_to_http_response, service::AppState, types::UpdateParams};

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
    debug!(
        "Processing update request: channel={}, target_arch={}, current_version={}",
        params.channel, params.target_arch, params.current_version
    );

    if &params.current_version == "0.0.0" {
        return StatusCode::NO_CONTENT.into_response();
    }

    match state
        .check_for_update(
            &params.channel,
            &params.target_arch,
            &params.current_version,
        )
        .await
    {
        Ok(Some(update)) => {
            debug!("Update available: version {}", update.version);
            debug!(
                "Update response: signature_length={}, notes_length={}, url_length={}",
                update.signature.len(),
                update.notes.len(),
                update.url.len()
            );
            (StatusCode::OK, Json(update)).into_response()
        }
        Ok(None) => {
            debug!("No update available");
            // Return 204 No Content with empty body as per RFC 7231
            // This is the correct way to indicate "no update available" to Tauri updater
            StatusCode::NO_CONTENT.into_response()
        }
        Err(e) => {
            warn!("Update check failed: {}", e);
            let (status, error_response) = error_to_http_response(&e);
            (status, error_response).into_response()
        }
    }
}
