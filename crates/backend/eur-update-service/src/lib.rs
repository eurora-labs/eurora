use anyhow::{Context, Result};
use aws_config::BehaviorVersion;
use aws_sdk_s3::Client as S3Client;
use aws_sdk_s3::presigning::PresigningConfig;
use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::get,
};
use chrono::Utc;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::{error, info};

/// Application state containing the S3 client and configuration
#[derive(Clone)]
pub struct AppState {
    s3_client: S3Client,
    bucket_name: String,
}

/// Tauri updater response format (dynamic server)
#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateResponse {
    pub version: String,
    pub pub_date: String,
    pub url: String,
    pub signature: String,
    pub notes: String,
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
        let config = aws_config::defaults(BehaviorVersion::latest())
            .region("us-west-2")
            .load()
            .await;
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

        // List objects in the S3 bucket for this channel
        // New structure: releases/{channel}/{version}/{target}/{arch}/
        let prefix = format!("releases/{}/", channel);

        let resp = self
            .s3_client
            .list_objects_v2()
            .bucket(&self.bucket_name)
            .prefix(&prefix)
            .send()
            .await
            .context("Failed to list S3 objects")?;

        let mut latest_version: Option<Version> = None;
        let mut latest_version_str: Option<String> = None;

        // Parse target_arch to extract target and arch components
        let (target, arch) = parse_target_arch(target_arch)?;

        // Find the latest version that has files for our target platform
        for object in resp.contents() {
            if let Some(key) = object.key() {
                // Extract version from key (format: releases/channel/version/target/arch/...)
                if let Some(version_str) = extract_version_from_key(key, &prefix, &target, &arch) {
                    if let Ok(version) = Version::parse(&version_str) {
                        if version > current_ver
                            && (latest_version.is_none()
                                || version > *latest_version.as_ref().unwrap())
                        {
                            latest_version = Some(version);
                            latest_version_str = Some(version_str);
                        }
                    }
                }
            }
        }

        if let (Some(_latest_ver), Some(latest_ver_str)) = (latest_version, latest_version_str) {
            // Construct the update response
            let update_response = self
                .build_update_response(channel, target_arch, &target, &arch, &latest_ver_str)
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
        target: &str,
        arch: &str,
        version: &str,
    ) -> Result<UpdateResponse> {
        // Get signature file content
        let signature_key = format!(
            "releases/{}/{}/{}/{}/signature",
            channel, version, target, arch
        );
        let signature = self
            .get_file_content(&signature_key)
            .await
            .unwrap_or_else(|_| "".to_string());

        // Find the actual download file in the directory
        let directory_prefix = format!("releases/{}/{}/{}/{}/", channel, version, target, arch);
        let file_key = self.find_download_file(&directory_prefix, target).await?;

        // Generate presigned URL valid for 1 hour
        let presigning_config = PresigningConfig::expires_in(Duration::from_secs(60))?;
        let presigned_request = self
            .s3_client
            .get_object()
            .bucket(&self.bucket_name)
            .key(&file_key)
            .presigned(presigning_config)
            .await
            .context("Failed to generate presigned URL")?;

        let download_url = presigned_request.uri().to_string();

        // Try to get release notes
        let notes_key = format!(
            "releases/{}/{}/{}/{}/notes.txt",
            channel, version, target, arch
        );
        let notes = self
            .get_file_content(&notes_key)
            .await
            .unwrap_or_else(|_| format!("Update to version {}", version));

        Ok(UpdateResponse {
            version: version.to_string(),
            pub_date: Utc::now().to_rfc3339(),
            url: download_url,
            signature,
            notes,
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

    /// Find the actual download file in the S3 directory
    async fn find_download_file(&self, directory_prefix: &str, target: &str) -> Result<String> {
        let resp = self
            .s3_client
            .list_objects_v2()
            .bucket(&self.bucket_name)
            .prefix(directory_prefix)
            .send()
            .await
            .context("Failed to list files in release directory")?;

        // Define expected file extensions based on target platform
        let expected_extensions = match target {
            "linux" => vec![".AppImage.tar.gz", ".AppImage", ".tar.gz"],
            "darwin" => vec![".app.tar.gz", ".dmg", ".tar.gz"],
            "windows" => vec![".msi.zip", ".msi", ".exe", ".zip"],
            _ => vec![".tar.gz", ".zip"],
        };

        // Find the first file that matches expected extensions and is not "signature" or "notes.txt"
        for object in resp.contents() {
            if let Some(key) = object.key() {
                let filename = key.strip_prefix(directory_prefix).unwrap_or(key);

                // Skip signature and notes files
                if filename == "signature" || filename == "notes.txt" {
                    continue;
                }

                // Check if file matches expected extensions
                for ext in &expected_extensions {
                    if filename.ends_with(ext) {
                        return Ok(key.to_string());
                    }
                }
            }
        }

        // If no specific file found, return the first non-signature/notes file
        for object in resp.contents() {
            if let Some(key) = object.key() {
                let filename = key.strip_prefix(directory_prefix).unwrap_or(key);
                if filename != "signature" && filename != "notes.txt" && !filename.is_empty() {
                    return Ok(key.to_string());
                }
            }
        }

        Err(anyhow::anyhow!(
            "No download file found in directory: {}",
            directory_prefix
        ))
    }
}

/// Parse target_arch into target and arch components
/// e.g., "linux-x86_64" -> ("linux", "x86_64")
fn parse_target_arch(target_arch: &str) -> Result<(String, String)> {
    let parts: Vec<&str> = target_arch.split('-').collect();
    if parts.len() < 2 {
        return Err(anyhow::anyhow!(
            "Invalid target_arch format: {}",
            target_arch
        ));
    }

    let target = parts[0].to_string();
    let arch = parts[1..].join("-"); // Handle cases like "aarch64" or multi-part arch

    Ok((target, arch))
}

/// Extract version from S3 object key for the new structure
/// Expected format: releases/{channel}/{version}/{target}/{arch}/filename
fn extract_version_from_key(key: &str, prefix: &str, target: &str, arch: &str) -> Option<String> {
    if let Some(remainder) = key.strip_prefix(prefix) {
        // Split by '/' to get path components
        let parts: Vec<&str> = remainder.split('/').collect();

        if parts.len() >= 3 {
            let version_str = parts[0];
            let key_target = parts[1];
            let key_arch = parts[2];

            // Check if this key is for our target platform
            if key_target == target && key_arch == arch {
                // Validate that this looks like a version
                if Version::parse(version_str).is_ok() {
                    return Some(version_str.to_string());
                }
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
    fn test_parse_target_arch() {
        assert_eq!(
            parse_target_arch("linux-x86_64").unwrap(),
            ("linux".to_string(), "x86_64".to_string())
        );

        assert_eq!(
            parse_target_arch("darwin-aarch64").unwrap(),
            ("darwin".to_string(), "aarch64".to_string())
        );

        assert_eq!(
            parse_target_arch("windows-x86_64").unwrap(),
            ("windows".to_string(), "x86_64".to_string())
        );

        assert!(parse_target_arch("invalid").is_err());
    }

    #[test]
    fn test_extract_version_from_key() {
        let prefix = "releases/nightly/";
        let target = "linux";
        let arch = "x86_64";

        assert_eq!(
            extract_version_from_key(
                "releases/nightly/1.0.0/linux/x86_64/bundle.AppImage.tar.gz",
                prefix,
                target,
                arch
            ),
            Some("1.0.0".to_string())
        );

        assert_eq!(
            extract_version_from_key(
                "releases/nightly/1.2.3-beta.1/linux/x86_64/signature",
                prefix,
                target,
                arch
            ),
            Some("1.2.3-beta.1".to_string())
        );

        // Different target should return None
        assert_eq!(
            extract_version_from_key(
                "releases/nightly/1.0.0/darwin/x86_64/bundle.app.tar.gz",
                prefix,
                target,
                arch
            ),
            None
        );

        // Invalid version should return None
        assert_eq!(
            extract_version_from_key(
                "releases/nightly/invalid-version/linux/x86_64/file",
                prefix,
                target,
                arch
            ),
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
