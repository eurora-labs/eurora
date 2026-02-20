use std::{collections::BTreeMap, time::Duration};

use anyhow::{Context, Result};
use aws_config::BehaviorVersion;
use aws_sdk_s3::{Client as S3Client, presigning::PresigningConfig, types::Object};
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
            .region("eu-central-1")
            .load()
            .await;
        let s3_client = S3Client::new(&config);

        Ok(Self {
            s3_client,
            bucket_name,
        })
    }

    async fn list_versions(
        &self,
        prefix: &str,
    ) -> Result<Vec<(Version, String)>, UpdateServiceError> {
        let mut versions: Vec<(Version, String)> = Vec::new();

        let mut paginator = self
            .s3_client
            .list_objects_v2()
            .bucket(&self.bucket_name)
            .prefix(prefix)
            .delimiter("/")
            .into_paginator()
            .send();

        while let Some(resp) = paginator.next().await {
            let resp = resp.map_err(|e| UpdateServiceError::S3Error(format!("{e:#}")))?;

            for common_prefix in resp.common_prefixes() {
                let Some(prefix_str) = common_prefix.prefix() else {
                    continue;
                };
                let version_str = prefix_str
                    .strip_prefix(prefix)
                    .and_then(|s| s.strip_suffix('/'))
                    .unwrap_or("");

                if let Ok(version) = Version::parse(version_str) {
                    versions.push((version, version_str.to_owned()));
                }
            }
        }

        versions.sort_unstable_by(|a, b| b.0.cmp(&a.0));
        Ok(versions)
    }

    /// List all objects under an S3 prefix, paginating to collect everything.
    async fn list_all_objects(&self, prefix: &str) -> Result<Vec<Object>, UpdateServiceError> {
        let mut objects = Vec::new();

        let mut paginator = self
            .s3_client
            .list_objects_v2()
            .bucket(&self.bucket_name)
            .prefix(prefix)
            .into_paginator()
            .send();

        while let Some(resp) = paginator.next().await {
            let resp = resp.map_err(|e| UpdateServiceError::S3Error(format!("{e:#}")))?;
            objects.extend(resp.contents().iter().cloned());
        }

        Ok(objects)
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
            let resp = resp.map_err(|e| UpdateServiceError::S3Error(format!("{e:#}")))?;

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

    fn validate_channel(&self, channel: &str) -> Result<(), UpdateServiceError> {
        if !matches!(channel, "nightly" | "release" | "beta") {
            return Err(UpdateServiceError::InvalidChannel(channel.to_owned()));
        }
        Ok(())
    }

    fn validate_extension_channel(
        &self,
        channel: &str,
    ) -> Result<ExtensionChannel, UpdateServiceError> {
        channel
            .parse()
            .map_err(|_| UpdateServiceError::InvalidExtensionChannel(channel.to_owned()))
    }

    #[instrument(skip(self), fields(channel, target_arch, current_version, ?bundle_type))]
    pub async fn check_for_update(
        &self,
        channel: &str,
        target_arch: &str,
        current_version: &str,
        bundle_type: Option<&str>,
    ) -> Result<Option<UpdateResponse>, UpdateServiceError> {
        self.validate_channel(channel)?;

        let current_ver = Version::parse(current_version)
            .map_err(|_| UpdateServiceError::InvalidVersion(current_version.to_owned()))?;

        let (target, arch) = parse_target_arch(target_arch)?;

        let prefix = format!("releases/{}/", channel);
        let all_versions = self.list_versions(&prefix).await?;

        let candidates: Vec<&str> = all_versions
            .iter()
            .filter(|(v, _)| v > &current_ver)
            .map(|(_, s)| s.as_str())
            .collect();

        debug!(
            "Found {} candidate versions newer than {}",
            candidates.len(),
            current_version
        );

        for version_str in &candidates {
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
                .map_err(|e| UpdateServiceError::S3Error(format!("{e:#}")))?;

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

        let (file_key, last_modified, signature) = self
            .find_signed_download_file(&directory_prefix, target, bundle_type)
            .await?;

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

    /// Find download file candidates in the S3 directory, ordered by priority.
    ///
    /// Returns the file key, last-modified timestamp, and the signature content.
    /// Only files that have a corresponding `.sig` signature are considered.
    #[instrument(skip(self), fields(directory_prefix, target, ?bundle_type))]
    async fn find_signed_download_file(
        &self,
        directory_prefix: &str,
        target: &str,
        bundle_type: Option<&str>,
    ) -> Result<(String, DateTime<Utc>, String), UpdateServiceError> {
        let objects = self.list_all_objects(directory_prefix).await?;

        let expected_extensions = match bundle_type {
            Some("deb") => vec![".deb", ".AppImage"],
            Some("rpm") => vec![".rpm", ".AppImage"],
            Some("appimage") => vec![".AppImage"],
            Some("msi") => vec![".msi"],
            Some("nsis") => vec![".exe"],
            Some("app") => vec![".app.tar.gz", ".tar.gz"],
            _ => match target {
                "linux" => vec![".AppImage"],
                "darwin" => vec![".app.tar.gz", ".dmg", ".tar.gz"],
                "windows" => vec![".msi"],
                _ => vec![".tar.gz", ".zip"],
            },
        };

        let is_metadata = |filename: &str| filename.ends_with(".sig") || filename == "notes.txt";

        let all_keys: std::collections::HashSet<&str> =
            objects.iter().filter_map(|o| o.key()).collect();

        for ext in &expected_extensions {
            for object in &objects {
                let Some(key) = object.key() else { continue };
                let filename = key.strip_prefix(directory_prefix).unwrap_or(key);

                if is_metadata(filename) {
                    continue;
                }

                if filename.ends_with(ext) {
                    let sig_key = format!("{}.sig", key);
                    if all_keys.contains(sig_key.as_str()) {
                        let last_modified = extract_last_modified(object)?;
                        let signature = self
                            .get_file_content(&sig_key)
                            .await
                            .map_err(|_| UpdateServiceError::SignatureNotFound(sig_key))?;
                        return Ok((key.to_owned(), last_modified, signature));
                    }
                    debug!("Skipping {} (no signature file {})", key, sig_key);
                }
            }
        }

        for object in &objects {
            let Some(key) = object.key() else { continue };
            let filename = key.strip_prefix(directory_prefix).unwrap_or(key);

            if !is_metadata(filename) && !filename.is_empty() {
                let sig_key = format!("{}.sig", key);
                if all_keys.contains(sig_key.as_str()) {
                    let last_modified = extract_last_modified(object)?;
                    let signature = self
                        .get_file_content(&sig_key)
                        .await
                        .map_err(|_| UpdateServiceError::SignatureNotFound(sig_key))?;
                    return Ok((key.to_owned(), last_modified, signature));
                }
            }
        }

        Err(UpdateServiceError::DownloadFileNotFound(
            directory_prefix.to_owned(),
        ))
    }

    #[instrument(skip(self), fields(channel, target_arch, ?bundle_type))]
    pub async fn get_download_url(
        &self,
        channel: &str,
        target_arch: &str,
        bundle_type: Option<&str>,
    ) -> Result<String, UpdateServiceError> {
        self.validate_channel(channel)?;

        let (target, arch) = parse_target_arch(target_arch)?;

        let prefix = format!("releases/{}/", channel);
        let all_versions = self.list_versions(&prefix).await?;

        if all_versions.is_empty() {
            return Err(UpdateServiceError::DownloadFileNotFound(prefix));
        }

        for (_, version_str) in &all_versions {
            let directory_prefix =
                format!("releases/{}/{}/{}/{}/", channel, version_str, target, arch);

            match self
                .find_download_file_unsigned(&directory_prefix, &target, bundle_type)
                .await
            {
                Ok(file_key) => {
                    let url = self.generate_presigned_url(&file_key).await?;
                    return Ok(url);
                }
                Err(_) => continue,
            }
        }

        Err(UpdateServiceError::DownloadFileNotFound(format!(
            "releases/{}/{}/{}/",
            channel, target, arch
        )))
    }

    /// Find a download file without requiring a `.sig` signature file.
    /// Used for website downloads where Tauri signature verification is not needed.
    #[instrument(skip(self), fields(directory_prefix, target, ?bundle_type))]
    async fn find_download_file_unsigned(
        &self,
        directory_prefix: &str,
        target: &str,
        bundle_type: Option<&str>,
    ) -> Result<String, UpdateServiceError> {
        let objects = self.list_all_objects(directory_prefix).await?;

        let expected_extensions = match bundle_type {
            Some("deb") => vec![".deb"],
            Some("rpm") => vec![".rpm"],
            Some("appimage") => vec![".AppImage"],
            Some("msi") => vec![".msi"],
            Some("nsis") => vec![".exe"],
            Some("dmg") => vec![".dmg"],
            Some("app") => vec![".app.tar.gz", ".tar.gz"],
            _ => match target {
                "linux" => vec![".AppImage"],
                "darwin" => vec![".dmg", ".app.tar.gz", ".tar.gz"],
                "windows" => vec![".msi"],
                _ => vec![".tar.gz", ".zip"],
            },
        };

        let is_metadata = |filename: &str| filename.ends_with(".sig") || filename == "notes.txt";

        for ext in &expected_extensions {
            for object in &objects {
                let Some(key) = object.key() else { continue };
                let filename = key.strip_prefix(directory_prefix).unwrap_or(key);

                if is_metadata(filename) {
                    continue;
                }

                if filename.ends_with(ext) {
                    return Ok(key.to_owned());
                }
            }
        }

        for object in &objects {
            let Some(key) = object.key() else { continue };
            let filename = key.strip_prefix(directory_prefix).unwrap_or(key);

            if !is_metadata(filename) && !filename.is_empty() {
                return Ok(key.to_owned());
            }
        }

        Err(UpdateServiceError::DownloadFileNotFound(
            directory_prefix.to_owned(),
        ))
    }

    #[instrument(skip(self), fields(channel))]
    pub async fn get_latest_release(
        &self,
        channel: &str,
    ) -> Result<Option<ReleaseInfoResponse>, UpdateServiceError> {
        self.validate_channel(channel)?;

        let prefix = format!("releases/{}/", channel);
        let all_versions = self.list_versions(&prefix).await?;

        if all_versions.is_empty() {
            return Ok(None);
        }

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

                let (file_key, last_modified, signature) = match self
                    .find_signed_download_file(&directory_prefix, target, None)
                    .await
                {
                    Ok(result) => result,
                    Err(e) => {
                        debug!("No download file found for {}/{}: {}", target, arch, e);
                        continue;
                    }
                };

                update_max(&mut max_last_modified, last_modified);

                let url = match self.generate_presigned_url(&file_key).await {
                    Ok(url) => url,
                    Err(e) => {
                        debug!("Failed to generate URL for {}/{}: {}", target, arch, e);
                        continue;
                    }
                };

                let platform_key = format!("{}-{}", target, arch);
                platforms.insert(platform_key, PlatformInfo { url, signature });
            }
        }

        let pub_date = max_last_modified.unwrap_or_else(Utc::now);
        Ok((platforms, pub_date))
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
        let all_versions = self.list_versions(&prefix).await?;

        if all_versions.is_empty() {
            return Ok(None);
        }

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
        let objects = self.list_all_objects(directory_prefix).await?;

        let expected_extensions: &[&str] = match browser {
            BrowserType::Firefox => &[".xpi", ".zip"],
            BrowserType::Chrome => &[".crx", ".zip"],
            BrowserType::Safari => &[".zip", ".safariextz"],
        };

        let is_metadata = |filename: &str| {
            filename.ends_with(".sig") || filename == "notes.txt" || filename == "manifest.json"
        };

        for ext in expected_extensions {
            for object in &objects {
                let Some(key) = object.key() else { continue };
                let filename = key.strip_prefix(directory_prefix).unwrap_or(key);

                if is_metadata(filename) {
                    continue;
                }

                if filename.ends_with(ext) {
                    let last_modified = extract_last_modified(object)?;
                    let url = self.generate_presigned_url(key).await?;
                    return Ok((url, last_modified));
                }
            }
        }

        for object in &objects {
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

fn extract_last_modified(object: &Object) -> Result<DateTime<Utc>, UpdateServiceError> {
    let smithy_dt = object.last_modified().ok_or_else(|| {
        UpdateServiceError::S3Error("Object missing last_modified timestamp".to_owned())
    })?;

    let nanos = smithy_dt.subsec_nanos().clamp(0, 999_999_999);

    DateTime::from_timestamp(smithy_dt.secs(), nanos).ok_or_else(|| {
        UpdateServiceError::S3Error(format!(
            "Invalid S3 timestamp: secs={}, nanos={}",
            smithy_dt.secs(),
            nanos,
        ))
    })
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
