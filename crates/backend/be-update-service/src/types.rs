//! Data types and structures for the update service

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Tauri updater response format (dynamic server)
#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateResponse {
    pub version: String,
    pub pub_date: String,
    pub url: String,
    pub signature: String,
    pub notes: String,
}

/// Path parameters for the update endpoint
#[derive(Deserialize, Debug)]
pub struct UpdateParams {
    pub channel: String,     // "nightly" or "release"
    pub target_arch: String, // e.g., "linux-x86_64", "darwin-aarch64"
    pub current_version: String,
}

/// Path parameters for the update endpoint with bundle type
#[derive(Deserialize, Debug)]
pub struct UpdateWithBundleTypeParams {
    pub channel: String,     // "nightly" or "release"
    pub target_arch: String, // e.g., "linux-x86_64", "darwin-aarch64"
    pub current_version: String,
    pub bundle_type: String, // e.g., "appimage", "deb", "rpm", "msi", "nsis", "app"
}

/// Path parameters for the release info endpoint
#[derive(Deserialize, Debug)]
pub struct ReleaseParams {
    pub channel: String, // "nightly", "release", or "beta"
}

/// Platform-specific download information
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlatformInfo {
    pub url: String,
    pub signature: String,
}

/// Response for the releases/{channel} endpoint
/// Contains the latest version info with all available platforms
#[derive(Serialize, Deserialize, Debug)]
pub struct ReleaseInfoResponse {
    pub version: String,
    pub pub_date: String,
    /// Map of platform identifiers (e.g., "windows-x86_64", "linux-x86_64") to their download info
    /// Sorted alphabetically by platform name
    pub platforms: BTreeMap<String, PlatformInfo>,
}

// ============================================================================
// Browser Extension Types
// ============================================================================

/// Supported browser types for extensions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BrowserType {
    Firefox,
    Chrome,
    Safari,
}

impl BrowserType {
    /// Get the S3 directory name for this browser type
    pub fn as_str(&self) -> &'static str {
        match self {
            BrowserType::Firefox => "firefox",
            BrowserType::Chrome => "chrome",
            BrowserType::Safari => "safari",
        }
    }
}

impl std::str::FromStr for BrowserType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "firefox" => Ok(BrowserType::Firefox),
            "chrome" | "chromium" => Ok(BrowserType::Chrome),
            "safari" => Ok(BrowserType::Safari),
            _ => Err(format!("Unknown browser type: {}", s)),
        }
    }
}

impl std::fmt::Display for BrowserType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Release channel for browser extensions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExtensionChannel {
    Release,
    Nightly,
}

impl ExtensionChannel {
    /// Get the S3 directory name for this channel
    pub fn as_str(&self) -> &'static str {
        match self {
            ExtensionChannel::Release => "release",
            ExtensionChannel::Nightly => "nightly",
        }
    }
}

impl std::str::FromStr for ExtensionChannel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "release" => Ok(ExtensionChannel::Release),
            "nightly" => Ok(ExtensionChannel::Nightly),
            _ => Err(format!("Unknown extension channel: {}", s)),
        }
    }
}

impl std::fmt::Display for ExtensionChannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Path parameters for the extension release endpoint
#[derive(Deserialize, Debug)]
pub struct ExtensionReleaseParams {
    pub channel: String, // "release" or "nightly"
}

/// Browser-specific extension download information
/// Similar to PlatformInfo but for browser extensions
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BrowserExtensionInfo {
    /// Version string for this browser's extension
    pub version: String,
    /// Download URL for the extension package
    pub url: String,
}

/// Response for the /extensions/{channel} endpoint
/// Contains the latest extension versions for all browsers in a channel
/// Mirrors the structure of ReleaseInfoResponse for desktop releases
#[derive(Serialize, Deserialize, Debug)]
pub struct ExtensionReleaseResponse {
    /// The release channel (release, nightly)
    pub channel: String,
    /// ISO 8601 timestamp of the most recent publication across all browsers
    pub pub_date: String,
    /// Map of browser identifiers (firefox, chrome, safari) to their extension info
    /// Sorted alphabetically by browser name
    pub browsers: BTreeMap<String, BrowserExtensionInfo>,
}
