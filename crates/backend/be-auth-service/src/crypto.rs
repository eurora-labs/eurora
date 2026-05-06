//! Authenticated encryption for sensitive auth data at rest.
//!
//! Used for two distinct concerns:
//!
//! - **Ephemeral OAuth state** (PKCE verifier, OIDC nonce): rows expire in
//!   minutes, so simple key rotation is "wait one window then rotate".
//! - **Long-lived OAuth credentials** (`oauth_credentials` table): rows
//!   live as long as the user's account. To allow key rotation without
//!   bricking those rows, every ciphertext is prefixed with a one-byte
//!   *key version*. The primary key (used for new encryptions) is named
//!   by `PKCE_ENCRYPTION_KEY_VERSION`; older keys can be supplied as
//!   `PKCE_ENCRYPTION_KEY_V<N>` so existing ciphertexts keep decrypting.
//!
//! For backwards compatibility with rows written by the previous
//! version-less format, the legacy `PKCE_ENCRYPTION_KEY` env var is
//! accepted and treated as version `0`. New ciphertexts are always
//! written in the versioned format.

use std::collections::HashMap;

use chacha20poly1305::{
    Key, XChaCha20Poly1305, XNonce,
    aead::{Aead, KeyInit},
};
use rand::Rng;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("no encryption key configured (set PKCE_ENCRYPTION_KEY or PKCE_ENCRYPTION_KEY_V<N>)")]
    MissingEncryptionKey,

    #[error("invalid encryption key length: expected 32 bytes (64 hex chars), got {0}")]
    InvalidKeyLength(usize),

    #[error("failed to decode encryption key from hex")]
    HexDecodeError(#[from] hex::FromHexError),

    #[error("encryption failed")]
    EncryptionFailed,

    #[error("decryption failed")]
    DecryptionFailed,

    #[error("invalid encrypted data format")]
    InvalidFormat,

    #[error("ciphertext encrypted with unknown key version {0}")]
    UnknownKeyVersion(u8),

    #[error("invalid PKCE_ENCRYPTION_KEY_VERSION value: {0}")]
    InvalidKeyVersionEnv(String),
}

const NONCE_SIZE: usize = 24;
const VERSION_TAG: u8 = 0xFF;

const ENV_LEGACY_KEY: &str = "PKCE_ENCRYPTION_KEY";
const ENV_PRIMARY_VERSION: &str = "PKCE_ENCRYPTION_KEY_PRIMARY";
/// Trailing underscore disambiguates from any other variable that happens
/// to share the `PKCE_ENCRYPTION_KEY` prefix (e.g. the legacy var).
const ENV_VERSIONED_PREFIX: &str = "PKCE_ENCRYPTION_KEY_V_";

/// All keys known to this process: primary plus any decrypt-only fallbacks.
struct KeyRing {
    primary_version: u8,
    keys: HashMap<u8, Key>,
}

impl KeyRing {
    fn primary(&self) -> &Key {
        self.keys
            .get(&self.primary_version)
            .expect("primary key must be in keyring")
    }

    fn get(&self, version: u8) -> Option<&Key> {
        self.keys.get(&version)
    }
}

fn parse_key_hex(hex_value: &str) -> Result<Key, CryptoError> {
    let bytes = hex::decode(hex_value)?;
    if bytes.len() != 32 {
        return Err(CryptoError::InvalidKeyLength(bytes.len()));
    }
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    Ok(Key::from(arr))
}

fn load_keyring_from_env() -> Result<KeyRing, CryptoError> {
    let mut keys: HashMap<u8, Key> = HashMap::new();

    if let Ok(legacy) = std::env::var(ENV_LEGACY_KEY) {
        keys.insert(0, parse_key_hex(&legacy)?);
    }

    for (name, value) in std::env::vars() {
        if let Some(suffix) = name.strip_prefix(ENV_VERSIONED_PREFIX) {
            let version: u8 = suffix
                .parse()
                .map_err(|_| CryptoError::InvalidKeyVersionEnv(name.clone()))?;
            keys.insert(version, parse_key_hex(&value)?);
        }
    }

    if keys.is_empty() {
        return Err(CryptoError::MissingEncryptionKey);
    }

    let primary_version = match std::env::var(ENV_PRIMARY_VERSION) {
        Ok(v) => v
            .parse::<u8>()
            .map_err(|_| CryptoError::InvalidKeyVersionEnv(ENV_PRIMARY_VERSION.into()))?,
        Err(_) => {
            // Default: highest declared version. With only the legacy var, that's 0.
            *keys.keys().max().unwrap()
        }
    };

    if !keys.contains_key(&primary_version) {
        return Err(CryptoError::InvalidKeyVersionEnv(format!(
            "{ENV_PRIMARY_VERSION}={primary_version} but no key with that version is configured"
        )));
    }

    Ok(KeyRing {
        primary_version,
        keys,
    })
}

#[cfg(not(test))]
fn keyring() -> Result<&'static KeyRing, CryptoError> {
    static CACHED: std::sync::OnceLock<KeyRing> = std::sync::OnceLock::new();
    if let Some(k) = CACHED.get() {
        return Ok(k);
    }
    let loaded = load_keyring_from_env()?;
    Ok(CACHED.get_or_init(|| loaded))
}

#[cfg(test)]
fn keyring() -> Result<KeyRing, CryptoError> {
    load_keyring_from_env()
}

/// Encrypt `plaintext` under the keyring's primary key. The output layout
/// is `[VERSION_TAG | key_version | nonce(24) | ciphertext+tag]`. The
/// `VERSION_TAG` discriminator lets us cheaply tell the new format apart
/// from any legacy `[nonce(24) | ciphertext+tag]` rows that pre-date this
/// change — see [`decrypt_sensitive_string`].
pub fn encrypt_sensitive_string(plaintext: &str) -> Result<Vec<u8>, CryptoError> {
    let ring = keyring()?;
    let key = ring.primary();
    let cipher = XChaCha20Poly1305::new(key);

    let mut nonce_bytes = [0u8; NONCE_SIZE];
    rand::rng().fill_bytes(&mut nonce_bytes);
    let nonce = XNonce::from_slice(&nonce_bytes);

    let ciphertext = cipher.encrypt(nonce, plaintext.as_bytes()).map_err(|e| {
        tracing::error!(error = %e, "sensitive-data encryption failed");
        CryptoError::EncryptionFailed
    })?;

    let mut out = Vec::with_capacity(2 + NONCE_SIZE + ciphertext.len());
    out.push(VERSION_TAG);
    out.push(ring.primary_version);
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&ciphertext);
    Ok(out)
}

/// Decrypt a value produced by [`encrypt_sensitive_string`]. Also accepts
/// the legacy unversioned layout (`[nonce(24) | ciphertext+tag]`) for
/// rows written before the version byte was introduced; those are tried
/// against the version-`0` key (i.e. the legacy `PKCE_ENCRYPTION_KEY`).
pub fn decrypt_sensitive_string(encrypted: &[u8]) -> Result<String, CryptoError> {
    let ring = keyring_borrow()?;

    let (key_version, nonce_bytes, ciphertext) = if encrypted.first() == Some(&VERSION_TAG) {
        // Versioned layout.
        if encrypted.len() < 2 + NONCE_SIZE {
            return Err(CryptoError::InvalidFormat);
        }
        let version = encrypted[1];
        let nonce = &encrypted[2..2 + NONCE_SIZE];
        let ct = &encrypted[2 + NONCE_SIZE..];
        (version, nonce, ct)
    } else {
        // Legacy layout — try against version 0.
        if encrypted.len() < NONCE_SIZE {
            return Err(CryptoError::InvalidFormat);
        }
        let nonce = &encrypted[..NONCE_SIZE];
        let ct = &encrypted[NONCE_SIZE..];
        (0, nonce, ct)
    };

    let key = ring
        .get(key_version)
        .ok_or(CryptoError::UnknownKeyVersion(key_version))?;

    let cipher = XChaCha20Poly1305::new(key);
    let nonce = XNonce::from_slice(nonce_bytes);
    let plaintext = cipher.decrypt(nonce, ciphertext).map_err(|e| {
        tracing::error!(error = %e, "sensitive-data decryption failed");
        CryptoError::DecryptionFailed
    })?;

    String::from_utf8(plaintext).map_err(|_| CryptoError::DecryptionFailed)
}

#[cfg(not(test))]
fn keyring_borrow() -> Result<&'static KeyRing, CryptoError> {
    keyring()
}

#[cfg(test)]
fn keyring_borrow() -> Result<KeyRing, CryptoError> {
    keyring()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_MUTEX: Mutex<()> = Mutex::new(());

    const TEST_KEY_A: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    const TEST_KEY_B: &str = "fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210";

    fn clear_env() {
        unsafe {
            std::env::remove_var(ENV_LEGACY_KEY);
            std::env::remove_var(ENV_PRIMARY_VERSION);
            for v in 0u32..=10 {
                std::env::remove_var(format!("{ENV_VERSIONED_PREFIX}{v}"));
            }
        }
    }

    /// Test wrapper that clears all crypto-related env vars on entry and
    /// exit. Done in one place so a panicking test still has its env
    /// state cleaned up before the next test runs.
    fn with_clean_env<F: FnOnce()>(test: F) {
        // Acquire the lock; recover from poison so a previous panicking
        // test doesn't cascade into every subsequent test.
        let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| {
            ENV_MUTEX.clear_poison();
            e.into_inner()
        });
        clear_env();
        test();
        clear_env();
    }

    fn set_env(name: &str, value: &str) {
        unsafe {
            std::env::set_var(name, value);
        }
    }

    #[test]
    fn roundtrip_with_legacy_env() {
        with_clean_env(|| {
            set_env(ENV_LEGACY_KEY, TEST_KEY_A);

            let plaintext = "test_pkce_verifier_12345";
            let enc = encrypt_sensitive_string(plaintext).expect("encrypt");
            // versioned format
            assert_eq!(enc[0], VERSION_TAG);
            assert_eq!(enc[1], 0);

            let dec = decrypt_sensitive_string(&enc).expect("decrypt");
            assert_eq!(dec, plaintext);
        });
    }

    #[test]
    fn rejects_tampered_ciphertext() {
        with_clean_env(|| {
            set_env(ENV_LEGACY_KEY, TEST_KEY_A);

            let mut enc = encrypt_sensitive_string("hello").expect("encrypt");
            let last = enc.len() - 1;
            enc[last] ^= 0xFF;
            let result = decrypt_sensitive_string(&enc);
            assert!(matches!(result, Err(CryptoError::DecryptionFailed)));
        });
    }

    #[test]
    fn missing_key_returns_error() {
        with_clean_env(|| {
            let result = encrypt_sensitive_string("test");
            assert!(matches!(result, Err(CryptoError::MissingEncryptionKey)));
        });
    }

    #[test]
    fn invalid_key_length_returns_error() {
        with_clean_env(|| {
            set_env(ENV_LEGACY_KEY, "0123456789abcdef");
            let result = encrypt_sensitive_string("test");
            assert!(matches!(result, Err(CryptoError::InvalidKeyLength(_))));
        });
    }

    #[test]
    fn legacy_unversioned_ciphertext_still_decrypts() {
        with_clean_env(|| {
            set_env(ENV_LEGACY_KEY, TEST_KEY_A);

            // Build a legacy-format ciphertext directly (no version prefix).
            let key = parse_key_hex(TEST_KEY_A).unwrap();
            let cipher = XChaCha20Poly1305::new(&key);
            let mut nonce_bytes = [0u8; NONCE_SIZE];
            rand::rng().fill_bytes(&mut nonce_bytes);
            let nonce = XNonce::from_slice(&nonce_bytes);
            let plaintext = "legacy_payload";
            let ct = cipher.encrypt(nonce, plaintext.as_bytes()).unwrap();
            let mut legacy = Vec::with_capacity(NONCE_SIZE + ct.len());
            legacy.extend_from_slice(&nonce_bytes);
            legacy.extend_from_slice(&ct);

            let dec = decrypt_sensitive_string(&legacy).expect("legacy decrypt");
            assert_eq!(dec, plaintext);
        });
    }

    #[test]
    fn key_rotation_decrypts_old_versions() {
        with_clean_env(|| {
            // Stage 1: primary is V1, encrypt under it.
            set_env(&format!("{ENV_VERSIONED_PREFIX}1"), TEST_KEY_A);
            set_env(ENV_PRIMARY_VERSION, "1");
            let enc_v1 = encrypt_sensitive_string("under_v1").expect("encrypt v1");
            assert_eq!(enc_v1[1], 1);

            // Stage 2: rotate to V2, but keep V1 around for decryption.
            set_env(&format!("{ENV_VERSIONED_PREFIX}2"), TEST_KEY_B);
            set_env(ENV_PRIMARY_VERSION, "2");

            let dec = decrypt_sensitive_string(&enc_v1).expect("decrypt with old key still loaded");
            assert_eq!(dec, "under_v1");

            // New encryptions use V2.
            let enc_v2 = encrypt_sensitive_string("under_v2").expect("encrypt v2");
            assert_eq!(enc_v2[1], 2);
        });
    }

    #[test]
    fn unknown_key_version_returns_error() {
        with_clean_env(|| {
            set_env(ENV_LEGACY_KEY, TEST_KEY_A);

            // Hand-crafted ciphertext claiming version 9, which isn't loaded.
            let mut bytes = vec![VERSION_TAG, 9];
            bytes.extend_from_slice(&[0u8; NONCE_SIZE]);
            bytes.extend_from_slice(&[0u8; 16]);
            let result = decrypt_sensitive_string(&bytes);
            assert!(matches!(result, Err(CryptoError::UnknownKeyVersion(9))));
        });
    }
}
