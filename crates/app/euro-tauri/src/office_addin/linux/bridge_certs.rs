//! Linux backend for bridge CA trust install. Both entry points are
//! no-ops because no consumer on Linux relies on the OS root store:
//!
//! - Word for the web / Word desktop don't run natively here, so the
//!   WebView trust path that drives the macOS / Windows backends has
//!   no equivalent to converge on.
//! - The native-messaging host loads `ca.crt` directly into rustls,
//!   bypassing the OS trust store entirely.
//!
//! [`super::ensure`] still mints the on-disk PEM material on Linux so
//! the native-messaging host has something to read; this file just
//! reports `Skipped` for the trust step itself.

use std::path::Path;

use crate::office_addin::bridge_certs::TrustOutcome;

pub fn install(_ca_path: &Path) -> TrustOutcome {
    TrustOutcome::Skipped
}

pub fn untrust() -> TrustOutcome {
    TrustOutcome::Skipped
}
