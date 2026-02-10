use crate::error::UpdateServiceError;

/// Parse target_arch into (target, arch) components.
/// e.g., "linux-x86_64" -> ("linux", "x86_64")
///
/// Normalizes "darwin" to "macos" to match our S3 directory structure
/// (Tauri uses "darwin" but our release script stores files under "macos").
pub fn parse_target_arch(target_arch: &str) -> Result<(String, String), UpdateServiceError> {
    let Some((target, arch)) = target_arch.split_once('-') else {
        return Err(UpdateServiceError::InvalidTargetArch(
            target_arch.to_owned(),
        ));
    };

    let target = if target == "darwin" {
        "macos".to_owned()
    } else {
        target.to_owned()
    };

    Ok((target, arch.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_target_arch() {
        assert_eq!(
            parse_target_arch("linux-x86_64").unwrap(),
            ("linux".to_owned(), "x86_64".to_owned())
        );
        assert_eq!(
            parse_target_arch("darwin-aarch64").unwrap(),
            ("macos".to_owned(), "aarch64".to_owned())
        );
        assert_eq!(
            parse_target_arch("darwin-x86_64").unwrap(),
            ("macos".to_owned(), "x86_64".to_owned())
        );
        assert_eq!(
            parse_target_arch("windows-x86_64").unwrap(),
            ("windows".to_owned(), "x86_64".to_owned())
        );
        assert!(parse_target_arch("invalid").is_err());
    }
}
