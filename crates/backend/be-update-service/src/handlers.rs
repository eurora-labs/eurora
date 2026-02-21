use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json, Redirect, Response},
};

use crate::{
    analytics,
    service::AppState,
    types::{
        DownloadParams, DownloadWithBundleTypeParams, ExtensionReleaseParams, ReleaseParams,
        UpdateParams, UpdateWithBundleTypeParams,
    },
    utils::parse_target_arch,
};

#[tracing::instrument(skip(state), fields(
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

    let (target, arch) = match parse_target_arch(&params.target_arch) {
        Ok(ta) => ta,
        Err(e) => {
            analytics::track_update_check_failed(
                &params.channel,
                &params.target_arch,
                &params.current_version,
                e.error_kind(),
            );
            tracing::warn!("Update check failed: {}", e);
            return e.into_response();
        }
    };

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
            tracing::debug!("Update available: version {}", update.version);
            analytics::track_update_check(
                &params.channel,
                &target,
                &arch,
                &params.current_version,
                None,
                true,
                Some(&update.version),
            );
            (StatusCode::OK, Json(update)).into_response()
        }
        Ok(None) => {
            analytics::track_update_check(
                &params.channel,
                &target,
                &arch,
                &params.current_version,
                None,
                false,
                None,
            );
            StatusCode::NO_CONTENT.into_response()
        }
        Err(e) => {
            analytics::track_update_check_failed(
                &params.channel,
                &params.target_arch,
                &params.current_version,
                e.error_kind(),
            );
            tracing::warn!("Update check failed: {}", e);
            e.into_response()
        }
    }
}

#[tracing::instrument(skip(state), fields(channel = %params.channel))]
pub async fn get_release_handler(
    State(state): State<Arc<AppState>>,
    Path(params): Path<ReleaseParams>,
) -> Response {
    match state.get_latest_release(&params.channel).await {
        Ok(Some(release_info)) => {
            tracing::debug!(
                "Release info: version={}, platforms={}",
                release_info.version,
                release_info.platforms.len()
            );
            analytics::track_release_info_request(
                &params.channel,
                Some(&release_info.version),
                release_info.platforms.len(),
            );
            (StatusCode::OK, Json(release_info)).into_response()
        }
        Ok(None) => {
            analytics::track_release_info_request(&params.channel, None, 0);
            StatusCode::NOT_FOUND.into_response()
        }
        Err(e) => {
            analytics::track_release_info_request(&params.channel, None, 0);
            tracing::warn!("Release info request failed: {}", e);
            e.into_response()
        }
    }
}

/// Serves the correct artifact format based on the Tauri `{{bundle_type}}` variable
/// (e.g. .deb for deb installs, .AppImage for appimage installs).
#[tracing::instrument(skip(state), fields(
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

    let (target, arch) = match parse_target_arch(&params.target_arch) {
        Ok(ta) => ta,
        Err(e) => {
            analytics::track_update_check_failed(
                &params.channel,
                &params.target_arch,
                &params.current_version,
                e.error_kind(),
            );
            tracing::warn!("Update check failed: {}", e);
            return e.into_response();
        }
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
            tracing::debug!("Update available: version {}", update.version);
            analytics::track_update_check(
                &params.channel,
                &target,
                &arch,
                &params.current_version,
                bundle_type,
                true,
                Some(&update.version),
            );
            (StatusCode::OK, Json(update)).into_response()
        }
        Ok(None) => {
            analytics::track_update_check(
                &params.channel,
                &target,
                &arch,
                &params.current_version,
                bundle_type,
                false,
                None,
            );
            StatusCode::NO_CONTENT.into_response()
        }
        Err(e) => {
            analytics::track_update_check_failed(
                &params.channel,
                &params.target_arch,
                &params.current_version,
                e.error_kind(),
            );
            tracing::warn!("Update check failed: {}", e);
            e.into_response()
        }
    }
}

#[tracing::instrument(skip(state), fields(channel = %params.channel))]
pub async fn get_extension_release_handler(
    State(state): State<Arc<AppState>>,
    Path(params): Path<ExtensionReleaseParams>,
) -> Response {
    match state.get_extension_release(&params.channel).await {
        Ok(Some(release_info)) => {
            tracing::debug!(
                "Extension release: channel={}, browsers={}",
                release_info.channel,
                release_info.browsers.len()
            );
            let browsers: Vec<String> = release_info.browsers.keys().cloned().collect();
            analytics::track_extension_check(&params.channel, &browsers);
            (StatusCode::OK, Json(release_info)).into_response()
        }
        Ok(None) => {
            analytics::track_extension_check(&params.channel, &[]);
            StatusCode::NOT_FOUND.into_response()
        }
        Err(e) => {
            analytics::track_extension_check_failed(&params.channel, e.error_kind());
            tracing::warn!("Extension release request failed: {}", e);
            e.into_response()
        }
    }
}

/// Redirects to a presigned S3 URL for the latest release artifact.
/// Used by website download buttons.
#[tracing::instrument(skip(state), fields(
    channel = %params.channel,
    target_arch = %params.target_arch
))]
pub async fn download_handler(
    State(state): State<Arc<AppState>>,
    Path(params): Path<DownloadParams>,
) -> Response {
    let (target, arch) = match parse_target_arch(&params.target_arch) {
        Ok(ta) => ta,
        Err(e) => {
            analytics::track_download_failed(
                &params.channel,
                &params.target_arch,
                None,
                e.error_kind(),
            );
            tracing::warn!("Download failed: {}", e);
            return e.into_response();
        }
    };

    match state
        .get_download_url(&params.channel, &params.target_arch, None)
        .await
    {
        Ok(url) => {
            tracing::debug!("Redirecting download for {}", params.target_arch);
            analytics::track_download_redirect(&params.channel, &target, &arch, None);
            Redirect::temporary(&url).into_response()
        }
        Err(e) => {
            analytics::track_download_failed(
                &params.channel,
                &params.target_arch,
                None,
                e.error_kind(),
            );
            tracing::warn!("Download failed: {}", e);
            e.into_response()
        }
    }
}

/// Redirects to a presigned S3 URL for a specific bundle type (e.g. deb, rpm, dmg).
#[tracing::instrument(skip(state), fields(
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

    let (target, arch) = match parse_target_arch(&params.target_arch) {
        Ok(ta) => ta,
        Err(e) => {
            analytics::track_download_failed(
                &params.channel,
                &params.target_arch,
                bundle_type,
                e.error_kind(),
            );
            tracing::warn!("Download failed: {}", e);
            return e.into_response();
        }
    };

    match state
        .get_download_url(&params.channel, &params.target_arch, bundle_type)
        .await
    {
        Ok(url) => {
            tracing::debug!(
                "Redirecting download for {} ({})",
                params.target_arch,
                params.bundle_type
            );
            analytics::track_download_redirect(&params.channel, &target, &arch, bundle_type);
            Redirect::temporary(&url).into_response()
        }
        Err(e) => {
            analytics::track_download_failed(
                &params.channel,
                &params.target_arch,
                bundle_type,
                e.error_kind(),
            );
            tracing::warn!("Download failed: {}", e);
            e.into_response()
        }
    }
}
