use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Tauri updater response format
#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateResponse {
    pub version: String,
    pub pub_date: String,
    pub url: String,
    pub signature: String,
    pub notes: String,
}

#[derive(Deserialize, Debug)]
pub struct UpdateParams {
    pub channel: String,
    pub target_arch: String,
    pub current_version: String,
}

#[derive(Deserialize, Debug)]
pub struct UpdateWithBundleTypeParams {
    pub channel: String,
    pub target_arch: String,
    pub current_version: String,
    pub bundle_type: String,
}

#[derive(Deserialize, Debug)]
pub struct ReleaseParams {
    pub channel: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlatformInfo {
    pub url: String,
    pub signature: String,
}

/// Response for the `GET /releases/{channel}` endpoint.
/// Contains the latest version with all available platform downloads.
#[derive(Serialize, Deserialize, Debug)]
pub struct ReleaseInfoResponse {
    pub version: String,
    pub pub_date: String,
    pub platforms: BTreeMap<String, PlatformInfo>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BrowserType {
    Firefox,
    Chrome,
    Safari,
}

impl BrowserType {
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
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExtensionChannel {
    Release,
    Nightly,
}

impl ExtensionChannel {
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
        f.write_str(self.as_str())
    }
}

#[derive(Deserialize, Debug)]
pub struct ExtensionReleaseParams {
    pub channel: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BrowserExtensionInfo {
    pub version: String,
    pub url: String,
}

/// Response for the `GET /extensions/{channel}` endpoint.
/// Contains the latest extension versions for all browsers in a channel.
#[derive(Serialize, Deserialize, Debug)]
pub struct ExtensionReleaseResponse {
    pub channel: String,
    pub pub_date: String,
    pub browsers: BTreeMap<String, BrowserExtensionInfo>,
}
