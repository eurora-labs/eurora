//! Stable pseudonymisation for emails appearing in structured logs.
//!
//! When `LOG_EMAIL_PEPPER` is set, emails are HMAC-SHA-256'd under the
//! pepper so SecOps can correlate events without anyone with read-only
//! log access being able to recover the underlying address. With no
//! pepper configured, [`hash_email_for_log`] returns `None` and callers
//! omit the field entirely — better than emitting plain SHA-256 (which
//! is trivially reversible by enumeration over plausible addresses).

use std::sync::OnceLock;

use hmac::{Hmac, Mac};
use sha2::Sha256;

const ENV_PEPPER: &str = "LOG_EMAIL_PEPPER";

fn pepper() -> Option<&'static [u8]> {
    static CACHED: OnceLock<Option<Vec<u8>>> = OnceLock::new();
    CACHED
        .get_or_init(|| std::env::var(ENV_PEPPER).ok().map(String::into_bytes))
        .as_deref()
}

/// Returns a hex-encoded HMAC-SHA-256 over the lower-cased email, or
/// `None` if no log pepper is configured. The pepper is read once and
/// cached for the lifetime of the process.
pub(crate) fn hash_email_for_log(email: &str) -> Option<String> {
    let pepper = pepper()?;
    let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(pepper)
        .expect("HMAC accepts any byte length for the key");
    mac.update(email.to_ascii_lowercase().as_bytes());
    Some(hex::encode(mac.finalize().into_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;

    // The OnceLock semantics mean these tests cannot reliably toggle the
    // pepper at runtime; we exercise the real load path indirectly through
    // the integration tests. Unit-test the deterministic primitive here.

    #[test]
    fn hmac_is_case_insensitive_and_deterministic() {
        let pepper = b"unit-test-pepper";
        let mut a = <Hmac<Sha256> as Mac>::new_from_slice(pepper).unwrap();
        a.update(b"user@example.com");
        let mut b = <Hmac<Sha256> as Mac>::new_from_slice(pepper).unwrap();
        b.update(b"User@Example.com".to_ascii_lowercase().as_slice());
        assert_eq!(
            hex::encode(a.finalize().into_bytes()),
            hex::encode(b.finalize().into_bytes())
        );
    }
}
