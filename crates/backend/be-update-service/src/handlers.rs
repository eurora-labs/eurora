//! HTTP request handlers for the update service

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
};
use tracing::{debug, instrument, warn};

use crate::{
    error::error_to_http_response,
    service::AppState,
    types::{ExtensionReleaseParams, ReleaseParams, UpdateParams, UpdateWithBundleTypeParams},
};

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
            None,
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

/// Handler for the release info endpoint
/// Returns the latest version for a channel with all available platforms
#[instrument(skip(state), fields(channel = %params.channel))]
pub async fn get_release_handler(
    State(state): State<Arc<AppState>>,
    Path(params): Path<ReleaseParams>,
) -> Response {
    debug!(
        "Processing release info request: channel={}",
        params.channel
    );

    match state.get_latest_release(&params.channel).await {
        Ok(Some(release_info)) => {
            debug!(
                "Release info found: version={}, platforms={}",
                release_info.version,
                release_info.platforms.len()
            );
            (StatusCode::OK, Json(release_info)).into_response()
        }
        Ok(None) => {
            debug!("No release found for channel: {}", params.channel);
            StatusCode::NOT_FOUND.into_response()
        }
        Err(e) => {
            warn!("Release info request failed: {}", e);
            let (status, error_response) = error_to_http_response(&e);
            (status, error_response).into_response()
        }
    }
}

/// Handler for the update endpoint with bundle type
/// This allows serving the correct artifact format (e.g. .deb for deb installs,
/// .AppImage.tar.gz for appimage installs) based on the Tauri {{bundle_type}} variable.
#[instrument(skip(state), fields(
    channel = %params.channel,
    target_arch = %params.target_arch,
    current_version = %params.current_version,
    bundle_type = %params.bundle_type
))]
pub async fn check_update_with_bundle_type_handler(
    State(state): State<Arc<AppState>>,
    Path(params): Path<UpdateWithBundleTypeParams>,
) -> Response {
    debug!(
        "Processing update request: channel={}, target_arch={}, current_version={}, bundle_type={}",
        params.channel, params.target_arch, params.current_version, params.bundle_type
    );

    if &params.current_version == "0.0.0" {
        return StatusCode::NO_CONTENT.into_response();
    }

    let bundle_type = if params.bundle_type.is_empty() || params.bundle_type == "unknown" {
        None
    } else {
        Some(params.bundle_type.as_str())
    };

    match state
        .check_for_update(
            &params.channel,
            &params.target_arch,
            &params.current_version,
            bundle_type,
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
            StatusCode::NO_CONTENT.into_response()
        }
        Err(e) => {
            warn!("Update check failed: {}", e);
            let (status, error_response) = error_to_http_response(&e);
            (status, error_response).into_response()
        }
    }
}

// ============================================================================
// Browser Extension Handlers
// ============================================================================

/// Handler for getting extension releases for a specific channel
/// GET /extensions/{channel}
/// Returns the latest versions of all browser extensions for the specified channel
///
/// Example response:
/// ```json
/// {
///   "channel": "release",
///   "pub_date": "2026-01-31T10:00:00Z",
///   "browsers": {
///     "chrome": { "version": "1.2.3", "url": "https://..." },
///     "firefox": { "version": "1.2.3", "url": "https://..." },
///     "safari": { "version": "1.2.3", "url": "https://..." }
///   }
/// }
/// ```
#[instrument(skip(state), fields(channel = %params.channel))]
pub async fn get_extension_release_handler(
    State(state): State<Arc<AppState>>,
    Path(params): Path<ExtensionReleaseParams>,
) -> Response {
    debug!(
        "Processing extension release request: channel={}",
        params.channel
    );

    match state.get_extension_release(&params.channel).await {
        Ok(Some(release_info)) => {
            debug!(
                "Extension release found: channel={}, browsers={}",
                release_info.channel,
                release_info.browsers.len()
            );
            (StatusCode::OK, Json(release_info)).into_response()
        }
        Ok(None) => {
            debug!("No extensions found for channel: {}", params.channel);
            StatusCode::NOT_FOUND.into_response()
        }
        Err(e) => {
            warn!("Extension release request failed: {}", e);
            let (status, error_response) = error_to_http_response(&e);
            (status, error_response).into_response()
        }
    }
}
