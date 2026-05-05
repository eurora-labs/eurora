//! macOS keychain integration for the bridge CA. Drives the per-user
//! login keychain via `/usr/bin/security` so the trust install needs no
//! sudo and never re-prompts on reruns.
//!
//! Trust state is keyed off the SHA-1 thumbprint of the on-disk CA's
//! DER body, not its common name: when [`super::ensure`] rotates the
//! chain it mints a fresh CA keypair under the same CN, so a CN-only
//! pre-check would leave the rotated CA untrusted and the prior one
//! orphaned in the keychain forever. Every successful install also
//! prunes prior CN-matching entries so the keychain converges on
//! "exactly one Eurora bridge CA, with the thumbprint of the file on
//! disk."

use std::env;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

use crate::office_addin::bridge_certs::{CA_COMMON_NAME, TrustOutcome, ca_thumbprint, run_quiet};

pub fn install(ca_path: &Path) -> TrustOutcome {
    let keychain = match resolve_login_keychain() {
        Ok(k) => k,
        Err(reason) => return TrustOutcome::Failed(reason),
    };
    let existing = match list_thumbprints_for_cn(&keychain, CA_COMMON_NAME) {
        Ok(list) => list,
        Err(reason) => {
            return TrustOutcome::Failed(format!("listing existing bridge CAs: {reason}"));
        }
    };
    converge_install(&keychain, ca_path, &existing)
}

pub fn untrust() -> TrustOutcome {
    let keychain = match resolve_login_keychain() {
        Ok(k) => k,
        Err(reason) => return TrustOutcome::Failed(reason),
    };
    let existing = match list_thumbprints_for_cn(&keychain, CA_COMMON_NAME) {
        Ok(list) => list,
        Err(reason) => {
            return TrustOutcome::Failed(format!("listing existing bridge CAs: {reason}"));
        }
    };
    converge_untrust(&keychain, &existing)
}

fn converge_install(keychain: &Path, ca_path: &Path, existing: &[String]) -> TrustOutcome {
    let current = match ca_thumbprint(ca_path) {
        Ok(t) => t,
        Err(err) => {
            return TrustOutcome::Failed(format!(
                "computing thumbprint of {}: {err}",
                ca_path.display()
            ));
        }
    };

    let already_present = existing.iter().any(|t| t == &current);
    let stale: Vec<&str> = existing
        .iter()
        .filter(|t| **t != current)
        .map(String::as_str)
        .collect();

    if already_present && stale.is_empty() {
        tracing::debug!(
            "Bridge CA {current} already trusted in {} with no stale rotations",
            keychain.display()
        );
        return TrustOutcome::NoChange;
    }

    if !already_present
        && let Err(reason) = add_trusted_cert(keychain, ca_path) {
            return TrustOutcome::Failed(reason);
        }

    let mut stale_removed = 0;
    for thumbprint in stale {
        match delete_thumbprint(keychain, thumbprint) {
            Ok(()) => stale_removed += 1,
            Err(reason) => tracing::warn!(
                "Failed to prune stale bridge CA {thumbprint} from {}: {reason}",
                keychain.display()
            ),
        }
    }

    tracing::info!(
        "security install succeeded for bridge CA {current} (stale_removed={stale_removed})",
    );
    TrustOutcome::Installed { stale_removed }
}

fn converge_untrust(keychain: &Path, existing: &[String]) -> TrustOutcome {
    if existing.is_empty() {
        tracing::debug!(
            "No bridge CAs to untrust in {}; nothing to do",
            keychain.display()
        );
        return TrustOutcome::NoChange;
    }

    let mut removed = 0;
    let mut last_failure: Option<String> = None;
    for thumbprint in existing {
        match delete_thumbprint(keychain, thumbprint) {
            Ok(()) => removed += 1,
            Err(reason) => last_failure = Some(reason),
        }
    }

    if removed == 0 {
        return TrustOutcome::Failed(
            last_failure.unwrap_or_else(|| "no certificates were removed".to_owned()),
        );
    }
    if let Some(reason) = last_failure {
        tracing::warn!(
            "Partial untrust of bridge CAs in {}; last error: {reason}",
            keychain.display()
        );
    }

    tracing::info!("security untrust succeeded for bridge CA (removed={removed})");
    TrustOutcome::Untrusted { removed }
}

fn resolve_login_keychain() -> Result<PathBuf, String> {
    match env::var_os("HOME") {
        Some(home) => Ok(PathBuf::from(home).join("Library/Keychains/login.keychain-db")),
        None => Err("HOME is not set".to_owned()),
    }
}

fn add_trusted_cert(keychain: &Path, ca_path: &Path) -> Result<(), String> {
    let args: [&OsStr; 4] = [
        "add-trusted-cert".as_ref(),
        "-k".as_ref(),
        keychain.as_os_str(),
        ca_path.as_os_str(),
    ];
    match run_quiet("security", &args) {
        Ok(out) if out.status.success() => Ok(()),
        Ok(out) => Err(format_security_failure("add-trusted-cert", &out)),
        Err(err) => Err(format!("add-trusted-cert invocation: {err}")),
    }
}

fn delete_thumbprint(keychain: &Path, thumbprint: &str) -> Result<(), String> {
    // `security delete-certificate -Z <hash>` accepts hex in either
    // case; we hand it lowercase because that's what `ca_thumbprint`
    // produces.
    let args: [&OsStr; 4] = [
        "delete-certificate".as_ref(),
        "-Z".as_ref(),
        thumbprint.as_ref(),
        keychain.as_os_str(),
    ];
    match run_quiet("security", &args) {
        Ok(out) if out.status.success() => Ok(()),
        Ok(out) => Err(format_security_failure("delete-certificate", &out)),
        Err(err) => Err(format!("delete-certificate invocation: {err}")),
    }
}

fn list_thumbprints_for_cn(keychain: &Path, cn: &str) -> Result<Vec<String>, String> {
    let args: [&OsStr; 5] = [
        "find-certificate".as_ref(),
        "-a".as_ref(),
        "-Z".as_ref(),
        "-c".as_ref(),
        cn.as_ref(),
    ];
    let mut full_args: Vec<&OsStr> = args.to_vec();
    let keychain_arg: OsString = keychain.as_os_str().to_owned();
    full_args.push(keychain_arg.as_os_str());

    match run_quiet("security", &full_args) {
        // `find-certificate` exits non-zero when the CN matches no
        // certs in the keychain — that's the expected fresh-install
        // case, not an error.
        Ok(out) if !out.status.success() && out.stdout.is_empty() => Ok(Vec::new()),
        Ok(out) if out.status.success() || !out.stdout.is_empty() => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            Ok(parse_sha1_hashes(&stdout))
        }
        Ok(out) => Err(format_security_failure("find-certificate", &out)),
        Err(err) => Err(format!("find-certificate invocation: {err}")),
    }
}

/// Pull every `SHA-1 hash: <hex>` line out of `security
/// find-certificate -Z` output and return the hashes lowercased
/// (matching the format [`ca_thumbprint`] uses for comparison).
///
/// The `find-certificate` output groups attributes per certificate;
/// when `-a` is passed alongside `-Z` the SHA-1 line appears once per
/// match. Anything that isn't a recognisable hex digest after the
/// `:` is skipped — `security` occasionally prints SHA-256 hashes on
/// adjacent lines using the same prefix style.
fn parse_sha1_hashes(stdout: &str) -> Vec<String> {
    const PREFIX: &str = "SHA-1 hash:";
    let mut out = Vec::new();
    for line in stdout.lines() {
        let trimmed = line.trim_start();
        let Some(rest) = trimmed.strip_prefix(PREFIX) else {
            continue;
        };
        let candidate = rest.trim();
        if candidate.len() == 40 && candidate.chars().all(|c| c.is_ascii_hexdigit()) {
            out.push(candidate.to_ascii_lowercase());
        }
    }
    out
}

fn format_security_failure(verb: &str, out: &std::process::Output) -> String {
    let stderr = String::from_utf8_lossy(&out.stderr).trim().to_owned();
    let stdout = String::from_utf8_lossy(&out.stdout).trim().to_owned();
    format!(
        "security {verb} exit={}: {}",
        out.status,
        if !stderr.is_empty() { stderr } else { stdout }
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_sha1_hashes_extracts_each_match() {
        let stdout = "\
keychain: \"/Users/alice/Library/Keychains/login.keychain-db\"
version: 256
class: 0x80001000
attributes:
    \"alis\"<blob>=\"Eurora Local Bridge CA\"
SHA-1 hash: 1234567890ABCDEF1234567890ABCDEF12345678
SHA-256 hash: 0000000000000000000000000000000000000000000000000000000000000000
keychain: \"/Users/alice/Library/Keychains/login.keychain-db\"
version: 256
class: 0x80001000
attributes:
    \"alis\"<blob>=\"Eurora Local Bridge CA\"
SHA-1 hash: deadbeefdeadbeefdeadbeefdeadbeefdeadbeef
";
        let parsed = parse_sha1_hashes(stdout);
        assert_eq!(
            parsed,
            vec![
                "1234567890abcdef1234567890abcdef12345678".to_owned(),
                "deadbeefdeadbeefdeadbeefdeadbeefdeadbeef".to_owned(),
            ]
        );
    }

    #[test]
    fn parse_sha1_hashes_returns_empty_when_no_matches() {
        let stdout = "no certificates found\n";
        assert!(parse_sha1_hashes(stdout).is_empty());
    }

    #[test]
    fn parse_sha1_hashes_skips_non_hex_and_wrong_length() {
        let stdout = "\
SHA-1 hash: not-hex-not-hex-not-hex-not-hex-not-hex-
SHA-1 hash: cafebabe
SHA-1 hash: 1234567890abcdef1234567890abcdef12345678
";
        assert_eq!(
            parse_sha1_hashes(stdout),
            vec!["1234567890abcdef1234567890abcdef12345678".to_owned()]
        );
    }

    #[test]
    fn parse_sha1_hashes_normalises_to_lowercase() {
        let stdout = "SHA-1 hash: ABCDEF1234567890ABCDEF1234567890ABCDEF12\n";
        assert_eq!(
            parse_sha1_hashes(stdout),
            vec!["abcdef1234567890abcdef1234567890abcdef12".to_owned()]
        );
    }
}
