//! macOS keychain integration for the bridge CA. Drives the per-user
//! login keychain via `/usr/bin/security` so the trust install needs no
//! sudo and never re-prompts on reruns.

use std::env;
use std::path::{Path, PathBuf};

use crate::office_addin::bridge_certs::{
    CA_COMMON_NAME, TrustAction, TrustOutcome, action_label, run_quiet,
};

pub fn trust_impl(ca_path: &Path, action: TrustAction) -> TrustOutcome {
    let keychain = match env::var_os("HOME") {
        Some(home) => PathBuf::from(home).join("Library/Keychains/login.keychain-db"),
        None => return TrustOutcome::Failed("HOME is not set".into()),
    };

    // Pre-check via `security find-certificate`. Returns 0 when the
    // common-name match exists in the keychain.
    let already_present = matches!(
        run_quiet(
            "security",
            &[
                "find-certificate".as_ref(),
                "-c".as_ref(),
                CA_COMMON_NAME.as_ref(),
                keychain.as_os_str(),
            ],
        ),
        Ok(out) if out.status.success()
    );
    match (action, already_present) {
        (TrustAction::Install, true) => return TrustOutcome::AlreadyTrusted,
        (TrustAction::Untrust, false) => return TrustOutcome::AlreadyTrusted,
        _ => {}
    }

    let result = match action {
        TrustAction::Install => run_quiet(
            "security",
            &[
                "add-trusted-cert".as_ref(),
                "-k".as_ref(),
                keychain.as_os_str(),
                ca_path.as_os_str(),
            ],
        ),
        TrustAction::Untrust => run_quiet(
            "security",
            &[
                "delete-certificate".as_ref(),
                "-c".as_ref(),
                CA_COMMON_NAME.as_ref(),
                keychain.as_os_str(),
            ],
        ),
    };

    match result {
        Ok(out) if out.status.success() => {
            tracing::info!(
                "security {:?} succeeded for bridge CA",
                action_label(action)
            );
            TrustOutcome::Installed
        }
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr).trim().to_owned();
            let stdout = String::from_utf8_lossy(&out.stdout).trim().to_owned();
            TrustOutcome::Failed(format!(
                "security exit={}: {}",
                out.status,
                if !stderr.is_empty() { stderr } else { stdout }
            ))
        }
        Err(err) => TrustOutcome::Failed(format!("security invocation: {err}")),
    }
}
