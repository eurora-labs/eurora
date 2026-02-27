use chacha20poly1305::{
    Key, XChaCha20Poly1305, XNonce,
    aead::{Aead, KeyInit},
};
use rand::TryRngCore;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("PKCE_ENCRYPTION_KEY environment variable not set")]
    MissingEncryptionKey,

    #[error("Invalid encryption key length: expected 32 bytes (64 hex chars), got {0}")]
    InvalidKeyLength(usize),

    #[error("Failed to decode encryption key from hex: {0}")]
    HexDecodeError(#[from] hex::FromHexError),

    #[error("Encryption failed: {0}")]
    EncryptionFailed(String),

    #[error("Decryption failed: {0}")]
    DecryptionFailed(String),

    #[error("Invalid encrypted data format")]
    InvalidFormat,
}

const NONCE_SIZE: usize = 24;

fn parse_encryption_key() -> Result<Key, CryptoError> {
    let key_hex =
        std::env::var("PKCE_ENCRYPTION_KEY").map_err(|_| CryptoError::MissingEncryptionKey)?;

    let key_bytes = hex::decode(&key_hex)?;

    if key_bytes.len() != 32 {
        return Err(CryptoError::InvalidKeyLength(key_bytes.len()));
    }

    let mut key_array = [0u8; 32];
    key_array.copy_from_slice(&key_bytes);

    Ok(Key::from(key_array))
}

#[cfg(not(test))]
fn get_encryption_key() -> Result<Key, CryptoError> {
    static CACHED_KEY: std::sync::OnceLock<Key> = std::sync::OnceLock::new();
    if let Some(key) = CACHED_KEY.get() {
        return Ok(*key);
    }
    let key = parse_encryption_key()?;
    let _ = CACHED_KEY.set(key);
    Ok(key)
}

#[cfg(test)]
fn get_encryption_key() -> Result<Key, CryptoError> {
    parse_encryption_key()
}

pub fn encrypt_sensitive_string(verifier: &str) -> Result<Vec<u8>, CryptoError> {
    let key = get_encryption_key()?;
    let cipher = XChaCha20Poly1305::new(&key);

    let mut nonce_bytes = [0u8; NONCE_SIZE];
    rand::rngs::OsRng
        .try_fill_bytes(&mut nonce_bytes)
        .map_err(|e| CryptoError::EncryptionFailed(format!("Failed to generate nonce: {e}")))?;
    let nonce = XNonce::from_slice(&nonce_bytes);

    let ciphertext = cipher.encrypt(nonce, verifier.as_bytes()).map_err(|e| {
        tracing::error!("PKCE verifier encryption failed: {}", e);
        CryptoError::EncryptionFailed(e.to_string())
    })?;

    let mut encrypted = Vec::with_capacity(NONCE_SIZE + ciphertext.len());
    encrypted.extend_from_slice(&nonce_bytes);
    encrypted.extend_from_slice(&ciphertext);

    Ok(encrypted)
}

pub fn decrypt_sensitive_string(encrypted: &[u8]) -> Result<String, CryptoError> {
    if encrypted.len() < NONCE_SIZE {
        return Err(CryptoError::InvalidFormat);
    }

    let key = get_encryption_key()?;
    let cipher = XChaCha20Poly1305::new(&key);

    let (nonce_bytes, ciphertext) = encrypted.split_at(NONCE_SIZE);
    let nonce = XNonce::from_slice(nonce_bytes);

    let plaintext = cipher.decrypt(nonce, ciphertext).map_err(|e| {
        tracing::error!("PKCE verifier decryption failed: {}", e);
        CryptoError::DecryptionFailed(e.to_string())
    })?;

    String::from_utf8(plaintext)
        .map_err(|_| CryptoError::DecryptionFailed("Invalid UTF-8 in decrypted data".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_MUTEX: Mutex<()> = Mutex::new(());

    const TEST_KEY: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let _lock = ENV_MUTEX.lock().unwrap();

        // SAFETY: This is a test-only operation protected by mutex
        unsafe {
            std::env::set_var("PKCE_ENCRYPTION_KEY", TEST_KEY);
        }

        let verifier = "test_pkce_verifier_12345";

        let encrypted = encrypt_sensitive_string(verifier).expect("Encryption should succeed");
        assert!(encrypted.len() > NONCE_SIZE);

        let decrypted = decrypt_sensitive_string(&encrypted).expect("Decryption should succeed");
        assert_eq!(verifier, decrypted);

        // SAFETY: This is a test-only operation protected by mutex
        unsafe {
            std::env::remove_var("PKCE_ENCRYPTION_KEY");
        }
    }

    #[test]
    fn test_decrypt_tampered_data_fails() {
        let _lock = ENV_MUTEX.lock().unwrap();

        // SAFETY: This is a test-only operation protected by mutex
        unsafe {
            std::env::set_var("PKCE_ENCRYPTION_KEY", TEST_KEY);
        }

        let verifier = "test_pkce_verifier";
        let mut encrypted = encrypt_sensitive_string(verifier).expect("Encryption should succeed");

        if let Some(byte) = encrypted.last_mut() {
            *byte ^= 0xFF;
        }

        let result = decrypt_sensitive_string(&encrypted);
        assert!(result.is_err());

        // SAFETY: This is a test-only operation protected by mutex
        unsafe {
            std::env::remove_var("PKCE_ENCRYPTION_KEY");
        }
    }

    #[test]
    fn test_missing_key_returns_error() {
        let _lock = ENV_MUTEX.lock().unwrap();

        // SAFETY: This is a test-only operation protected by mutex
        unsafe {
            std::env::remove_var("PKCE_ENCRYPTION_KEY");
        }

        let result = encrypt_sensitive_string("test");
        assert!(matches!(result, Err(CryptoError::MissingEncryptionKey)));
    }

    #[test]
    fn test_invalid_key_length_returns_error() {
        let _lock = ENV_MUTEX.lock().unwrap();

        // SAFETY: This is a test-only operation protected by mutex
        unsafe {
            std::env::set_var("PKCE_ENCRYPTION_KEY", "0123456789abcdef"); // Too short
        }

        let result = encrypt_sensitive_string("test");
        assert!(matches!(result, Err(CryptoError::InvalidKeyLength(_))));

        // SAFETY: This is a test-only operation protected by mutex
        unsafe {
            std::env::remove_var("PKCE_ENCRYPTION_KEY");
        }
    }
}
