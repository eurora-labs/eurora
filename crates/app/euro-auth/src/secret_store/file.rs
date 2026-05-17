//! Encrypted on-disk persistence for [`super::PersistedState`].
//!
//! Storage layout: `[24-byte nonce] || XChaCha20Poly1305(plaintext)`,
//! where `plaintext = serde_json::to_vec(&PersistedState)`. The nonce
//! is freshly random per write — XChaCha20-Poly1305's 192-bit nonce
//! space makes collision under random sampling negligible.
//!
//! Writes go via a temp-file-then-rename to avoid leaving half-written
//! `secrets.enc` files behind if the process crashes mid-flush. On
//! Unix the temp file is `chmod 0o600`'d before the rename so the
//! final file inherits the restrictive mode.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{Key, XChaCha20Poly1305, XNonce};
use rand::Rng as _;
use zeroize::{Zeroize, Zeroizing};

use super::PersistedState;
use super::error::SecretStoreError;
use super::main_key::MainKey;

pub(super) const STORE_FILENAME: &str = "secrets.enc";
const NONCE_BYTES: usize = 24;
const TAG_BYTES: usize = 16;
const MIN_FILE_LEN: usize = NONCE_BYTES + TAG_BYTES;

/// Cached key material wrapped so the bytes are wiped on drop.
pub(super) struct Cipher {
    key: Key,
}

impl Drop for Cipher {
    fn drop(&mut self) {
        self.key.as_mut_slice().zeroize();
    }
}

impl Cipher {
    pub(super) fn new(key: &MainKey) -> Self {
        Self {
            key: Key::from(*key.as_bytes()),
        }
    }

    fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>, SecretStoreError> {
        let cipher = XChaCha20Poly1305::new(&self.key);
        let mut nonce_bytes = [0u8; NONCE_BYTES];
        rand::rng().fill_bytes(&mut nonce_bytes);
        let nonce = XNonce::from_slice(&nonce_bytes);
        let ciphertext = cipher
            .encrypt(nonce, plaintext)
            .map_err(|_| SecretStoreError::Encrypt)?;
        let mut out = Vec::with_capacity(NONCE_BYTES + ciphertext.len());
        out.extend_from_slice(&nonce_bytes);
        out.extend_from_slice(&ciphertext);
        Ok(out)
    }

    fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>, SecretStoreError> {
        if data.len() < MIN_FILE_LEN {
            return Err(SecretStoreError::Decrypt);
        }
        let (nonce_bytes, ciphertext) = data.split_at(NONCE_BYTES);
        let nonce = XNonce::from_slice(nonce_bytes);
        let cipher = XChaCha20Poly1305::new(&self.key);
        cipher
            .decrypt(nonce, ciphertext)
            .map_err(|_| SecretStoreError::Decrypt)
    }
}

/// Read and decrypt `path` into a [`PersistedState`].
///
/// Returns `Ok(PersistedState::default())` when there's nothing to
/// load — either because the file doesn't exist yet (first launch) or
/// because the existing file can't be decrypted/parsed (key changed
/// or file corrupted). In the latter case the bad file is removed and
/// the event is logged. This is deliberately permissive: refusing to
/// start would strand the user, whereas falling back to "fresh state"
/// just makes them re-authenticate.
pub(super) fn load(cipher: &Cipher, path: &Path) -> Result<PersistedState, SecretStoreError> {
    let ciphertext = match fs::read(path) {
        Ok(b) => b,
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            return Ok(PersistedState::default());
        }
        Err(err) => {
            return Err(SecretStoreError::Io {
                path: path.to_path_buf(),
                source: err,
            });
        }
    };

    match decode(cipher, &ciphertext) {
        Ok(state) => Ok(state),
        Err(err) => {
            tracing::warn!(
                path = %path.display(),
                error = %err,
                "secret store could not be decrypted (key may have changed); starting fresh",
            );
            // Best-effort cleanup; failures here aren't actionable.
            let _ = fs::remove_file(path);
            Ok(PersistedState::default())
        }
    }
}

fn decode(cipher: &Cipher, ciphertext: &[u8]) -> Result<PersistedState, SecretStoreError> {
    let plaintext = Zeroizing::new(cipher.decrypt(ciphertext)?);
    let state = serde_json::from_slice(&plaintext)?;
    Ok(state)
}

/// Encrypt `state` and atomically replace `path`.
pub(super) fn save(
    cipher: &Cipher,
    state: &PersistedState,
    path: &Path,
) -> Result<(), SecretStoreError> {
    let plaintext = Zeroizing::new(serde_json::to_vec(state)?);
    let ciphertext = cipher.encrypt(&plaintext)?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| SecretStoreError::Io {
            path: parent.to_path_buf(),
            source: err,
        })?;
    }

    let tmp_path = tmp_path_for(path);
    fs::write(&tmp_path, &ciphertext).map_err(|err| SecretStoreError::Io {
        path: tmp_path.clone(),
        source: err,
    })?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&tmp_path, fs::Permissions::from_mode(0o600)).map_err(|err| {
            SecretStoreError::Io {
                path: tmp_path.clone(),
                source: err,
            }
        })?;
    }

    fs::rename(&tmp_path, path).map_err(|err| SecretStoreError::Io {
        path: path.to_path_buf(),
        source: err,
    })
}

/// Best-effort deletion of the on-disk file. `NotFound` is success.
pub(super) fn remove(path: &Path) -> Result<(), SecretStoreError> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(SecretStoreError::Io {
            path: path.to_path_buf(),
            source: err,
        }),
    }
}

fn tmp_path_for(path: &Path) -> PathBuf {
    path.with_extension("enc.tmp")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn random_cipher() -> Cipher {
        let mut bytes = [0u8; 32];
        rand::rng().fill_bytes(&mut bytes);
        // Construct a MainKey directly via the test-only debug build
        // path is overkill; just stand up a Cipher with random key bytes.
        Cipher {
            key: Key::from(bytes),
        }
    }

    #[test]
    fn round_trip_persists_all_fields() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(STORE_FILENAME);
        let cipher = random_cipher();

        let original = PersistedState {
            access_token: Some("access".into()),
            refresh_token: Some("refresh".into()),
            pkce_verifier: Some("pkce".into()),
        };
        save(&cipher, &original, &path).unwrap();

        let loaded = load(&cipher, &path).unwrap();
        assert_eq!(loaded.access_token.as_deref(), Some("access"));
        assert_eq!(loaded.refresh_token.as_deref(), Some("refresh"));
        assert_eq!(loaded.pkce_verifier.as_deref(), Some("pkce"));
    }

    #[test]
    fn load_returns_default_when_file_missing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(STORE_FILENAME);
        let cipher = random_cipher();

        let loaded = load(&cipher, &path).unwrap();
        assert!(loaded.access_token.is_none());
        assert!(loaded.refresh_token.is_none());
        assert!(loaded.pkce_verifier.is_none());
    }

    #[test]
    fn load_with_wrong_key_recovers_to_default_and_deletes_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(STORE_FILENAME);

        let writer = random_cipher();
        let mut doomed = PersistedState::default();
        doomed.access_token = Some("doomed".into());
        save(&writer, &doomed, &path).unwrap();
        assert!(path.exists());

        let reader = random_cipher();
        let loaded = load(&reader, &path).unwrap();
        assert!(loaded.access_token.is_none());
        assert!(!path.exists(), "corrupt file should be cleaned up");
    }

    #[test]
    fn save_creates_parent_directories() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("a").join("b").join(STORE_FILENAME);
        let cipher = random_cipher();

        save(&cipher, &PersistedState::default(), &nested).unwrap();
        assert!(nested.exists());
    }

    #[test]
    fn save_writes_mode_0600_on_unix() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(STORE_FILENAME);
        let cipher = random_cipher();

        save(&cipher, &PersistedState::default(), &path).unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600);
        }
    }

    #[test]
    fn remove_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(STORE_FILENAME);
        remove(&path).unwrap();
        remove(&path).unwrap();
    }

    #[test]
    fn decrypt_rejects_truncated_input() {
        let cipher = random_cipher();
        assert!(cipher.decrypt(&[0u8; 10]).is_err());
    }
}
