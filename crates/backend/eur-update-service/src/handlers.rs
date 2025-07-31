//! HTTP request handlers for the update service

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use std::sync::Arc;
use tracing::info;

use crate::error::{ErrorResponse, error_to_http_response};
use crate::service::AppState;
use crate::types::{UpdateParams, UpdateResponse};

/// Handler for the update endpoint
pub async fn check_update_handler(
    State(state): State<Arc<AppState>>,
    Path(params): Path<UpdateParams>,
) -> Result<Json<UpdateResponse>, (StatusCode, Json<ErrorResponse>)> {
    info!(
        "Checking for updates: channel={}, target_arch={}, current_version={}",
        params.channel, params.target_arch, params.current_version
    );

    match state
        .check_for_update(
            &params.channel,
            &params.target_arch,
            &params.current_version,
        )
        .await
    {
        Ok(Some(update)) => {
            info!("Update available: version {}", update.version);
            Ok(Json(update))
        }
        Ok(None) => {
            info!("No update available");
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
            let error_response = error_to_http_response(&e);
            Err(error_response)
        }
    }
}
