use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use anyhow::{Context, Result, bail};
use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{Key, XChaCha20Poly1305, XNonce};
use rand::RngCore;
use zeroize::Zeroize;

const STORE_FILENAME: &str = "secrets.enc";
const MIN_FILE_LEN: usize = 24 + 16;

struct SecretStore {
    secrets: HashMap<String, String>,
    key: Key,
    path: PathBuf,
}

impl Drop for SecretStore {
    fn drop(&mut self) {
        for value in self.secrets.values_mut() {
            value.zeroize();
        }
        self.key.zeroize();
    }
}

static STORE: OnceLock<Mutex<SecretStore>> = OnceLock::new();

pub(crate) fn init(encryption_key: [u8; 32], data_dir: PathBuf) -> Result<()> {
    let path = data_dir.join(STORE_FILENAME);
    let key = Key::from(encryption_key);

    let secrets = if path.exists() {
        let data = fs::read(&path)
            .with_context(|| format!("failed to read secret store at {}", path.display()))?;
        decrypt_store(&key, &data)
            .with_context(|| format!("failed to decrypt secret store at {}", path.display()))?
    } else {
        HashMap::new()
    };

    tracing::debug!(
        path = %path.display(),
        num_secrets = secrets.len(),
        "encrypted secret store initialised",
    );

    STORE
        .set(Mutex::new(SecretStore { secrets, key, path }))
        .map_err(|_| anyhow::anyhow!("secret file store already initialised"))?;

    Ok(())
}

pub(crate) fn is_initialized() -> bool {
    STORE.get().is_some()
}

pub(crate) fn get(qualified_handle: &str) -> Result<Option<String>> {
    let store = lock()?;
    Ok(store.secrets.get(qualified_handle).cloned())
}

pub(crate) fn set(qualified_handle: &str, value: &str) -> Result<()> {
    let mut store = lock()?;

    if value.is_empty() {
        store.secrets.remove(qualified_handle);
    } else {
        store
            .secrets
            .insert(qualified_handle.to_owned(), value.to_owned());
    }

    flush(&store)
}

pub(crate) fn remove(qualified_handle: &str) -> Result<()> {
    let mut store = lock()?;
    if store.secrets.remove(qualified_handle).is_some() {
        flush(&store)?;
    }
    Ok(())
}

fn lock() -> Result<std::sync::MutexGuard<'static, SecretStore>> {
    STORE
        .get()
        .ok_or_else(|| {
            anyhow::anyhow!("secret file store not initialised — call init_file_store first")
        })?
        .lock()
        .map_err(|e| anyhow::anyhow!("secret store lock poisoned: {e}"))
}

fn flush(store: &SecretStore) -> Result<()> {
    let json = serde_json::to_vec(&store.secrets).context("failed to serialise secrets")?;
    let encrypted = encrypt_store(&store.key, &json)?;

    if let Some(parent) = store.path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create directory {}", parent.display()))?;
    }

    // Atomic write via temp + rename to avoid half-written files on crash.
    let tmp_path = store.path.with_extension("enc.tmp");
    fs::write(&tmp_path, &encrypted)
        .with_context(|| format!("failed to write temp file {}", tmp_path.display()))?;

    // Restrict file permissions so only the owning user can read the secrets.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&tmp_path, fs::Permissions::from_mode(0o600))
            .with_context(|| format!("failed to set permissions on {}", tmp_path.display()))?;
    }

    fs::rename(&tmp_path, &store.path)
        .with_context(|| format!("failed to rename temp file to {}", store.path.display()))?;

    Ok(())
}

fn encrypt_store(key: &Key, plaintext: &[u8]) -> Result<Vec<u8>> {
    let cipher = XChaCha20Poly1305::new(key);

    let mut nonce_bytes = [0u8; 24];
    rand::rng().fill_bytes(&mut nonce_bytes);
    let nonce = XNonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| anyhow::anyhow!("secret store encryption failed: {e}"))?;

    let mut out = Vec::with_capacity(24 + ciphertext.len());
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&ciphertext);
    Ok(out)
}

fn decrypt_store(key: &Key, data: &[u8]) -> Result<HashMap<String, String>> {
    if data.len() < MIN_FILE_LEN {
        bail!(
            "secret store file too short ({} bytes, need at least {})",
            data.len(),
            MIN_FILE_LEN,
        );
    }

    let (nonce_bytes, ciphertext) = data.split_at(24);
    let nonce = XNonce::from_slice(nonce_bytes);
    let cipher = XChaCha20Poly1305::new(key);

    let plaintext = cipher.decrypt(nonce, ciphertext).map_err(|_| {
        anyhow::anyhow!("failed to decrypt secret store (wrong key or corrupted file)")
    })?;

    serde_json::from_slice(&plaintext).context("failed to parse decrypted secret store as JSON")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_store(key: [u8; 32]) -> (PathBuf, Key) {
        let dir = std::env::temp_dir().join(format!("euro-secret-test-{}", rand::random::<u64>()));
        fs::create_dir_all(&dir).unwrap();
        (dir, Key::from(key))
    }

    #[test]
    fn encrypt_decrypt_roundtrip() {
        let mut key_bytes = [0u8; 32];
        rand::rng().fill_bytes(&mut key_bytes);
        let key = Key::from(key_bytes);

        let mut secrets = HashMap::new();
        secrets.insert("token".to_owned(), "super-secret".to_owned());
        let json = serde_json::to_vec(&secrets).unwrap();

        let encrypted = encrypt_store(&key, &json).unwrap();
        let decrypted = decrypt_store(&key, &encrypted).unwrap();
        assert_eq!(decrypted, secrets);
    }

    #[test]
    fn decrypt_with_wrong_key_fails() {
        let mut key_a = [0u8; 32];
        let mut key_b = [0u8; 32];
        rand::rng().fill_bytes(&mut key_a);
        rand::rng().fill_bytes(&mut key_b);

        let key = Key::from(key_a);
        let json = serde_json::to_vec(&HashMap::<String, String>::new()).unwrap();
        let encrypted = encrypt_store(&key, &json).unwrap();

        let wrong_key = Key::from(key_b);
        assert!(decrypt_store(&wrong_key, &encrypted).is_err());
    }

    #[test]
    fn decrypt_rejects_truncated_data() {
        assert!(decrypt_store(&Key::from([0u8; 32]), &[0u8; 10]).is_err());
    }

    #[test]
    fn flush_creates_parent_dirs() {
        let mut key_bytes = [0u8; 32];
        rand::rng().fill_bytes(&mut key_bytes);
        let (dir, key) = temp_store(key_bytes);

        let nested = dir.join("a").join("b");
        let store = SecretStore {
            secrets: HashMap::new(),
            key,
            path: nested.join(STORE_FILENAME),
        };

        flush(&store).unwrap();
        assert!(store.path.exists());

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn set_empty_value_removes_entry() {
        let mut key_bytes = [0u8; 32];
        rand::rng().fill_bytes(&mut key_bytes);
        let (dir, key) = temp_store(key_bytes);

        let path = dir.join(STORE_FILENAME);
        let mut store = SecretStore {
            secrets: HashMap::from([("handle".to_owned(), "value".to_owned())]),
            key,
            path: path.clone(),
        };

        flush(&store).unwrap();
        let data = fs::read(&path).unwrap();
        let loaded = decrypt_store(&store.key, &data).unwrap();
        assert_eq!(loaded.get("handle").unwrap(), "value");

        store.secrets.remove("handle");
        flush(&store).unwrap();
        let data = fs::read(&path).unwrap();
        let loaded = decrypt_store(&store.key, &data).unwrap();
        assert!(loaded.get("handle").is_none());

        fs::remove_dir_all(&dir).ok();
    }
}
