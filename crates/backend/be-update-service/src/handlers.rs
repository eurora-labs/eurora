use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json, Redirect, Response},
};
use tracing::{debug, instrument, warn};

use crate::{
    service::AppState,
    types::{
        DownloadParams, DownloadWithBundleTypeParams, ExtensionReleaseParams, ReleaseParams,
        UpdateParams, UpdateWithBundleTypeParams,
    },
};

#[instrument(skip(state), fields(
    channel = %params.channel,
    target_arch = %params.target_arch,
    current_version = %params.current_version
))]
pub async fn check_update_handler(
    State(state): State<Arc<AppState>>,
    Path(params): Path<UpdateParams>,
) -> Response {
    if params.current_version == "0.0.0" {
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
            (StatusCode::OK, Json(update)).into_response()
        }
        Ok(None) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => {
            warn!("Update check failed: {}", e);
            e.into_response()
        }
    }
}

#[instrument(skip(state), fields(channel = %params.channel))]
pub async fn get_release_handler(
    State(state): State<Arc<AppState>>,
    Path(params): Path<ReleaseParams>,
) -> Response {
    match state.get_latest_release(&params.channel).await {
        Ok(Some(release_info)) => {
            debug!(
                "Release info: version={}, platforms={}",
                release_info.version,
                release_info.platforms.len()
            );
            (StatusCode::OK, Json(release_info)).into_response()
        }
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            warn!("Release info request failed: {}", e);
            e.into_response()
        }
    }
}

/// Serves the correct artifact format based on the Tauri `{{bundle_type}}` variable
/// (e.g. .deb for deb installs, .AppImage for appimage installs).
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
    if params.current_version == "0.0.0" {
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
            (StatusCode::OK, Json(update)).into_response()
        }
        Ok(None) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => {
            warn!("Update check failed: {}", e);
            e.into_response()
        }
    }
}

#[instrument(skip(state), fields(channel = %params.channel))]
pub async fn get_extension_release_handler(
    State(state): State<Arc<AppState>>,
    Path(params): Path<ExtensionReleaseParams>,
) -> Response {
    match state.get_extension_release(&params.channel).await {
        Ok(Some(release_info)) => {
            debug!(
                "Extension release: channel={}, browsers={}",
                release_info.channel,
                release_info.browsers.len()
            );
            (StatusCode::OK, Json(release_info)).into_response()
        }
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            warn!("Extension release request failed: {}", e);
            e.into_response()
        }
    }
}

/// Redirects to a presigned S3 URL for the latest release artifact.
/// Used by website download buttons.
#[instrument(skip(state), fields(
    channel = %params.channel,
    target_arch = %params.target_arch
))]
pub async fn download_handler(
    State(state): State<Arc<AppState>>,
    Path(params): Path<DownloadParams>,
) -> Response {
    match state
        .get_download_url(&params.channel, &params.target_arch, None)
        .await
    {
        Ok(url) => {
            debug!("Redirecting download for {}", params.target_arch);
            Redirect::temporary(&url).into_response()
        }
        Err(e) => {
            warn!("Download failed: {}", e);
            e.into_response()
        }
    }
}

/// Redirects to a presigned S3 URL for a specific bundle type (e.g. deb, rpm, dmg).
#[instrument(skip(state), fields(
    channel = %params.channel,
    target_arch = %params.target_arch,
    bundle_type = %params.bundle_type
))]
pub async fn download_with_bundle_type_handler(
    State(state): State<Arc<AppState>>,
    Path(params): Path<DownloadWithBundleTypeParams>,
) -> Response {
    let bundle_type = if params.bundle_type.is_empty() || params.bundle_type == "unknown" {
        None
    } else {
        Some(params.bundle_type.as_str())
    };

    match state
        .get_download_url(&params.channel, &params.target_arch, bundle_type)
        .await
    {
        Ok(url) => {
            debug!(
                "Redirecting download for {} ({})",
                params.target_arch, params.bundle_type
            );
            Redirect::temporary(&url).into_response()
        }
        Err(e) => {
            warn!("Download failed: {}", e);
            e.into_response()
        }
    }
}
