//! Core update service logic and S3 operations

use anyhow::{Context, Result};
use aws_config::BehaviorVersion;
use aws_sdk_s3::Client as S3Client;
use aws_sdk_s3::presigning::PresigningConfig;
use chrono::Utc;
use semver::Version;
use std::time::Duration;
use tracing::{info, warn};

use crate::error::UpdateServiceError;
use crate::types::UpdateResponse;
use crate::utils::{extract_version_from_key, parse_target_arch};

/// Application state containing the S3 client and configuration
#[derive(Clone)]
pub struct AppState {
    s3_client: S3Client,
    bucket_name: String,
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
    pub async fn check_for_update(
        &self,
        channel: &str,
        target_arch: &str,
        current_version: &str,
    ) -> Result<Option<UpdateResponse>> {
        // Validate inputs
        self.validate_inputs(channel, target_arch, current_version)?;

        // Parse current version
        let current_ver = Version::parse(current_version).map_err(|_| {
            anyhow::Error::from(UpdateServiceError::InvalidVersion(
                current_version.to_string(),
            ))
        })?;

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
                            && latest_version.as_ref().map_or(true, |v| version > *v)
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
                .build_update_response(channel, &target, &arch, &latest_ver_str)
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
        target: &str,
        arch: &str,
        version: &str,
    ) -> Result<UpdateResponse> {
        // Get signature file content
        let signature_key = format!(
            "releases/{}/{}/{}/{}/signature",
            channel, version, target, arch
        );
        let signature = match self.get_file_content(&signature_key).await {
            Ok(sig) => sig,
            Err(e) => {
                warn!("Signature file not found at {}: {}", signature_key, e);
                // For security, we might want to fail here if signatures are mandatory
                // For now, we'll continue with empty signature but log the warning
                String::new()
            }
        };

        // Find the actual download file in the directory
        let directory_prefix = format!("releases/{}/{}/{}/{}/", channel, version, target, arch);
        let file_key = self.find_download_file(&directory_prefix, target).await?;

        // Generate presigned URL valid for 1 hour
        let presigning_config =
            PresigningConfig::expires_in(Duration::from_secs(3600)).map_err(|e| {
                anyhow::Error::from(UpdateServiceError::PresignedUrlError(e.to_string()))
            })?;
        let presigned_request = self
            .s3_client
            .get_object()
            .bucket(&self.bucket_name)
            .key(&file_key)
            .presigned(presigning_config)
            .await
            .map_err(|e| {
                anyhow::Error::from(UpdateServiceError::PresignedUrlError(e.to_string()))
            })?;

        let download_url = presigned_request.uri().to_string();

        // Try to get release notes
        let notes_key = format!(
            "releases/{}/{}/{}/{}/notes.txt",
            channel, version, target, arch
        );
        let notes = match self.get_file_content(&notes_key).await {
            Ok(notes) => notes,
            Err(e) => {
                info!("Release notes not found at {}: {}", notes_key, e);
                format!("Update to version {}", version)
            }
        };

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

        Err(anyhow::Error::from(
            UpdateServiceError::DownloadFileNotFound(directory_prefix.to_string()),
        ))
    }

    /// Validate input parameters
    fn validate_inputs(
        &self,
        channel: &str,
        target_arch: &str,
        current_version: &str,
    ) -> Result<()> {
        // Validate channel
        if !matches!(channel, "nightly" | "release" | "beta") {
            return Err(anyhow::Error::from(UpdateServiceError::InvalidChannel(
                channel.to_string(),
            )));
        }

        // Validate target_arch format
        if target_arch.is_empty() || !target_arch.contains('-') {
            return Err(anyhow::Error::from(UpdateServiceError::InvalidTargetArch(
                target_arch.to_string(),
            )));
        }

        // Validate current_version format
        if Version::parse(current_version).is_err() {
            return Err(anyhow::Error::from(UpdateServiceError::InvalidVersion(
                current_version.to_string(),
            )));
        }

        Ok(())
    }
}
