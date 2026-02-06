//! Utility functions for parsing and validation

use anyhow::Result;
use semver::Version;
use tracing::{debug, instrument};

use crate::error::UpdateServiceError;

/// Parse target_arch into target and arch components
/// e.g., "linux-x86_64" -> ("linux", "x86_64")
/// Note: Tauri uses "darwin" for macOS, but our S3 structure uses "macos",
/// so we normalize darwin -> macos here.
#[instrument(fields(target_arch))]
pub fn parse_target_arch(target_arch: &str) -> Result<(String, String)> {
    debug!("Parsing target architecture: {}", target_arch);
    let parts: Vec<&str> = target_arch.split('-').collect();
    if parts.len() < 2 {
        return Err(anyhow::Error::from(UpdateServiceError::InvalidTargetArch(
            target_arch.to_string(),
        )));
    }

    let target = parts[0].to_string();
    let arch = parts[1..].join("-"); // Handle cases like "aarch64" or multi-part arch

    // Normalize darwin -> macos to match our S3 directory structure
    // Tauri uses "darwin" but our release script stores files under "macos"
    let target = if target == "darwin" {
        "macos".to_string()
    } else {
        target
    };

    Ok((target, arch))
}

/// Extract version from S3 object key for the new structure
/// Expected format: releases/{channel}/{version}/{target}/{arch}/filename
pub fn extract_version_from_key(
    key: &str,
    prefix: &str,
    target: &str,
    arch: &str,
) -> Option<String> {
    if let Some(remainder) = key.strip_prefix(prefix) {
        // Split by '/' to get path components
        let parts: Vec<&str> = remainder.split('/').collect();
        debug!("Key path components: {:?}", parts);

        if parts.len() >= 3 {
            let version_str = parts[0];
            let key_target = parts[1];
            let key_arch = parts[2];

            debug!(
                "Extracted components: version={}, target={}, arch={}",
                version_str, key_target, key_arch
            );

            // Check if this key is for our target platform
            if key_target == target && key_arch == arch {
                debug!("Target platform matches: {}/{}", target, arch);
                // Validate that this looks like a version
                if Version::parse(version_str).is_ok() {
                    debug!("Valid version extracted: {}", version_str);
                    return Some(version_str.to_string());
                } else {
                    debug!("Invalid version format: {}", version_str);
                }
            } else {
                debug!(
                    "Target platform mismatch: expected {}/{}, got {}/{}",
                    target, arch, key_target, key_arch
                );
            }
        } else {
            debug!(
                "Insufficient path components: {} (need at least 3)",
                parts.len()
            );
        }
    } else {
        debug!("Key doesn't match prefix: {} (prefix: {})", key, prefix);
    }
    None
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

        // darwin should be normalized to macos to match S3 directory structure
        assert_eq!(
            parse_target_arch("darwin-aarch64").unwrap(),
            ("macos".to_string(), "aarch64".to_string())
        );

        assert_eq!(
            parse_target_arch("darwin-x86_64").unwrap(),
            ("macos".to_string(), "x86_64".to_string())
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
