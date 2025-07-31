//! HTTP request handlers for the update service

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use std::sync::Arc;
use tracing::{debug, info, instrument, warn};

use crate::error::{ErrorResponse, error_to_http_response};
use crate::service::AppState;
use crate::types::{UpdateParams, UpdateResponse};

/// Handler for the update endpoint
#[instrument(skip(state), fields(
    channel = %params.channel,
    target_arch = %params.target_arch,
    current_version = %params.current_version
))]
pub async fn check_update_handler(
    State(state): State<Arc<AppState>>,
    Path(params): Path<UpdateParams>,
) -> Result<Json<UpdateResponse>, (StatusCode, Json<ErrorResponse>)> {
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
            Ok(Json(update))
        }
        Ok(None) => {
            let duration = start_time.elapsed();
            info!("No update available (processed in {:?})", duration);
            Err((
                StatusCode::NO_CONTENT,
                Json(ErrorResponse {
                    error: "no_update_available".to_string(),
                    message: "No update available".to_string(),
                    details: None,
                }),
            ))
        }
        Err(e) => {
            let duration = start_time.elapsed();
            warn!("Update check failed after {:?}: {}", duration, e);
            let error_response = error_to_http_response(&e);
            Err(error_response)
        }
    }
}
