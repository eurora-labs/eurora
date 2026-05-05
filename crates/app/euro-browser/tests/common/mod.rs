//! Shared test utilities for the bridge integration tests. Each
//! integration test in this crate stands up an ephemeral TLS bridge
//! against a freshly-minted CA living in a `tempfile::TempDir`.
//!
//! Lives under `tests/common/` per the standard cargo idiom for
//! integration-test helpers (suppresses the
//! `unused_crate_dependencies` warning that a lone `mod.rs` would
//! otherwise trip if linked from a single test file).

#![allow(dead_code)]

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::sync::Arc;

use euro_browser::TlsMaterial;
use rcgen::{
    BasicConstraints, CertificateParams, DistinguishedName, DnType, ExtendedKeyUsagePurpose,
    Ia5String, IsCa, KeyPair, KeyUsagePurpose, PKCS_ECDSA_P256_SHA256, SanType,
};
use rustls::{ClientConfig, RootCertStore, pki_types::CertificateDer};
use tempfile::TempDir;
use time::{Duration, OffsetDateTime};
use tokio_tungstenite::Connector;

pub struct TestChain {
    pub material: TlsMaterial,
    pub ca_pem: Vec<u8>,
    /// Backing tempdir; held to keep the cert files alive for the
    /// lifetime of the test.
    pub _root: TempDir,
}

/// Generate a fresh CA + `localhost` leaf into a tempdir. The chain
/// is functionally identical to what `bridge_certs::ensure_at` mints
/// in production, so this verifies the same code paths.
pub fn mint_localhost_chain() -> TestChain {
    let root = TempDir::new().expect("tempdir");
    let dir = root.path();

    let now = OffsetDateTime::now_utc();
    let ca_key = KeyPair::generate_for(&PKCS_ECDSA_P256_SHA256).unwrap();
    let mut ca_params = CertificateParams::default();
    ca_params.distinguished_name = DistinguishedName::new();
    ca_params
        .distinguished_name
        .push(DnType::CommonName, "Eurora Test CA");
    ca_params.is_ca = IsCa::Ca(BasicConstraints::Constrained(0));
    ca_params.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];
    ca_params.not_before = now - Duration::days(1);
    ca_params.not_after = now + Duration::days(365);
    let ca_cert = ca_params.self_signed(&ca_key).unwrap();

    let leaf_key = KeyPair::generate_for(&PKCS_ECDSA_P256_SHA256).unwrap();
    let mut leaf_params = CertificateParams::default();
    leaf_params.distinguished_name = DistinguishedName::new();
    leaf_params
        .distinguished_name
        .push(DnType::CommonName, "localhost");
    leaf_params.subject_alt_names = vec![
        SanType::DnsName(Ia5String::try_from("localhost".to_string()).unwrap()),
        SanType::IpAddress(IpAddr::V4(Ipv4Addr::LOCALHOST)),
        SanType::IpAddress(IpAddr::V6(Ipv6Addr::LOCALHOST)),
    ];
    leaf_params.key_usages = vec![
        KeyUsagePurpose::DigitalSignature,
        KeyUsagePurpose::KeyEncipherment,
    ];
    leaf_params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];
    leaf_params.not_before = now - Duration::days(1);
    leaf_params.not_after = now + Duration::days(180);
    let leaf_cert = leaf_params.signed_by(&leaf_key, &ca_cert, &ca_key).unwrap();

    let ca_pem = ca_cert.pem().into_bytes();
    let cert_path = dir.join("server.crt");
    let key_path = dir.join("server.key");
    std::fs::write(dir.join("ca.crt"), &ca_pem).unwrap();
    std::fs::write(&cert_path, leaf_cert.pem()).unwrap();
    std::fs::write(&key_path, leaf_key.serialize_pem()).unwrap();

    TestChain {
        material: TlsMaterial {
            cert_path,
            key_path,
        },
        ca_pem,
        _root: root,
    }
}

/// Build a tokio-tungstenite Connector trusting only the test CA in
/// `ca_pem`. Mirrors what the native-messaging host does in
/// production via `euro_native_messaging::tls::build_connector`.
pub fn client_connector(ca_pem: &[u8]) -> Connector {
    let mut roots = RootCertStore::empty();
    let mut reader = std::io::Cursor::new(ca_pem);
    for cert in rustls_pemfile::certs(&mut reader) {
        let cert: CertificateDer<'static> = cert.unwrap();
        roots.add(cert).unwrap();
    }
    let config = ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();
    Connector::Rustls(Arc::new(config))
}
