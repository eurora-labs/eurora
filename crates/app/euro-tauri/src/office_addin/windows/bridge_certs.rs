//! Windows root-store integration for the bridge CA. Drives the
//! per-user `Root` store via `certutil`, keyed by the SHA-1 thumbprint
//! over the CA's DER body so the install pre-check is a single,
//! deterministic store query.

use std::ffi::OsStr;
use std::fs;
use std::path::Path;

use sha1::{Digest, Sha1};

use crate::office_addin::bridge_certs::{
    TrustAction, TrustOutcome, action_label, read_first_certificate_der, run_quiet,
};

pub fn trust_impl(ca_path: &Path, action: TrustAction) -> TrustOutcome {
    let thumbprint = match read_ca_thumbprint(ca_path) {
        Ok(t) => t,
        Err(err) => return TrustOutcome::Failed(format!("read CA thumbprint: {err}")),
    };

    let already_present = certutil_pre_check(&thumbprint);
    match (action, already_present) {
        (TrustAction::Install, true) => {
            tracing::debug!("Bridge CA {thumbprint} already trusted in user root store");
            return TrustOutcome::AlreadyTrusted;
        }
        (TrustAction::Untrust, false) => {
            tracing::debug!("Bridge CA {thumbprint} already absent from user root store");
            return TrustOutcome::AlreadyTrusted;
        }
        _ => {}
    }

    let args: Vec<&OsStr> = match action {
        TrustAction::Install => vec![
            "-user".as_ref(),
            "-addstore".as_ref(),
            "Root".as_ref(),
            ca_path.as_os_str(),
        ],
        TrustAction::Untrust => vec![
            "-user".as_ref(),
            "-delstore".as_ref(),
            "Root".as_ref(),
            thumbprint.as_ref(),
        ],
    };

    match run_quiet("certutil", &args) {
        Ok(out) if out.status.success() => {
            tracing::info!(
                "certutil {:?} succeeded for bridge CA",
                action_label(action)
            );
            TrustOutcome::Installed
        }
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr).trim().to_owned();
            let stdout = String::from_utf8_lossy(&out.stdout).trim().to_owned();
            TrustOutcome::Failed(format!(
                "certutil exit={}: {}",
                out.status,
                if !stderr.is_empty() { stderr } else { stdout }
            ))
        }
        Err(err) => TrustOutcome::Failed(format!("certutil invocation: {err}")),
    }
}

fn certutil_pre_check(thumbprint: &str) -> bool {
    let args: [&OsStr; 4] = [
        "-user".as_ref(),
        "-store".as_ref(),
        "Root".as_ref(),
        thumbprint.as_ref(),
    ];
    matches!(run_quiet("certutil", &args), Ok(out) if out.status.success())
}

fn read_ca_thumbprint(ca_path: &Path) -> Result<String, String> {
    let pem = fs::read(ca_path).map_err(|err| err.to_string())?;
    let der =
        read_first_certificate_der(&pem).ok_or_else(|| "no certificate in PEM file".to_string())?;
    // SHA-1 over the DER body — what `certutil -store Root <thumbprint>`
    // matches against. SHA-1 is fine for this trust-store keying use:
    // collisions don't help an attacker since the cert is local-only.
    let hash = Sha1::digest(&der);
    let mut hex = String::with_capacity(hash.len() * 2);
    for byte in hash.iter() {
        hex.push_str(&format!("{byte:02x}"));
    }
    Ok(hex)
}
