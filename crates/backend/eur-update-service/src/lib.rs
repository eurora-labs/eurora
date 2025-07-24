use anyhow::{Context, Result};
use aws_config::BehaviorVersion;
use aws_sdk_s3::Client as S3Client;
use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::get,
};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::{error, info, warn};

/// Application state containing the S3 client and configuration
#[derive(Clone)]
pub struct AppState {
    s3_client: S3Client,
    bucket_name: String,
}

/// Tauri updater response format
#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateResponse {
    pub version: String,
    pub notes: String,
    pub pub_date: String,
    pub platforms: Platforms,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Platforms {
    #[serde(rename = "linux-x86_64")]
    pub linux_x86_64: Option<PlatformInfo>,
    #[serde(rename = "darwin-x86_64")]
    pub darwin_x86_64: Option<PlatformInfo>,
    #[serde(rename = "darwin-aarch64")]
    pub darwin_aarch64: Option<PlatformInfo>,
    #[serde(rename = "windows-x86_64")]
    pub windows_x86_64: Option<PlatformInfo>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PlatformInfo {
    pub signature: String,
    pub url: String,
}

/// Error response for when no update is available
#[derive(Serialize)]
pub struct NoUpdateResponse {
    pub message: String,
}

/// Path parameters for the update endpoint
#[derive(Deserialize)]
pub struct UpdateParams {
    pub channel: String,     // "nightly" or "release"
    pub target_arch: String, // e.g., "linux-x86_64", "darwin-aarch64"
    pub current_version: String,
}

impl AppState {
    /// Create a new AppState with S3 client
    pub async fn new(bucket_name: String) -> Result<Self> {
        let config = aws_config::defaults(BehaviorVersion::latest()).load().await;
        let s3_client = S3Client::new(&config);

        Ok(Self {
            s3_client,
            bucket_name,
        })
    }

    /// Check if a newer version exists in S3 for the given platform
    async fn check_for_update(
        &self,
        channel: &str,
        target_arch: &str,
        current_version: &str,
    ) -> Result<Option<UpdateResponse>> {
        // Parse current version
        let current_ver =
            Version::parse(current_version).context("Failed to parse current version")?;

        // List objects in the S3 bucket for this channel and platform
        let prefix = format!("{}/{}/", channel, target_arch);

        let resp = self
            .s3_client
            .list_objects_v2()
            .bucket(&self.bucket_name)
            .prefix(&prefix)
            .send()
            .await
            .context("Failed to list S3 objects")?;

        let mut latest_version: Option<Version> = None;
        let mut latest_key: Option<String> = None;

        // Find the latest version
        for object in resp.contents() {
            if let Some(key) = object.key() {
                // Extract version from key (assuming format: channel/target-arch/version/...)
                if let Some(version_str) = extract_version_from_key(key, &prefix) {
                    if let Ok(version) = Version::parse(&version_str) {
                        if version > current_ver
                            && (latest_version.is_none()
                                || version > *latest_version.as_ref().unwrap())
                        {
                            latest_version = Some(version);
                            latest_key = Some(key.to_string());
                        }
                    }
                }
            }
        }

        if let (Some(latest_ver), Some(_key)) = (latest_version, latest_key) {
            // Construct the update response
            let update_response = self
                .build_update_response(channel, target_arch, &latest_ver.to_string())
                .await?;
            Ok(Some(update_response))
        } else {
            Ok(None)
        }
    }

    /// Build the update response with platform-specific information
    async fn build_update_response(
        &self,
        channel: &str,
        target_arch: &str,
        version: &str,
    ) -> Result<UpdateResponse> {
        // Construct the base URL for the release files
        let base_url = format!(
            "https://{}.s3.amazonaws.com/{}/{}/{}",
            self.bucket_name, channel, target_arch, version
        );

        // Get signature file content
        let signature_key = format!("{}/{}/{}/signature", channel, target_arch, version);
        let signature = self
            .get_file_content(&signature_key)
            .await
            .unwrap_or_else(|_| "".to_string());

        // Determine the appropriate file extension and URL based on platform
        let (_file_extension, download_url) = match target_arch {
            arch if arch.starts_with("linux") => (
                "AppImage.tar.gz",
                format!("{}/bundle.AppImage.tar.gz", base_url),
            ),
            arch if arch.starts_with("darwin") => {
                ("app.tar.gz", format!("{}/bundle.app.tar.gz", base_url))
            }
            arch if arch.starts_with("windows") => {
                ("msi.zip", format!("{}/bundle.msi.zip", base_url))
            }
            _ => ("tar.gz", format!("{}/bundle.tar.gz", base_url)),
        };

        // Create platform info
        let platform_info = PlatformInfo {
            signature,
            url: download_url,
        };

        // Build platforms object with the appropriate platform set
        let mut platforms = Platforms {
            linux_x86_64: None,
            darwin_x86_64: None,
            darwin_aarch64: None,
            windows_x86_64: None,
        };

        match target_arch {
            "linux-x86_64" => platforms.linux_x86_64 = Some(platform_info),
            "darwin-x86_64" => platforms.darwin_x86_64 = Some(platform_info),
            "darwin-aarch64" => platforms.darwin_aarch64 = Some(platform_info),
            "windows-x86_64" => platforms.windows_x86_64 = Some(platform_info),
            _ => {
                warn!("Unknown target architecture: {}", target_arch);
                platforms.linux_x86_64 = Some(platform_info); // Default fallback
            }
        }

        // Try to get release notes
        let notes_key = format!("{}/{}/{}/notes.txt", channel, target_arch, version);
        let notes = self
            .get_file_content(&notes_key)
            .await
            .unwrap_or_else(|_| format!("Update to version {}", version));

        Ok(UpdateResponse {
            version: version.to_string(),
            notes,
            pub_date: chrono::Utc::now().to_rfc3339(),
            platforms,
        })
    }

    /// Get file content from S3
    async fn get_file_content(&self, key: &str) -> Result<String> {
        let resp = self
            .s3_client
            .get_object()
            .bucket(&self.bucket_name)
            .key(key)
            .send()
            .await
            .context("Failed to get object from S3")?;

        let body = resp
            .body
            .collect()
            .await
            .context("Failed to read object body")?;

        String::from_utf8(body.to_vec()).context("Failed to convert body to string")
    }
}

/// Extract version from S3 object key
fn extract_version_from_key(key: &str, prefix: &str) -> Option<String> {
    if let Some(remainder) = key.strip_prefix(prefix) {
        // Assuming format: version/filename
        if let Some(slash_pos) = remainder.find('/') {
            let version_str = &remainder[..slash_pos];
            // Validate that this looks like a version
            if Version::parse(version_str).is_ok() {
                return Some(version_str.to_string());
            }
        }
    }
    None
}

/// Handler for the update endpoint
pub async fn check_update_handler(
    State(state): State<Arc<AppState>>,
    Path(params): Path<UpdateParams>,
) -> Result<Json<UpdateResponse>, (StatusCode, Json<NoUpdateResponse>)> {
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
                Json(NoUpdateResponse {
                    message: "No update available".to_string(),
                }),
            ))
        }
        Err(e) => {
            error!("Error checking for update: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(NoUpdateResponse {
                    message: "Internal server error".to_string(),
                }),
            ))
        }
    }
}

/// Create the axum router
pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route(
            "/releases/{channel}/{target_arch}/{current_version}",
            get(check_update_handler),
        )
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CorsLayer::permissive()),
        )
        .with_state(state)
}

/// Initialize the update service and return the router
pub async fn init_update_service(bucket_name: String) -> Result<Router> {
    info!("Initializing update service with bucket: {}", bucket_name);

    // Create application state
    let state = Arc::new(
        AppState::new(bucket_name)
            .await
            .context("Failed to create application state")?,
    );

    // Create and return router
    Ok(create_router(state))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_version_from_key() {
        let prefix = "nightly/linux-x86_64/";

        assert_eq!(
            extract_version_from_key("nightly/linux-x86_64/1.0.0/bundle.AppImage.tar.gz", prefix),
            Some("1.0.0".to_string())
        );

        assert_eq!(
            extract_version_from_key("nightly/linux-x86_64/1.2.3-beta.1/signature", prefix),
            Some("1.2.3-beta.1".to_string())
        );

        assert_eq!(
            extract_version_from_key("nightly/linux-x86_64/invalid-version/file", prefix),
            None
        );
    }

    #[test]
    fn test_version_comparison() {
        let current = Version::parse("1.0.0").unwrap();
        let newer = Version::parse("1.0.1").unwrap();
        let older = Version::parse("0.9.9").unwrap();

        assert!(newer > current);
        assert!(older < current);
    }
}
