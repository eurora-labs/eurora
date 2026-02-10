use std::{collections::BTreeMap, time::Duration};

use anyhow::{Context, Result};
use aws_config::BehaviorVersion;
use aws_sdk_s3::{Client as S3Client, presigning::PresigningConfig};
use chrono::{DateTime, Utc};
use semver::Version;
use tracing::{debug, instrument};

use crate::{
    error::UpdateServiceError,
    types::{
        BrowserExtensionInfo, BrowserType, ExtensionChannel, ExtensionReleaseResponse,
        PlatformInfo, ReleaseInfoResponse, UpdateResponse,
    },
    utils::parse_target_arch,
};

#[derive(Clone)]
pub struct AppState {
    s3_client: S3Client,
    bucket_name: String,
}

impl AppState {
    #[instrument(skip_all, fields(bucket = %bucket_name))]
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

    #[instrument(skip(self), fields(channel, target_arch, current_version, ?bundle_type))]
    pub async fn check_for_update(
        &self,
        channel: &str,
        target_arch: &str,
        current_version: &str,
        bundle_type: Option<&str>,
    ) -> Result<Option<UpdateResponse>, UpdateServiceError> {
        self.validate_inputs(channel, target_arch, current_version)?;

        let current_ver = Version::parse(current_version)
            .map_err(|_| UpdateServiceError::InvalidVersion(current_version.to_owned()))?;

        let (target, arch) = parse_target_arch(target_arch)?;

        // Use delimiter to efficiently list only version directories
        // Structure: releases/{channel}/{version}/...
        let prefix = format!("releases/{}/", channel);
        let mut candidates: Vec<(Version, String)> = Vec::new();

        let mut paginator = self
            .s3_client
            .list_objects_v2()
            .bucket(&self.bucket_name)
            .prefix(&prefix)
            .delimiter("/")
            .into_paginator()
            .send();

        while let Some(resp) = paginator.next().await {
            let resp = resp.map_err(|e| UpdateServiceError::S3Error(e.to_string()))?;

            for common_prefix in resp.common_prefixes() {
                let Some(prefix_str) = common_prefix.prefix() else {
                    continue;
                };
                let version_str = prefix_str
                    .strip_prefix(&prefix)
                    .and_then(|s| s.strip_suffix('/'))
                    .unwrap_or("");

                if let Ok(version) = Version::parse(version_str)
                    && version > current_ver
                {
                    candidates.push((version, version_str.to_owned()));
                }
            }
        }

        debug!(
            "Found {} candidate versions newer than {}",
            candidates.len(),
            current_version
        );

        // Sort descending (newest first)
        candidates.sort_unstable_by(|a, b| b.0.cmp(&a.0));

        // Find the latest version that has files for our target platform
        for (_, version_str) in &candidates {
            let target_prefix =
                format!("releases/{}/{}/{}/{}/", channel, version_str, target, arch);

            let resp = self
                .s3_client
                .list_objects_v2()
                .bucket(&self.bucket_name)
                .prefix(&target_prefix)
                .max_keys(1)
                .send()
                .await
                .map_err(|e| UpdateServiceError::S3Error(e.to_string()))?;

            if !resp.contents().is_empty() {
                debug!(
                    "Latest version with files for {}/{}: {}",
                    target, arch, version_str
                );
                let response = self
                    .build_update_response(channel, &target, &arch, version_str, bundle_type)
                    .await?;
                return Ok(Some(response));
            }
        }

        Ok(None)
    }

    #[instrument(skip(self), fields(channel, target, arch, version, ?bundle_type))]
    async fn build_update_response(
        &self,
        channel: &str,
        target: &str,
        arch: &str,
        version: &str,
        bundle_type: Option<&str>,
    ) -> Result<UpdateResponse, UpdateServiceError> {
        let directory_prefix = format!("releases/{}/{}/{}/{}/", channel, version, target, arch);

        let (file_key, last_modified) = self
            .find_download_file(&directory_prefix, target, bundle_type)
            .await?;

        let signature_key = format!("{}.sig", file_key);
        let signature = self
            .get_file_content(&signature_key)
            .await
            .map_err(|_| UpdateServiceError::SignatureNotFound(signature_key))?;

        let download_url = self.generate_presigned_url(&file_key).await?;

        let notes_key = format!(
            "releases/{}/{}/{}/{}/notes.txt",
            channel, version, target, arch
        );
        let notes = self
            .get_file_content(&notes_key)
            .await
            .unwrap_or_else(|_| format!("Update to version {}", version));

        Ok(UpdateResponse {
            version: strip_build_metadata(version),
            pub_date: last_modified.to_rfc3339(),
            url: download_url,
            signature,
            notes,
        })
    }

    async fn get_file_content(&self, key: &str) -> Result<String> {
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

        String::from_utf8(body.to_vec()).context("Failed to convert body to string")
    }

    /// Find download file candidates in the S3 directory, ordered by priority.
    ///
    /// Returns candidates in priority order based on bundle_type. For example,
    /// `bundle_type = "deb"` returns `.deb` files first, then `.AppImage` as fallback.
    /// Only files that have a corresponding `.sig` signature are included.
    #[instrument(skip(self), fields(directory_prefix, target, ?bundle_type))]
    async fn find_download_file(
        &self,
        directory_prefix: &str,
        target: &str,
        bundle_type: Option<&str>,
    ) -> Result<(String, DateTime<Utc>), UpdateServiceError> {
        let resp = self
            .s3_client
            .list_objects_v2()
            .bucket(&self.bucket_name)
            .prefix(directory_prefix)
            .send()
            .await
            .map_err(|e| UpdateServiceError::S3Error(e.to_string()))?;

        // With createUpdaterArtifacts: true, native bundles are served directly:
        //   - Linux AppImage: .AppImage (no tar.gz wrapper)
        //   - Linux deb/rpm: .deb / .rpm (fall back to .AppImage if not available)
        //   - Windows MSI: .msi (no zip wrapper)
        //   - macOS: .app.tar.gz
        let expected_extensions = match bundle_type {
            Some("deb") => vec![".deb", ".AppImage"],
            Some("rpm") => vec![".rpm", ".AppImage"],
            Some("appimage") => vec![".AppImage"],
            Some("msi") => vec![".msi"],
            Some("nsis") => vec![".exe"],
            Some("app") => vec![".app.tar.gz", ".tar.gz"],
            _ => match target {
                "linux" => vec![".AppImage"],
                "darwin" | "macos" => vec![".app.tar.gz", ".dmg", ".tar.gz"],
                "windows" => vec![".msi"],
                _ => vec![".tar.gz", ".zip"],
            },
        };

        let is_metadata = |filename: &str| filename.ends_with(".sig") || filename == "notes.txt";

        // Collect all S3 keys for quick `.sig` existence checks
        let all_keys: std::collections::HashSet<&str> =
            resp.contents().iter().filter_map(|o| o.key()).collect();

        // Find the first file matching expected extensions, respecting priority order.
        // Extensions are listed in priority order (e.g. [".deb", ".AppImage"] prefers .deb),
        // so we iterate extensions first to avoid S3's lexicographic ordering from
        // returning a lower-priority match (e.g. .AppImage before .deb).
        // Only return files that have a corresponding .sig signature file,
        // since Tauri's updater requires a valid signature.
        for ext in &expected_extensions {
            for object in resp.contents() {
                let Some(key) = object.key() else { continue };
                let filename = key.strip_prefix(directory_prefix).unwrap_or(key);

                if is_metadata(filename) {
                    continue;
                }

                if filename.ends_with(ext) {
                    let sig_key = format!("{}.sig", key);
                    if all_keys.contains(sig_key.as_str()) {
                        let last_modified = extract_last_modified(object)?;
                        return Ok((key.to_owned(), last_modified));
                    }
                    debug!("Skipping {} (no signature file {})", key, sig_key);
                }
            }
        }

        // Fallback: return the first non-metadata file that has a signature
        for object in resp.contents() {
            let Some(key) = object.key() else { continue };
            let filename = key.strip_prefix(directory_prefix).unwrap_or(key);

            if !is_metadata(filename) && !filename.is_empty() {
                let sig_key = format!("{}.sig", key);
                if all_keys.contains(sig_key.as_str()) {
                    let last_modified = extract_last_modified(object)?;
                    return Ok((key.to_owned(), last_modified));
                }
            }
        }

        Err(UpdateServiceError::DownloadFileNotFound(
            directory_prefix.to_owned(),
        ))
    }

    fn validate_inputs(
        &self,
        channel: &str,
        target_arch: &str,
        current_version: &str,
    ) -> Result<(), UpdateServiceError> {
        self.validate_channel(channel)?;

        if target_arch.is_empty() || !target_arch.contains('-') {
            return Err(UpdateServiceError::InvalidTargetArch(
                target_arch.to_owned(),
            ));
        }

        if Version::parse(current_version).is_err() {
            return Err(UpdateServiceError::InvalidVersion(
                current_version.to_owned(),
            ));
        }

        Ok(())
    }

    fn validate_channel(&self, channel: &str) -> Result<(), UpdateServiceError> {
        if !matches!(channel, "nightly" | "release" | "beta") {
            return Err(UpdateServiceError::InvalidChannel(channel.to_owned()));
        }
        Ok(())
    }

    #[instrument(skip(self), fields(channel))]
    pub async fn get_latest_release(
        &self,
        channel: &str,
    ) -> Result<Option<ReleaseInfoResponse>, UpdateServiceError> {
        self.validate_channel(channel)?;

        let prefix = format!("releases/{}/", channel);
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
            let resp = resp.map_err(|e| UpdateServiceError::S3Error(e.to_string()))?;

            for common_prefix in resp.common_prefixes() {
                let Some(prefix_str) = common_prefix.prefix() else {
                    continue;
                };
                let version_str = prefix_str
                    .strip_prefix(&prefix)
                    .and_then(|s| s.strip_suffix('/'))
                    .unwrap_or("");

                if let Ok(version) = Version::parse(version_str) {
                    all_versions.push((version, version_str.to_owned()));
                }
            }
        }

        if all_versions.is_empty() {
            return Ok(None);
        }

        all_versions.sort_unstable_by(|a, b| b.0.cmp(&a.0));

        let latest_version_str = &all_versions[0].1;
        debug!(
            "Latest version for channel {}: {}",
            channel, latest_version_str
        );

        let (platforms, pub_date) = self
            .get_platforms_for_version(channel, latest_version_str)
            .await?;

        if platforms.is_empty() {
            return Ok(None);
        }

        Ok(Some(ReleaseInfoResponse {
            version: strip_build_metadata(latest_version_str),
            pub_date: pub_date.to_rfc3339(),
            platforms,
        }))
    }

    /// Get all available platforms for a specific version.
    /// Returns the platforms map and the maximum last_modified date across all platforms.
    #[instrument(skip(self), fields(channel, version))]
    async fn get_platforms_for_version(
        &self,
        channel: &str,
        version: &str,
    ) -> Result<(BTreeMap<String, PlatformInfo>, DateTime<Utc>), UpdateServiceError> {
        let mut platforms: BTreeMap<String, PlatformInfo> = BTreeMap::new();
        let mut max_last_modified: Option<DateTime<Utc>> = None;

        let version_prefix = format!("releases/{}/{}/", channel, version);

        let targets = self.list_subdirectories(&version_prefix).await?;

        for target in &targets {
            let target_prefix = format!("{}{}/", version_prefix, target);
            let arches = self.list_subdirectories(&target_prefix).await?;

            for arch in &arches {
                let directory_prefix =
                    format!("releases/{}/{}/{}/{}/", channel, version, target, arch);

                let (file_key, last_modified) = match self
                    .find_download_file(&directory_prefix, target, None)
                    .await
                {
                    Ok(result) => result,
                    Err(e) => {
                        debug!("No download file found for {}/{}: {}", target, arch, e);
                        continue;
                    }
                };

                update_max(&mut max_last_modified, last_modified);

                let signature_key = format!("{}.sig", file_key);
                let signature = match self.get_file_content(&signature_key).await {
                    Ok(sig) => sig,
                    Err(e) => {
                        debug!("No signature for {}/{}: {}", target, arch, e);
                        continue;
                    }
                };

                let url = match self.generate_presigned_url(&file_key).await {
                    Ok(url) => url,
                    Err(e) => {
                        debug!("Failed to generate URL for {}/{}: {}", target, arch, e);
                        continue;
                    }
                };

                // Normalize macos -> darwin to match Tauri's platform naming
                let normalized_target = if target == "macos" { "darwin" } else { target };
                let platform_key = format!("{}-{}", normalized_target, arch);
                platforms.insert(platform_key, PlatformInfo { url, signature });
            }
        }

        let pub_date = max_last_modified.unwrap_or_else(Utc::now);
        Ok((platforms, pub_date))
    }

    /// List immediate subdirectory names under a given S3 prefix.
    async fn list_subdirectories(&self, prefix: &str) -> Result<Vec<String>, UpdateServiceError> {
        let mut dirs = Vec::new();

        let mut paginator = self
            .s3_client
            .list_objects_v2()
            .bucket(&self.bucket_name)
            .prefix(prefix)
            .delimiter("/")
            .into_paginator()
            .send();

        while let Some(resp) = paginator.next().await {
            let resp = resp.map_err(|e| UpdateServiceError::S3Error(e.to_string()))?;

            for common_prefix in resp.common_prefixes() {
                let Some(prefix_str) = common_prefix.prefix() else {
                    continue;
                };
                if let Some(name) = prefix_str
                    .strip_prefix(prefix)
                    .and_then(|s| s.strip_suffix('/'))
                {
                    dirs.push(name.to_owned());
                }
            }
        }

        Ok(dirs)
    }

    async fn generate_presigned_url(&self, file_key: &str) -> Result<String, UpdateServiceError> {
        let presigning_config = PresigningConfig::expires_in(Duration::from_secs(3600))
            .map_err(|e| UpdateServiceError::PresignedUrlError(e.to_string()))?;

        let presigned_request = self
            .s3_client
            .get_object()
            .bucket(&self.bucket_name)
            .key(file_key)
            .presigned(presigning_config)
            .await
            .map_err(|e| UpdateServiceError::PresignedUrlError(e.to_string()))?;

        Ok(presigned_request.uri().to_string())
    }

    fn validate_extension_channel(
        &self,
        channel: &str,
    ) -> Result<ExtensionChannel, UpdateServiceError> {
        channel
            .parse()
            .map_err(|_| UpdateServiceError::InvalidExtensionChannel(channel.to_owned()))
    }

    /// Get the latest extension versions for all browsers in a specific channel.
    ///
    /// S3 structure: extensions/{channel}/{browser}/{version}/
    #[instrument(skip(self), fields(channel))]
    pub async fn get_extension_release(
        &self,
        channel: &str,
    ) -> Result<Option<ExtensionReleaseResponse>, UpdateServiceError> {
        let extension_channel = self.validate_extension_channel(channel)?;

        let mut browsers: BTreeMap<String, BrowserExtensionInfo> = BTreeMap::new();
        let mut max_last_modified: Option<DateTime<Utc>> = None;

        for browser in [
            BrowserType::Firefox,
            BrowserType::Chrome,
            BrowserType::Safari,
        ] {
            match self
                .get_latest_browser_extension(browser, extension_channel)
                .await
            {
                Ok(Some((info, last_modified))) => {
                    update_max(&mut max_last_modified, last_modified);
                    browsers.insert(browser.to_string(), info);
                }
                Ok(None) => {
                    debug!("No extension for {} in channel '{}'", browser, channel);
                }
                Err(e) => {
                    debug!(
                        "Error getting extension for {} channel '{}': {}",
                        browser, channel, e
                    );
                }
            }
        }

        if browsers.is_empty() {
            return Ok(None);
        }

        Ok(Some(ExtensionReleaseResponse {
            channel: extension_channel.to_string(),
            pub_date: max_last_modified.unwrap_or_else(Utc::now).to_rfc3339(),
            browsers,
        }))
    }

    #[instrument(skip(self), fields(%browser, %channel))]
    async fn get_latest_browser_extension(
        &self,
        browser: BrowserType,
        channel: ExtensionChannel,
    ) -> Result<Option<(BrowserExtensionInfo, DateTime<Utc>)>, UpdateServiceError> {
        let prefix = format!("extensions/{}/{}/", channel.as_str(), browser.as_str());
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
            let resp = resp.map_err(|e| UpdateServiceError::S3Error(e.to_string()))?;

            for common_prefix in resp.common_prefixes() {
                let Some(prefix_str) = common_prefix.prefix() else {
                    continue;
                };
                let version_str = prefix_str
                    .strip_prefix(&prefix)
                    .and_then(|s| s.strip_suffix('/'))
                    .unwrap_or("");

                if let Ok(version) = Version::parse(version_str) {
                    all_versions.push((version, version_str.to_owned()));
                }
            }
        }

        if all_versions.is_empty() {
            return Ok(None);
        }

        all_versions.sort_unstable_by(|a, b| b.0.cmp(&a.0));

        let latest_version_str = &all_versions[0].1;
        debug!(
            "Latest extension: {}/{} v{}",
            channel, browser, latest_version_str
        );

        let version_prefix = format!(
            "extensions/{}/{}/{}/",
            channel.as_str(),
            browser.as_str(),
            latest_version_str
        );

        let (url, last_modified) = self
            .find_extension_file_and_url(&version_prefix, browser)
            .await?;

        Ok(Some((
            BrowserExtensionInfo {
                version: strip_build_metadata(latest_version_str),
                url,
            },
            last_modified,
        )))
    }

    #[instrument(skip(self), fields(directory_prefix, %browser))]
    async fn find_extension_file_and_url(
        &self,
        directory_prefix: &str,
        browser: BrowserType,
    ) -> Result<(String, DateTime<Utc>), UpdateServiceError> {
        let resp = self
            .s3_client
            .list_objects_v2()
            .bucket(&self.bucket_name)
            .prefix(directory_prefix)
            .send()
            .await
            .map_err(|e| UpdateServiceError::S3Error(e.to_string()))?;

        let expected_extensions = match browser {
            BrowserType::Firefox => &[".xpi", ".zip"] as &[&str],
            BrowserType::Chrome => &[".crx", ".zip"],
            BrowserType::Safari => &[".zip", ".safariextz"],
        };

        let is_metadata = |filename: &str| {
            filename.ends_with(".sig") || filename == "notes.txt" || filename == "manifest.json"
        };

        // Find the first file matching expected extensions
        for object in resp.contents() {
            let Some(key) = object.key() else { continue };
            let filename = key.strip_prefix(directory_prefix).unwrap_or(key);

            if is_metadata(filename) {
                continue;
            }

            if expected_extensions
                .iter()
                .any(|ext| filename.ends_with(ext))
            {
                let last_modified = extract_last_modified(object)?;
                let url = self.generate_presigned_url(key).await?;
                return Ok((url, last_modified));
            }
        }

        // Fallback: return the first non-metadata file
        for object in resp.contents() {
            let Some(key) = object.key() else { continue };
            let filename = key.strip_prefix(directory_prefix).unwrap_or(key);

            if !is_metadata(filename) && !filename.is_empty() {
                let last_modified = extract_last_modified(object)?;
                let url = self.generate_presigned_url(key).await?;
                return Ok((url, last_modified));
            }
        }

        Err(UpdateServiceError::DownloadFileNotFound(
            directory_prefix.to_owned(),
        ))
    }
}

fn extract_last_modified(
    object: &aws_sdk_s3::types::Object,
) -> Result<DateTime<Utc>, UpdateServiceError> {
    let smithy_dt = object.last_modified().ok_or_else(|| {
        UpdateServiceError::S3Error("Object missing last_modified timestamp".to_owned())
    })?;

    DateTime::from_timestamp(smithy_dt.secs(), smithy_dt.subsec_nanos())
        .ok_or_else(|| UpdateServiceError::S3Error("Invalid S3 timestamp".to_owned()))
}

fn update_max(current: &mut Option<DateTime<Utc>>, candidate: DateTime<Utc>) {
    match current {
        Some(max) if candidate > *max => *current = Some(candidate),
        None => *current = Some(candidate),
        _ => {}
    }
}

/// Strip semver pre-release and build metadata, keeping only major.minor.patch.
fn strip_build_metadata(version_str: &str) -> String {
    match Version::parse(version_str) {
        Ok(v) => format!("{}.{}.{}", v.major, v.minor, v.patch),
        Err(_) => version_str.to_owned(),
    }
}
