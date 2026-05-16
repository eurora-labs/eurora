//! Bootstrap of the 32-byte file-encryption key.
//!
//! Lives behind a [`MainKey`] newtype that
//! * zeroes its bytes on drop,
//! * rejects degenerate values (all-zero, all-identical first byte),
//! * loads from / persists to the OS keychain on release builds,
//! * hard-codes a deterministic value under `#[cfg(debug_assertions)]`
//!   so developers don't need a populated keychain during `cargo run`.

use zeroize::{Zeroize, ZeroizeOnDrop};

use super::error::SecretStoreError;

/// Hard-coded key used in debug builds.
///
/// Skipping the keychain in debug means `cargo run`, integration tests,
/// and `tauri dev` reload work without prompting for credentials. The
/// constant is `#[cfg(debug_assertions)]` so it never ships in release
/// binaries.
#[cfg(debug_assertions)]
const DEV_MAIN_KEY: [u8; 32] = [
    0xA4, 0x1B, 0x7E, 0x3C, 0x92, 0xF0, 0x55, 0xD8, 0x6A, 0xC3, 0x11, 0xBF, 0x48, 0xE7, 0x2D, 0x9F,
    0x03, 0x86, 0xFA, 0x74, 0xCB, 0x60, 0x1D, 0xA5, 0x39, 0xEE, 0x57, 0x0C, 0xB2, 0x84, 0x63, 0xD1,
];

#[derive(Zeroize, ZeroizeOnDrop, Clone)]
pub(super) struct MainKey([u8; 32]);

impl std::fmt::Debug for MainKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("MainKey([REDACTED 32 bytes])")
    }
}

impl MainKey {
    pub(super) fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    fn from_bytes(bytes: [u8; 32]) -> Result<Self, SecretStoreError> {
        if bytes.iter().all(|&b| b == 0) {
            return Err(SecretStoreError::MainKey("cannot be all zeros"));
        }
        let first = bytes[0];
        if bytes.iter().all(|&b| b == first) {
            return Err(SecretStoreError::MainKey("has insufficient entropy"));
        }
        Ok(MainKey(bytes))
    }
}

/// Load the main key, generating + persisting a fresh one on first use.
///
/// Debug builds return [`DEV_MAIN_KEY`] and never touch the keychain;
/// release builds read from / write to a dedicated entry under the
/// shared `SERVICE` keychain namespace.
pub(super) fn load_or_create() -> Result<MainKey, SecretStoreError> {
    cfg_select! {
        debug_assertions => { MainKey::from_bytes(DEV_MAIN_KEY) }
        _ => { release::load_or_create() }
    }
}

#[cfg(not(debug_assertions))]
mod release {
    use base64::Engine as _;
    use base64::prelude::BASE64_STANDARD;
    use rand::Rng as _;
    use zeroize::Zeroizing;

    use super::super::SERVICE;
    use super::super::error::SecretStoreError;
    use super::MainKey;

    /// Keychain handle for the persisted main key.
    ///
    /// Preserved verbatim from the previous `euro-secret` layout so
    /// users upgrading from an earlier build keep the same key —
    /// otherwise their existing `secrets.enc` would fail to decrypt
    /// and they'd be silently signed out.
    const MAIN_KEY_HANDLE: &str = "eurora-USER_MAIN_KEY_HANDLE";

    pub(super) fn load_or_create() -> Result<MainKey, SecretStoreError> {
        let entry = keyring::Entry::new(MAIN_KEY_HANDLE, SERVICE)?;
        match entry.get_password() {
            Ok(b64) => decode(&b64),
            Err(keyring::Error::NoEntry) => generate_and_store(&entry),
            Err(err) => Err(err.into()),
        }
    }

    fn decode(b64: &str) -> Result<MainKey, SecretStoreError> {
        let bytes = BASE64_STANDARD
            .decode(b64.as_bytes())
            .map_err(|_| SecretStoreError::MainKey("stored value is not base64"))?;
        let bytes: [u8; 32] = bytes
            .try_into()
            .map_err(|_| SecretStoreError::MainKey("stored value is not 32 bytes"))?;
        MainKey::from_bytes(bytes)
    }

    fn generate_and_store(entry: &keyring::Entry) -> Result<MainKey, SecretStoreError> {
        let mut bytes = [0u8; 32];
        rand::rng().fill_bytes(&mut bytes);
        let key = MainKey::from_bytes(bytes)?;

        // base64 encoding allocates a fresh String; wrap it so we wipe
        // the heap allocation as soon as keyring has consumed it.
        let encoded = Zeroizing::new(BASE64_STANDARD.encode(key.as_bytes()));
        entry.set_password(&encoded)?;

        Ok(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_all_zero_key() {
        assert!(MainKey::from_bytes([0u8; 32]).is_err());
    }

    #[test]
    fn rejects_uniform_key() {
        assert!(MainKey::from_bytes([0x42u8; 32]).is_err());
    }

    #[test]
    fn accepts_random_key() {
        use rand::Rng as _;
        let mut bytes = [0u8; 32];
        rand::rng().fill_bytes(&mut bytes);
        assert!(MainKey::from_bytes(bytes).is_ok());
    }
}
