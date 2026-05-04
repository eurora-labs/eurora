//! Lifecycle for the local TLS chain that secures the
//! `wss://localhost:1431/bridge` channel between the desktop, the Word
//! add-in (over WebView2), and the browser native-messaging host.
//!
//! Two pieces, both idempotent on every desktop launch:
//!
//! - [`ensure`] mints (or rotates) the on-disk PEM material — a 10-year
//!   `Eurora Local Bridge CA` root and a 2-year `localhost` leaf signed
//!   by it — under `<app_data_dir>/bridge/`. The whole chain rotates as
//!   a unit so partial state is never observable; ECDSA P-256 keys are
//!   used everywhere because WebView2's TLS stack accepts them and they
//!   keep handshakes small.
//! - [`ensure_trusted`] adds the CA to the *user's* root store (no UAC,
//!   no admin/sudo), and prunes any prior `Eurora Local Bridge CA`
//!   entries left over from past rotations. The platform backend is
//!   responsible for making reruns idempotent so re-install never
//!   re-prompts the user.
//!
//! Every transition flows through `tracing` so the desktop's existing
//! log discipline (info on success, warn on recoverable failure)
//! applies uniformly. Failures are *non-fatal*: a missing trust chain
//! falls back to whatever cert error UI WebView2 surfaces, which is no
//! worse than the plaintext channel that Phase 1 replaces.

use std::fs;
use std::io::Write as _;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::path::{Path, PathBuf};

#[cfg(any(target_os = "macos", target_os = "windows"))]
use std::process::{Command, Output};

use rcgen::{
    BasicConstraints, CertificateParams, DistinguishedName, DnType, ExtendedKeyUsagePurpose,
    Ia5String, IsCa, KeyPair, KeyUsagePurpose, PKCS_ECDSA_P256_SHA256, SanType,
};
use tauri::{AppHandle, Manager, Runtime};
use tempfile::NamedTempFile;
use time::{Duration as TimeDuration, OffsetDateTime};

use super::{Error, Result};

const CA_CERT_FILENAME: &str = euro_bridge_protocol::BRIDGE_CA_FILENAME; // "ca.crt"
const CA_KEY_FILENAME: &str = "ca.key";
const SERVER_CERT_FILENAME: &str = "server.crt";
const SERVER_KEY_FILENAME: &str = "server.key";

/// Subject CN on the bridge CA. Surfaced to platform backends because
/// macOS and Windows both filter their root-store queries by CN
/// (`security find-certificate -c`, `certutil -store Root <CN>`).
pub(super) const CA_COMMON_NAME: &str = "Eurora Local Bridge CA";
const LEAF_COMMON_NAME: &str = "localhost";

/// CA validity: 10 years. Long enough to outlive most installs, short
/// enough to bound damage if the key ever leaks.
const CA_VALIDITY_DAYS: i64 = 365 * 10;
/// Leaf validity: 2 years. Rotated automatically at the renewal
/// threshold below.
const LEAF_VALIDITY_DAYS: i64 = 365 * 2;
/// Renew the chain when *any* cert is within this many days of
/// expiring (or already expired). Whole-chain rotation only — partial
/// rotation is not worth the complexity.
const RENEWAL_THRESHOLD_DAYS: i64 = 30;

/// Resolved on-disk paths for the bridge TLS chain.
#[derive(Debug, Clone)]
pub struct BridgeCerts {
    pub ca_path: PathBuf,
    pub ca_key_path: PathBuf,
    pub cert_path: PathBuf,
    pub key_path: PathBuf,
}

/// Outcome of [`ensure`] / [`ensure_at`], surfaced to callers for
/// logging.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnsureOutcome {
    /// All four files were already present, parseable, and not within
    /// the renewal window.
    Reused,
    /// Files were missing on disk and the chain was created from
    /// scratch.
    Generated,
    /// An existing chain was rotated. See [`RenewalReason`] for why.
    Renewed { reason: RenewalReason },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenewalReason {
    /// One or more files were missing.
    Missing,
    /// One or more files failed to parse as PEM/X.509.
    Unparseable,
    /// Some cert was within [`RENEWAL_THRESHOLD_DAYS`] of expiry.
    ExpiringSoon,
    /// Some cert was already past its `not_after`.
    Expired,
}

/// Outcome of [`ensure_trusted`] / [`ensure_untrusted`].
#[derive(Debug, Clone)]
pub enum TrustOutcome {
    /// `Install` only: the current CA was added to the user root
    /// store. `stale_removed` is the number of *prior*
    /// [`CA_COMMON_NAME`] entries (with different thumbprints, left
    /// over from past rotations) that were pruned in the same call.
    /// A successful no-op install where the CA was already present
    /// and the store was already clean is reported as [`Self::NoChange`]
    /// instead.
    Installed { stale_removed: usize },
    /// `Untrust` only: every [`CA_COMMON_NAME`] entry was deleted from
    /// the user root store. `removed` is the total count.
    Untrusted { removed: usize },
    /// The user root store was already in the desired state
    /// (current CA trusted with no stale rotations for `Install`;
    /// no [`CA_COMMON_NAME`] entries for `Untrust`).
    NoChange,
    /// This OS has no trust integration (Linux today — Word doesn't
    /// run natively).
    Skipped,
    /// The OS trust tool failed. The contained string is a single-line
    /// summary suitable for `tracing::warn!`.
    Failed(String),
}

/// Resolve the bridge directory under Tauri's per-user data dir, mint
/// (or rotate) the chain, and return its on-disk paths.
pub fn ensure<R: Runtime>(app: &AppHandle<R>) -> Result<BridgeCerts> {
    let data_dir = app.path().data_dir().map_err(|source| Error::Path {
        kind: "data_dir",
        source,
    })?;
    let root = euro_bridge_protocol::bridge_data_dir_under(&data_dir);
    let (certs, outcome) = ensure_at(&root)?;
    log_ensure_outcome(&outcome, &root);
    Ok(certs)
}

/// Same as [`ensure`] but takes the bridge directory as an argument.
/// Exposed so unit tests don't have to spin up a Tauri runtime.
pub fn ensure_at(root: &Path) -> Result<(BridgeCerts, EnsureOutcome)> {
    ensure_at_with_clock(root, OffsetDateTime::now_utc)
}

fn ensure_at_with_clock<F>(root: &Path, now: F) -> Result<(BridgeCerts, EnsureOutcome)>
where
    F: FnOnce() -> OffsetDateTime,
{
    fs::create_dir_all(root).map_err(|source| Error::Io {
        action: "creating",
        path: root.to_path_buf(),
        source,
    })?;

    let now_at = now();
    let certs = layout(root);
    let outcome = match validate_existing(&certs, now_at) {
        Validity::Ok => EnsureOutcome::Reused,
        Validity::NeedsRotation(reason) => {
            generate_chain(&certs, now_at)?;
            EnsureOutcome::Renewed { reason }
        }
        Validity::Empty => {
            generate_chain(&certs, now_at)?;
            EnsureOutcome::Generated
        }
    };
    Ok((certs, outcome))
}

fn layout(root: &Path) -> BridgeCerts {
    BridgeCerts {
        ca_path: root.join(CA_CERT_FILENAME),
        ca_key_path: root.join(CA_KEY_FILENAME),
        cert_path: root.join(SERVER_CERT_FILENAME),
        key_path: root.join(SERVER_KEY_FILENAME),
    }
}

enum Validity {
    /// All four files exist and are within their freshness window.
    Ok,
    /// At least one file exists but the chain needs to be rotated.
    NeedsRotation(RenewalReason),
    /// No files exist yet — first install.
    Empty,
}

fn validate_existing(certs: &BridgeCerts, now: OffsetDateTime) -> Validity {
    let any_present = [
        &certs.ca_path,
        &certs.ca_key_path,
        &certs.cert_path,
        &certs.key_path,
    ]
    .iter()
    .any(|p| p.exists());
    if !any_present {
        return Validity::Empty;
    }

    for path in [
        &certs.ca_path,
        &certs.ca_key_path,
        &certs.cert_path,
        &certs.key_path,
    ] {
        if !path.exists() {
            return Validity::NeedsRotation(RenewalReason::Missing);
        }
    }

    let renewal_window = TimeDuration::days(RENEWAL_THRESHOLD_DAYS);

    for cert_path in [&certs.ca_path, &certs.cert_path] {
        let pem = match fs::read(cert_path) {
            Ok(bytes) => bytes,
            Err(_) => return Validity::NeedsRotation(RenewalReason::Missing),
        };
        let der = match read_first_certificate_der(&pem) {
            Ok(der) => der,
            Err(_) => return Validity::NeedsRotation(RenewalReason::Unparseable),
        };
        let (_, parsed) = match x509_parser::parse_x509_certificate(&der) {
            Ok(v) => v,
            Err(_) => return Validity::NeedsRotation(RenewalReason::Unparseable),
        };
        let not_after =
            OffsetDateTime::from_unix_timestamp(parsed.validity().not_after.timestamp())
                .unwrap_or(OffsetDateTime::UNIX_EPOCH);
        if not_after <= now {
            return Validity::NeedsRotation(RenewalReason::Expired);
        }
        if not_after - now < renewal_window {
            return Validity::NeedsRotation(RenewalReason::ExpiringSoon);
        }
    }

    Validity::Ok
}

/// Why [`read_first_certificate_der`] could not return a DER body.
/// Distinguishes "no certificate present" from "PEM body malformed" so
/// callers can render the right diagnostic.
#[derive(Debug, thiserror::Error)]
pub(super) enum PemReadError {
    /// The file held no `BEGIN CERTIFICATE` block.
    #[error("no certificate found in PEM file")]
    Empty,
    /// A `BEGIN CERTIFICATE` block was present but the PEM body was
    /// malformed. The contained error is the underlying I/O / parse
    /// error from `rustls_pemfile`.
    #[error("malformed PEM body: {0}")]
    Malformed(#[source] std::io::Error),
}

/// Decode the first PEM certificate in `pem_bytes` to its DER body.
/// Surfaces "no cert" and "malformed PEM" as separate error variants
/// because the platform thumbprint code wants to render those
/// differently, while `validate_existing` collapses both to
/// [`RenewalReason::Unparseable`].
pub(super) fn read_first_certificate_der(
    pem_bytes: &[u8],
) -> std::result::Result<Vec<u8>, PemReadError> {
    let mut reader = std::io::Cursor::new(pem_bytes);
    match rustls_pemfile::certs(&mut reader).next() {
        Some(Ok(der)) => Ok(der.as_ref().to_vec()),
        Some(Err(err)) => Err(PemReadError::Malformed(err)),
        None => Err(PemReadError::Empty),
    }
}

/// SHA-1 thumbprint of the first certificate in the PEM file at
/// `ca_path`, formatted as lowercase hex with no separators.
///
/// This is the canonical key the platform trust backends use for both
/// pre-checks and deletes — `certutil -store Root <hex>` and
/// `security delete-certificate -Z <hex>` both accept this form. SHA-1
/// is fine for trust-store keying despite its cryptographic weakness:
/// collisions don't help an attacker since the cert is local-only and
/// minted by the desktop on first run.
#[cfg(any(target_os = "macos", target_os = "windows"))]
pub(super) fn ca_thumbprint(ca_path: &Path) -> std::result::Result<String, Error> {
    use sha1::{Digest, Sha1};

    let pem = fs::read(ca_path).map_err(|source| Error::Io {
        action: "reading",
        path: ca_path.to_path_buf(),
        source,
    })?;
    let der = read_first_certificate_der(&pem).map_err(|err| Error::CertParse {
        path: ca_path.to_path_buf(),
        reason: err.to_string(),
    })?;
    Ok(hex::encode(Sha1::digest(&der)))
}

fn generate_chain(certs: &BridgeCerts, now: OffsetDateTime) -> Result<()> {
    let ca_keypair = KeyPair::generate_for(&PKCS_ECDSA_P256_SHA256)?;
    let mut ca_params = CertificateParams::default();
    ca_params.distinguished_name = DistinguishedName::new();
    ca_params
        .distinguished_name
        .push(DnType::CommonName, CA_COMMON_NAME);
    ca_params.is_ca = IsCa::Ca(BasicConstraints::Constrained(0));
    ca_params.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];
    ca_params.not_before = now - TimeDuration::days(1);
    ca_params.not_after = now + TimeDuration::days(CA_VALIDITY_DAYS);
    let ca_cert = ca_params.self_signed(&ca_keypair)?;

    let leaf_keypair = KeyPair::generate_for(&PKCS_ECDSA_P256_SHA256)?;
    let mut leaf_params = CertificateParams::default();
    leaf_params.distinguished_name = DistinguishedName::new();
    leaf_params
        .distinguished_name
        .push(DnType::CommonName, LEAF_COMMON_NAME);
    let dns_name =
        Ia5String::try_from(LEAF_COMMON_NAME.to_string()).map_err(|err| Error::CertParse {
            path: certs.cert_path.clone(),
            reason: format!("DNS SAN encoding failed: {err}"),
        })?;
    leaf_params.subject_alt_names = vec![
        SanType::DnsName(dns_name),
        SanType::IpAddress(IpAddr::V4(Ipv4Addr::LOCALHOST)),
        SanType::IpAddress(IpAddr::V6(Ipv6Addr::LOCALHOST)),
    ];
    leaf_params.key_usages = vec![
        KeyUsagePurpose::DigitalSignature,
        KeyUsagePurpose::KeyEncipherment,
    ];
    leaf_params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];
    leaf_params.not_before = now - TimeDuration::days(1);
    leaf_params.not_after = now + TimeDuration::days(LEAF_VALIDITY_DAYS);
    let leaf_cert = leaf_params.signed_by(&leaf_keypair, &ca_cert, &ca_keypair)?;

    write_atomic(&certs.ca_path, ca_cert.pem().as_bytes())?;
    write_atomic_secret(&certs.ca_key_path, ca_keypair.serialize_pem().as_bytes())?;
    write_atomic(&certs.cert_path, leaf_cert.pem().as_bytes())?;
    write_atomic_secret(&certs.key_path, leaf_keypair.serialize_pem().as_bytes())?;

    Ok(())
}

/// Stage `contents` under a sibling tempfile, fsync, and atomically
/// rename into place. The tempfile is automatically unlinked on any
/// error path before `persist`.
fn write_atomic(path: &Path, contents: &[u8]) -> Result<()> {
    persist_via_tempfile(path, contents, /* secret */ false)
}

/// World-readable variant of [`write_atomic`] that additionally chmods
/// the file to 0600 on Unix before the rename. No-op on Windows: the
/// inheritance ACL on `<APPDATA>` already restricts the file to the
/// current user, and `set_permissions` on Windows would only toggle
/// the read-only bit.
fn write_atomic_secret(path: &Path, contents: &[u8]) -> Result<()> {
    persist_via_tempfile(path, contents, /* secret */ true)
}

fn persist_via_tempfile(path: &Path, contents: &[u8], secret: bool) -> Result<()> {
    let parent = path.parent().expect("layout paths always have a parent");
    fs::create_dir_all(parent).map_err(|source| Error::Io {
        action: "creating",
        path: parent.to_path_buf(),
        source,
    })?;
    let mut tmp = NamedTempFile::new_in(parent).map_err(|source| Error::Io {
        action: "writing",
        path: parent.to_path_buf(),
        source,
    })?;
    tmp.as_file_mut()
        .write_all(contents)
        .map_err(|source| Error::Io {
            action: "writing",
            path: tmp.path().to_path_buf(),
            source,
        })?;
    tmp.as_file_mut().sync_all().map_err(|source| Error::Io {
        action: "syncing",
        path: tmp.path().to_path_buf(),
        source,
    })?;
    #[cfg(unix)]
    if secret {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(tmp.path(), fs::Permissions::from_mode(0o600)).map_err(|source| {
            Error::Io {
                action: "chmod",
                path: tmp.path().to_path_buf(),
                source,
            }
        })?;
    }
    #[cfg(not(unix))]
    let _ = secret;
    tmp.persist(path).map_err(|err| Error::Io {
        action: "renaming",
        path: path.to_path_buf(),
        source: err.error,
    })?;
    fsync_dir(parent);
    Ok(())
}

/// Best-effort `fsync` of `dir` so the rename is durable across power
/// loss on POSIX. Failures are intentionally swallowed: this is belt-
/// and-braces durability over an already-renamed file, and a missing
/// `fsync` is no worse than the platform default.
#[cfg(unix)]
fn fsync_dir(dir: &Path) {
    if let Ok(handle) = fs::File::open(dir) {
        let _ = handle.sync_all();
    }
}

#[cfg(not(unix))]
fn fsync_dir(_dir: &Path) {}

fn log_ensure_outcome(outcome: &EnsureOutcome, root: &Path) {
    match outcome {
        EnsureOutcome::Reused => {
            tracing::debug!("Bridge TLS chain in {} is current; reusing", root.display())
        }
        EnsureOutcome::Generated => tracing::info!(
            "Generated bridge TLS chain in {} (no prior material on disk)",
            root.display()
        ),
        EnsureOutcome::Renewed { reason } => {
            tracing::info!("Rotated bridge TLS chain in {}: {reason:?}", root.display())
        }
    }
}

// ---------------------------------------------------------------------------
// Trust install — per-user, no UAC. The platform backend at
// `super::platform::bridge_certs` owns the actual root-store
// integration; we only fan out to `install` / `untrust` and shape
// the result.
// ---------------------------------------------------------------------------

/// Add the bridge CA at `ca_path` to the per-user OS root store, and
/// prune any prior `Eurora Local Bridge CA` entries left over from
/// past rotations. Idempotent: a pre-check via the OS query tool
/// short-circuits if the desired state is already in place. Failures
/// are non-fatal — the desktop logs and continues.
pub fn ensure_trusted(ca_path: &Path) -> TrustOutcome {
    super::platform::bridge_certs::install(ca_path)
}

/// Symmetric uninstall path. Removes every CN-matching CA from the
/// per-user root store, including any stale rotations. Idempotent;
/// takes no path because deletion is keyed off [`CA_COMMON_NAME`]
/// (the on-disk CA may already be gone by the time the user runs
/// the uninstaller).
pub fn ensure_untrusted() -> TrustOutcome {
    super::platform::bridge_certs::untrust()
}

/// Run a CLI tool, swallowing the console window pop-up on Windows so
/// trust operations stay invisible to the user. Promoted to
/// `pub(super)` so the macOS (`security`) and Windows (`certutil`)
/// backends share one process-launch path.
#[cfg(any(target_os = "macos", target_os = "windows"))]
pub(super) fn run_quiet(program: &str, args: &[&std::ffi::OsStr]) -> std::io::Result<Output> {
    let mut cmd = Command::new(program);
    cmd.args(args);
    #[cfg(target_os = "windows")]
    {
        // Suppress the console window that pops up when certutil is
        // launched from a windowed Tauri process.
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd.output()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn fixed_now() -> OffsetDateTime {
        // 2025-01-01T00:00:00Z — deterministic across runs.
        OffsetDateTime::from_unix_timestamp(1_735_689_600).unwrap()
    }

    #[test]
    fn ensure_at_generates_chain_when_root_empty() {
        let tmp = TempDir::new().unwrap();
        let (certs, outcome) = ensure_at(tmp.path()).unwrap();
        assert!(matches!(outcome, EnsureOutcome::Generated));

        for path in [
            &certs.ca_path,
            &certs.ca_key_path,
            &certs.cert_path,
            &certs.key_path,
        ] {
            assert!(path.exists(), "expected {} to exist", path.display());
        }

        let ca_pem = fs::read(&certs.ca_path).unwrap();
        let der = read_first_certificate_der(&ca_pem).expect("CA PEM should parse");
        let (_, parsed) = x509_parser::parse_x509_certificate(&der).expect("DER should parse");
        let cn = parsed
            .subject()
            .iter_common_name()
            .next()
            .unwrap()
            .as_str()
            .unwrap();
        assert_eq!(cn, CA_COMMON_NAME);
        assert!(parsed.tbs_certificate.is_ca());
    }

    #[test]
    fn ensure_at_is_idempotent() {
        let tmp = TempDir::new().unwrap();
        let (first, _) = ensure_at(tmp.path()).unwrap();
        let first_pem = fs::read(&first.ca_path).unwrap();

        let (second, outcome) = ensure_at(tmp.path()).unwrap();
        assert!(matches!(outcome, EnsureOutcome::Reused));
        let second_pem = fs::read(&second.ca_path).unwrap();
        assert_eq!(first_pem, second_pem, "second call must not rotate");
    }

    #[test]
    fn ensure_at_renews_when_files_unparseable() {
        let tmp = TempDir::new().unwrap();
        let certs = layout(tmp.path());
        fs::create_dir_all(tmp.path()).unwrap();
        for path in [
            &certs.ca_path,
            &certs.ca_key_path,
            &certs.cert_path,
            &certs.key_path,
        ] {
            fs::write(path, b"not a certificate").unwrap();
        }

        let (_, outcome) = ensure_at(tmp.path()).unwrap();
        assert_eq!(
            outcome,
            EnsureOutcome::Renewed {
                reason: RenewalReason::Unparseable
            }
        );
    }

    #[test]
    fn ensure_at_renews_when_some_files_missing() {
        let tmp = TempDir::new().unwrap();
        // Generate a chain, then delete one of the four files.
        let (certs, _) = ensure_at(tmp.path()).unwrap();
        fs::remove_file(&certs.cert_path).unwrap();

        let (_, outcome) = ensure_at(tmp.path()).unwrap();
        assert_eq!(
            outcome,
            EnsureOutcome::Renewed {
                reason: RenewalReason::Missing
            }
        );
    }

    #[test]
    fn ensure_at_renews_when_only_ca_present_on_fresh_dir() {
        // Distinct from the rotated-then-deleted case above: here the
        // directory has never seen a full chain. The "any present"
        // gate trips, then the "all present" gate must report
        // `Missing` rather than `Empty`.
        let tmp = TempDir::new().unwrap();
        let certs = layout(tmp.path());
        fs::create_dir_all(tmp.path()).unwrap();
        fs::write(&certs.ca_path, b"placeholder").unwrap();

        let (_, outcome) = ensure_at(tmp.path()).unwrap();
        assert_eq!(
            outcome,
            EnsureOutcome::Renewed {
                reason: RenewalReason::Missing
            }
        );
    }

    #[test]
    fn ensure_at_renews_when_within_renewal_window() {
        let tmp = TempDir::new().unwrap();
        // Mint a chain "long ago" — its leaf is 2y from `then`, so a
        // `now` that's 1y10mo later puts the chain inside the 30-day
        // renewal window.
        let then = fixed_now();
        ensure_at_with_clock(tmp.path(), || then).unwrap();

        let near_expiry = then + TimeDuration::days(LEAF_VALIDITY_DAYS - 10);
        let (_, outcome) = ensure_at_with_clock(tmp.path(), || near_expiry).unwrap();
        assert_eq!(
            outcome,
            EnsureOutcome::Renewed {
                reason: RenewalReason::ExpiringSoon
            }
        );
    }

    #[test]
    fn ensure_at_renews_when_expired() {
        let tmp = TempDir::new().unwrap();
        let then = fixed_now();
        ensure_at_with_clock(tmp.path(), || then).unwrap();

        let way_past = then + TimeDuration::days(LEAF_VALIDITY_DAYS + 30);
        let (_, outcome) = ensure_at_with_clock(tmp.path(), || way_past).unwrap();
        assert_eq!(
            outcome,
            EnsureOutcome::Renewed {
                reason: RenewalReason::Expired
            }
        );
    }

    #[test]
    fn leaf_san_includes_localhost_and_loopback_ips() {
        let tmp = TempDir::new().unwrap();
        let (certs, _) = ensure_at(tmp.path()).unwrap();
        let pem = fs::read(&certs.cert_path).unwrap();
        let der = read_first_certificate_der(&pem).unwrap();
        let (_, parsed) = x509_parser::parse_x509_certificate(&der).unwrap();
        let san_ext = parsed
            .extensions()
            .iter()
            .find_map(|ext| match ext.parsed_extension() {
                x509_parser::extensions::ParsedExtension::SubjectAlternativeName(san) => Some(san),
                _ => None,
            })
            .expect("SAN extension present");

        let mut saw_localhost = false;
        let mut saw_v4 = false;
        let mut saw_v6 = false;
        for name in &san_ext.general_names {
            match name {
                x509_parser::extensions::GeneralName::DNSName(s) if *s == "localhost" => {
                    saw_localhost = true
                }
                x509_parser::extensions::GeneralName::IPAddress(bytes) => {
                    if bytes.len() == 4 && *bytes == [127, 0, 0, 1] {
                        saw_v4 = true;
                    } else if bytes.len() == 16 {
                        let mut v6 = [0u8; 16];
                        v6.copy_from_slice(bytes);
                        if v6 == Ipv6Addr::LOCALHOST.octets() {
                            saw_v6 = true;
                        }
                    }
                }
                _ => {}
            }
        }
        assert!(saw_localhost, "SAN missing DNS:localhost");
        assert!(saw_v4, "SAN missing IP:127.0.0.1");
        assert!(saw_v6, "SAN missing IP:::1");
    }

    #[test]
    fn read_first_certificate_der_distinguishes_empty_and_malformed() {
        let empty = read_first_certificate_der(b"not pem at all").unwrap_err();
        assert!(matches!(empty, PemReadError::Empty), "{empty:?}");

        let malformed = read_first_certificate_der(
            b"-----BEGIN CERTIFICATE-----\nnot base64!\n-----END CERTIFICATE-----\n",
        )
        .unwrap_err();
        assert!(
            matches!(malformed, PemReadError::Malformed(_)),
            "{malformed:?}"
        );
    }

    #[cfg(any(target_os = "macos", target_os = "windows"))]
    #[test]
    fn ca_thumbprint_is_lowercase_hex_of_sha1_over_der() {
        use sha1::{Digest, Sha1};
        let tmp = TempDir::new().unwrap();
        let (certs, _) = ensure_at(tmp.path()).unwrap();

        let pem = fs::read(&certs.ca_path).unwrap();
        let der = read_first_certificate_der(&pem).unwrap();
        let expected = hex::encode(Sha1::digest(&der));
        let got = ca_thumbprint(&certs.ca_path).unwrap();
        assert_eq!(got, expected);
        assert_eq!(got.len(), 40);
        assert!(
            got.chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
        );
    }

    #[cfg(unix)]
    #[test]
    fn key_files_are_mode_0600_on_unix() {
        use std::os::unix::fs::PermissionsExt;

        let tmp = TempDir::new().unwrap();
        let (certs, _) = ensure_at(tmp.path()).unwrap();
        for key in [&certs.ca_key_path, &certs.key_path] {
            let mode = fs::metadata(key).unwrap().permissions().mode() & 0o777;
            assert_eq!(
                mode,
                0o600,
                "{} should be 0600, was {mode:o}",
                key.display()
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn cert_files_are_world_readable_on_unix() {
        use std::os::unix::fs::PermissionsExt;

        let tmp = TempDir::new().unwrap();
        let (certs, _) = ensure_at(tmp.path()).unwrap();
        for public in [&certs.ca_path, &certs.cert_path] {
            let mode = fs::metadata(public).unwrap().permissions().mode() & 0o777;
            // Whatever umask the test runner has, the public files
            // must at least let the owner read; we do not chmod them
            // explicitly, so we assert the negative — they are not
            // 0600-locked.
            assert!(
                mode & 0o400 != 0,
                "{} should be readable by owner",
                public.display()
            );
        }
    }
}
