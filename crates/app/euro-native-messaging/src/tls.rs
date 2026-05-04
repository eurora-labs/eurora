//! Build a `tokio_tungstenite::Connector` whose trust store contains
//! exactly the `Eurora Local Bridge CA`. The native-messaging host
//! only ever connects to one peer on the same machine, so pinning to
//! that single CA is more conservative than relying on the OS root
//! store (the path WebView2 has to take, since it has no API for
//! private trust).
//!
//! `build_connector` returns `None` when the CA file is not yet on
//! disk — typically because the desktop hasn't run for the first
//! time since install. The caller's reconnect loop retries
//! naturally.

use std::path::Path;
use std::sync::Arc;

use rustls::{ClientConfig, RootCertStore};
use tokio_tungstenite::Connector;

/// Idempotently install the rustls process-wide crypto provider. Safe
/// to call from `main` and from tests; subsequent calls are dropped.
pub fn install_default_crypto_provider() {
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
}

/// Build a TLS Connector pinned to the bridge CA at the conventional
/// path (`euro_bridge_protocol::bridge_ca_path()`). Returns `None` if
/// the CA file is missing.
pub fn build_connector() -> Option<Connector> {
    let path = euro_bridge_protocol::bridge_ca_path()?;
    build_connector_at(&path)
}

/// Build a TLS Connector pinned to the bridge CA at `ca_path`.
/// Returns `None` if the CA file is missing or contains no
/// certificates. Logs through `tracing` on every error path so the
/// host's existing log discipline applies.
pub fn build_connector_at(ca_path: &Path) -> Option<Connector> {
    let pem = match std::fs::read(ca_path) {
        Ok(p) => p,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            tracing::debug!("Bridge CA not yet present at {}", ca_path.display());
            return None;
        }
        Err(err) => {
            tracing::warn!("Failed to read bridge CA at {}: {err}", ca_path.display());
            return None;
        }
    };

    let mut roots = RootCertStore::empty();
    let mut reader = std::io::Cursor::new(pem);
    let mut count = 0usize;
    for cert in rustls_pemfile::certs(&mut reader) {
        match cert {
            Ok(cert) => {
                if let Err(err) = roots.add(cert) {
                    tracing::warn!(
                        "Failed to add bridge CA cert from {}: {err}",
                        ca_path.display()
                    );
                    return None;
                }
                count += 1;
            }
            Err(err) => {
                tracing::warn!(
                    "Failed to parse bridge CA PEM at {}: {err}",
                    ca_path.display()
                );
                return None;
            }
        }
    }
    if count == 0 {
        tracing::warn!(
            "Bridge CA file at {} contained no certificates",
            ca_path.display()
        );
        return None;
    }

    let config = ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();
    Some(Connector::Rustls(Arc::new(config)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn build_connector_returns_none_when_file_missing() {
        let tmp = TempDir::new().unwrap();
        let missing = tmp.path().join("nope.crt");
        assert!(build_connector_at(&missing).is_none());
    }

    #[test]
    fn build_connector_returns_none_when_pem_is_empty() {
        let tmp = TempDir::new().unwrap();
        let empty = tmp.path().join("empty.crt");
        std::fs::write(&empty, "").unwrap();
        assert!(build_connector_at(&empty).is_none());
    }

    #[test]
    fn build_connector_returns_none_when_pem_is_garbage() {
        let tmp = TempDir::new().unwrap();
        let garbage = tmp.path().join("garbage.crt");
        std::fs::write(&garbage, "this is not a certificate").unwrap();
        assert!(build_connector_at(&garbage).is_none());
    }

    #[test]
    fn build_connector_loads_a_real_pem() {
        // Mint a one-shot self-signed cert via rcgen so this test
        // doesn't depend on disk state. The connector just has to be
        // *some* `Some` — the certificate's contents don't matter.
        install_default_crypto_provider();
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("ca.crt");
        let key = rcgen::KeyPair::generate_for(&rcgen::PKCS_ECDSA_P256_SHA256).unwrap();
        let mut params = rcgen::CertificateParams::default();
        params
            .distinguished_name
            .push(rcgen::DnType::CommonName, "test");
        let cert = params.self_signed(&key).unwrap();
        std::fs::write(&path, cert.pem()).unwrap();

        assert!(build_connector_at(&path).is_some());
    }
}
