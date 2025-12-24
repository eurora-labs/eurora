//! Core update service logic and S3 operations

use std::time::Duration;

use anyhow::{Context, Result};
use aws_config::BehaviorVersion;
use aws_sdk_s3::{Client as S3Client, presigning::PresigningConfig};
use chrono::Utc;
use semver::Version;
use tracing::{debug, error, instrument};

use crate::{error::UpdateServiceError, types::UpdateResponse, utils::parse_target_arch};

/// Application state containing the S3 client and configuration
#[derive(Clone)]
pub struct AppState {
    s3_client: S3Client,
    bucket_name: String,
}

impl AppState {
    /// Create a new AppState with S3 client
    #[instrument(skip_all, fields(bucket = %bucket_name))]
    pub async fn new(bucket_name: String) -> Result<Self> {
        debug!("Initializing S3 client for bucket: {}", bucket_name);

        let config = aws_config::defaults(BehaviorVersion::latest())
            .region("us-west-2")
            .load()
            .await;
        let s3_client = S3Client::new(&config);

        debug!("S3 client initialized successfully");
        Ok(Self {
            s3_client,
            bucket_name,
        })
    }

    /// Check if a newer version exists in S3 for the given platform
    #[instrument(skip(self), fields(bucket = %self.bucket_name, channel, target_arch, current_version))]
    pub async fn check_for_update(
        &self,
        channel: &str,
        target_arch: &str,
        current_version: &str,
    ) -> Result<Option<UpdateResponse>> {
        debug!(
            "Starting update check for {}/{}/{}",
            channel, target_arch, current_version
        );

        // Validate inputs
        self.validate_inputs(channel, target_arch, current_version)?;
        debug!("Input validation passed");

        // Parse current version
        let current_ver = Version::parse(current_version).map_err(|_| {
            error!("Failed to parse current version: {}", current_version);
            anyhow::Error::from(UpdateServiceError::InvalidVersion(
                current_version.to_string(),
            ))
        })?;
        debug!("Parsed current version: {}", current_ver);

        // Parse target_arch to extract target and arch components
        let (target, arch) = parse_target_arch(target_arch)?;
        debug!(
            "Parsed target architecture: target={}, arch={}",
            target, arch
        );

        // Use delimiter to efficiently list only version directories
        // Structure: releases/{channel}/{version}/...
        let prefix = format!("releases/{}/", channel);
        debug!(
            "Listing version directories with prefix: {} and delimiter: /",
            prefix
        );

        // Collect all versions from the channel using delimiter to get only version prefixes
        let mut all_versions: Vec<(Version, String)> = Vec::new();

        let mut paginator = self
            .s3_client
            .list_objects_v2()
            .bucket(&self.bucket_name)
            .prefix(&prefix)
            .delimiter("/")
            .into_paginator()
            .send();

        while let Some(resp) = paginator.next().await {
            let resp = resp.context("Failed to list S3 objects")?;

            // common_prefixes contains the version directories (e.g., "releases/nightly/1.0.0/")
            for common_prefix in resp.common_prefixes() {
                if let Some(prefix_str) = common_prefix.prefix() {
                    // Extract version from prefix (format: releases/channel/version/)
                    let version_str = prefix_str
                        .strip_prefix(&prefix)
                        .and_then(|s| s.strip_suffix('/'))
                        .unwrap_or("");

                    if let Ok(version) = Version::parse(version_str)
                        && version > current_ver
                    {
                        debug!("Found candidate version: {}", version_str);
                        all_versions.push((version, version_str.to_string()));
                    }
                }
            }
        }

        debug!(
            "Found {} candidate versions newer than {}",
            all_versions.len(),
            current_version
        );

        // Sort versions in descending order (newest first)
        all_versions.sort_by(|a, b| b.0.cmp(&a.0));

        // Find the latest version that has files for our target platform
        let mut latest_version: Option<Version> = None;
        let mut latest_version_str: Option<String> = None;

        for (version, version_str) in all_versions {
            // Check if this version has files for our target/arch
            let target_prefix =
                format!("releases/{}/{}/{}/{}/", channel, version_str, target, arch);

            let resp = self
                .s3_client
                .list_objects_v2()
                .bucket(&self.bucket_name)
                .prefix(&target_prefix)
                .max_keys(1) // We only need to know if any file exists
                .send()
                .await
                .context("Failed to check version availability")?;

            if !resp.contents().is_empty() {
                debug!("Version {} has files for {}/{}", version_str, target, arch);
                latest_version = Some(version);
                latest_version_str = Some(version_str);
                break; // Found the latest version with files for our platform
            } else {
                debug!(
                    "Version {} has no files for {}/{}, checking older versions",
                    version_str, target, arch
                );
            }
        }

        if let (Some(latest_ver), Some(latest_ver_str)) = (latest_version, latest_version_str) {
            debug!(
                "Latest version found: {} (current: {})",
                latest_ver, current_ver
            );
            // Construct the update response
            let update_response = self
                .build_update_response(channel, &target, &arch, &latest_ver_str)
                .await?;
            debug!("Update response built successfully");
            Ok(Some(update_response))
        } else {
            debug!(
                "No newer version found for {}/{}/{}",
                channel, target_arch, current_version
            );
            Ok(None)
        }
    }

    /// Build the update response with platform-specific information
    #[instrument(skip(self), fields(bucket = %self.bucket_name, channel, target, arch, version))]
    async fn build_update_response(
        &self,
        channel: &str,
        target: &str,
        arch: &str,
        version: &str,
    ) -> Result<UpdateResponse> {
        debug!(
            "Building update response for {}/{}/{}/{}",
            channel, version, target, arch
        );

        // Find the actual download file in the directory first
        let directory_prefix = format!("releases/{}/{}/{}/{}/", channel, version, target, arch);
        debug!(
            "Looking for download file in directory: {}",
            directory_prefix
        );
        let file_key = self.find_download_file(&directory_prefix, target).await?;
        debug!("Found download file: {}", file_key);

        // Get signature file content based on the actual release file name
        let signature_key = format!("{}.sig", file_key);
        let signature = match self.get_file_content(&signature_key).await {
            Ok(sig) => {
                debug!("Successfully retrieved signature from {}", signature_key);
                sig
            }
            Err(_) => {
                return Err(anyhow::Error::from(UpdateServiceError::SignatureNotFound(
                    signature_key,
                )));
            }
        };

        // Generate presigned URL valid for 1 hour
        debug!("Generating presigned URL for file: {}", file_key);
        let presigning_config =
            PresigningConfig::expires_in(Duration::from_secs(3600)).map_err(|e| {
                error!("Failed to create presigning config: {}", e);
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
                error!("Failed to generate presigned URL for {}: {}", file_key, e);
                anyhow::Error::from(UpdateServiceError::PresignedUrlError(e.to_string()))
            })?;

        let download_url = presigned_request.uri().to_string();
        debug!(
            "Generated presigned URL successfully (length: {})",
            download_url.len()
        );

        // Try to get release notes
        let notes_key = format!(
            "releases/{}/{}/{}/{}/notes.txt",
            channel, version, target, arch
        );
        let notes = match self.get_file_content(&notes_key).await {
            Ok(notes) => {
                debug!("Successfully retrieved release notes from {}", notes_key);
                notes
            }
            Err(e) => {
                debug!("Release notes not found at {}: {}", notes_key, e);
                format!("Update to version {}", version)
            }
        };

        let response = UpdateResponse {
            version: version.to_string(),
            pub_date: Utc::now().to_rfc3339(),
            url: download_url,
            signature,
            notes,
        };

        debug!("Update response built successfully for version {}", version);
        Ok(response)
    }

    /// Get file content from S3
    #[instrument(skip(self), fields(bucket = %self.bucket_name, key))]
    async fn get_file_content(&self, key: &str) -> Result<String> {
        debug!("Fetching file content from S3: {}", key);
        let resp = self
            .s3_client
            .get_object()
            .bucket(&self.bucket_name)
            .key(key)
            .send()
            .await
            .with_context(|| format!("Failed to get object from S3: {}", key))?;

        let body = resp
            .body
            .collect()
            .await
            .context("Failed to read object body")?;

        let content =
            String::from_utf8(body.to_vec()).context("Failed to convert body to string")?;

        debug!(
            "Successfully fetched file content (length: {})",
            content.len()
        );
        Ok(content)
    }

    /// Find the actual download file in the S3 directory
    #[instrument(skip(self), fields(bucket = %self.bucket_name, directory_prefix, target))]
    async fn find_download_file(&self, directory_prefix: &str, target: &str) -> Result<String> {
        debug!(
            "Searching for download file in directory: {}",
            directory_prefix
        );
        let resp = self
            .s3_client
            .list_objects_v2()
            .bucket(&self.bucket_name)
            .prefix(directory_prefix)
            .send()
            .await
            .with_context(|| {
                format!(
                    "Failed to list files in release directory: {}",
                    directory_prefix
                )
            })?;

        let file_count = resp.contents().len();
        debug!(
            "Found {} files in directory {}",
            file_count, directory_prefix
        );

        // Define expected file extensions based on target platform
        let expected_extensions = match target {
            "linux" => vec![".AppImage.tar.gz", ".tar.gz"],
            "darwin" => vec![".app.tar.gz", ".dmg", ".tar.gz"],
            "windows" => vec![".msi.zip"],
            _ => vec![".tar.gz", ".zip"],
        };
        debug!(
            "Expected extensions for {}: {:?}",
            target, expected_extensions
        );

        // Find the first file that matches expected extensions and is not "signature" or "notes.txt"
        for object in resp.contents() {
            if let Some(key) = object.key() {
                let filename = key.strip_prefix(directory_prefix).unwrap_or(key);
                debug!("Examining file: {}", filename);

                // Skip signature and notes files
                if filename.ends_with(".sig") || filename == "notes.txt" {
                    debug!("Skipping metadata file: {}", filename);
                    continue;
                }

                // Check if file matches expected extensions
                for ext in &expected_extensions {
                    if filename.ends_with(ext) {
                        debug!(
                            "Found matching download file: {} (extension: {})",
                            filename, ext
                        );
                        return Ok(key.to_string());
                    }
                }
                debug!("File {} doesn't match expected extensions", filename);
            }
        }

        // If no specific file found, return the first non-signature/notes file
        for object in resp.contents() {
            if let Some(key) = object.key() {
                let filename = key.strip_prefix(directory_prefix).unwrap_or(key);
                if !filename.ends_with(".sig") && filename != "notes.txt" && !filename.is_empty() {
                    return Ok(key.to_string());
                }
            }
        }

        Err(anyhow::Error::from(
            UpdateServiceError::DownloadFileNotFound(directory_prefix.to_string()),
        ))
    }

    /// Validate input parameters
    #[instrument(skip(self), fields(channel, target_arch, current_version))]
    fn validate_inputs(
        &self,
        channel: &str,
        target_arch: &str,
        current_version: &str,
    ) -> Result<()> {
        debug!("Validating input parameters");
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
