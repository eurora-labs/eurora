//! Windows root-store integration for the bridge CA. Drives the
//! per-user `Root` store via `certutil`, keying every operation off
//! the SHA-1 thumbprint of the on-disk CA's DER body.
//!
//! Every successful install also prunes any prior `Eurora Local
//! Bridge CA` entries left over from past rotations, so the user's
//! root store converges on "exactly one Eurora bridge CA, with the
//! thumbprint of the file on disk."

use std::ffi::OsStr;

use std::path::Path;

use crate::office_addin::bridge_certs::{CA_COMMON_NAME, TrustOutcome, ca_thumbprint, run_quiet};

pub fn install(ca_path: &Path) -> TrustOutcome {
    let existing = match list_thumbprints_for_cn(CA_COMMON_NAME) {
        Ok(list) => list,
        Err(reason) => {
            return TrustOutcome::Failed(format!("listing existing bridge CAs: {reason}"));
        }
    };
    converge_install(ca_path, &existing)
}

pub fn untrust() -> TrustOutcome {
    let existing = match list_thumbprints_for_cn(CA_COMMON_NAME) {
        Ok(list) => list,
        Err(reason) => {
            return TrustOutcome::Failed(format!("listing existing bridge CAs: {reason}"));
        }
    };
    converge_untrust(&existing)
}

fn converge_install(ca_path: &Path, existing: &[String]) -> TrustOutcome {
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
        tracing::debug!("Bridge CA {current} already trusted with no stale rotations");
        return TrustOutcome::NoChange;
    }

    if !already_present {
        if let Err(reason) = addstore(ca_path) {
            return TrustOutcome::Failed(reason);
        }
    }

    let mut stale_removed = 0;
    for thumbprint in stale {
        match delstore(thumbprint) {
            Ok(()) => stale_removed += 1,
            Err(reason) => {
                tracing::warn!("Failed to prune stale bridge CA {thumbprint}: {reason}")
            }
        }
    }

    tracing::info!(
        "certutil install succeeded for bridge CA {current} (stale_removed={stale_removed})"
    );
    TrustOutcome::Installed { stale_removed }
}

fn converge_untrust(existing: &[String]) -> TrustOutcome {
    if existing.is_empty() {
        tracing::debug!("No bridge CAs in user root store; nothing to untrust");
        return TrustOutcome::NoChange;
    }

    let mut removed = 0;
    let mut last_failure: Option<String> = None;
    for thumbprint in existing {
        match delstore(thumbprint) {
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
        tracing::warn!("Partial untrust of bridge CAs; last error: {reason}");
    }

    tracing::info!("certutil untrust succeeded for bridge CA (removed={removed})");
    TrustOutcome::Untrusted { removed }
}

fn addstore(ca_path: &Path) -> Result<(), String> {
    let args: [&OsStr; 4] = [
        "-user".as_ref(),
        "-addstore".as_ref(),
        "Root".as_ref(),
        ca_path.as_os_str(),
    ];
    match run_quiet("certutil", &args) {
        Ok(out) if out.status.success() => Ok(()),
        Ok(out) => Err(format_certutil_failure("-addstore", &out)),
        Err(err) => Err(format!("certutil -addstore invocation: {err}")),
    }
}

fn delstore(thumbprint: &str) -> Result<(), String> {
    let args: [&OsStr; 4] = [
        "-user".as_ref(),
        "-delstore".as_ref(),
        "Root".as_ref(),
        thumbprint.as_ref(),
    ];
    match run_quiet("certutil", &args) {
        Ok(out) if out.status.success() => Ok(()),
        Ok(out) => Err(format_certutil_failure("-delstore", &out)),
        Err(err) => Err(format!("certutil -delstore invocation: {err}")),
    }
}

fn list_thumbprints_for_cn(cn: &str) -> Result<Vec<String>, String> {
    let args: [&OsStr; 4] = [
        "-user".as_ref(),
        "-store".as_ref(),
        "Root".as_ref(),
        cn.as_ref(),
    ];
    match run_quiet("certutil", &args) {
        Ok(out) if out.status.success() => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            Ok(parse_cert_hashes(&stdout))
        }
        // `certutil -store Root <CN>` returns non-zero when no certs
        // match the CN. That's the expected fresh-install case.
        Ok(out) if !out.status.success() => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let parsed = parse_cert_hashes(&stdout);
            if parsed.is_empty() {
                Ok(Vec::new())
            } else {
                // Non-zero exit *and* parseable hashes: treat as a
                // partial / weird result and fall through to `Ok` so
                // we still try to converge.
                Ok(parsed)
            }
        }
        Ok(out) => Err(format_certutil_failure("-store", &out)),
        Err(err) => Err(format!("certutil -store invocation: {err}")),
    }
}

/// Pull every `Cert Hash(sha1): <hex>` (or the localized ASCII
/// `Cert Hash(SHA1):`) line out of `certutil -store` output and return
/// the hashes lowercased.
///
/// `certutil` formats SHA-1 hashes with optional whitespace separators
/// between byte pairs in some locales (`12 34 56 ...`). We strip
/// whitespace before validating the result is 40 lowercase hex chars.
fn parse_cert_hashes(stdout: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in stdout.lines() {
        let trimmed = line.trim_start();
        let Some(rest) = strip_cert_hash_prefix(trimmed) else {
            continue;
        };
        let compact: String = rest
            .trim()
            .chars()
            .filter(|c| !c.is_ascii_whitespace())
            .collect();
        if compact.len() == 40 && compact.chars().all(|c| c.is_ascii_hexdigit()) {
            out.push(compact.to_ascii_lowercase());
        }
    }
    out
}

fn strip_cert_hash_prefix(line: &str) -> Option<&str> {
    // Accept either case in the algorithm tag — `certutil` has
    // historically printed both `(sha1)` and `(SHA1)` depending on
    // version and locale.
    for prefix in ["Cert Hash(sha1):", "Cert Hash(SHA1):"] {
        if let Some(rest) = line.strip_prefix(prefix) {
            return Some(rest);
        }
    }
    None
}

fn format_certutil_failure(verb: &str, out: &std::process::Output) -> String {
    let stderr = String::from_utf8_lossy(&out.stderr).trim().to_owned();
    let stdout = String::from_utf8_lossy(&out.stdout).trim().to_owned();
    format!(
        "certutil {verb} exit={}: {}",
        out.status,
        if !stderr.is_empty() { stderr } else { stdout }
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_cert_hashes_extracts_each_match() {
        let stdout = "\
================ Certificate 0 ================
Serial Number: 01
Issuer: CN=Eurora Local Bridge CA
Subject: CN=Eurora Local Bridge CA
Cert Hash(sha1): 1234567890abcdef1234567890abcdef12345678
  Key Container = whatever

================ Certificate 1 ================
Serial Number: 02
Cert Hash(sha1): deadbeefdeadbeefdeadbeefdeadbeefdeadbeef

CertUtil: -store command completed successfully.
";
        let parsed = parse_cert_hashes(stdout);
        assert_eq!(
            parsed,
            vec![
                "1234567890abcdef1234567890abcdef12345678".to_owned(),
                "deadbeefdeadbeefdeadbeefdeadbeefdeadbeef".to_owned(),
            ]
        );
    }

    #[test]
    fn parse_cert_hashes_handles_uppercase_sha1_label_and_spaces() {
        let stdout = "\
Cert Hash(SHA1): 12 34 56 78 90 ab cd ef 12 34 56 78 90 ab cd ef 12 34 56 78
";
        assert_eq!(
            parse_cert_hashes(stdout),
            vec!["1234567890abcdef1234567890abcdef12345678".to_owned()]
        );
    }

    #[test]
    fn parse_cert_hashes_returns_empty_when_no_matches() {
        let stdout = "CertUtil: -store command FAILED: 0x80092004\n";
        assert!(parse_cert_hashes(stdout).is_empty());
    }

    #[test]
    fn parse_cert_hashes_skips_garbage() {
        let stdout = "\
Cert Hash(sha1): not-hex
Cert Hash(sha1): cafebabe
Cert Hash(sha1): 1234567890abcdef1234567890abcdef12345678
";
        assert_eq!(
            parse_cert_hashes(stdout),
            vec!["1234567890abcdef1234567890abcdef12345678".to_owned()]
        );
    }
}
