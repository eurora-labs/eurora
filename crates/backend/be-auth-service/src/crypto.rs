//! Cryptographic utilities for the authentication service.
//!
//! This module provides encryption and decryption functionality for sensitive data
//! that needs to be stored securely, such as PKCE verifiers.

use chacha20poly1305::{
    Key, XChaCha20Poly1305, XNonce,
    aead::{Aead, KeyInit},
};
use rand::RngCore;
use thiserror::Error;

/// Errors that can occur during cryptographic operations.
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

/// Encrypted data format: [nonce (24 bytes)][ciphertext (variable)]
const NONCE_SIZE: usize = 24;

/// Get the encryption key from environment variable.
///
/// The key should be a 32-byte (256-bit) key encoded as 64 hex characters.
/// This can be generated with: `openssl rand -hex 32`
fn get_encryption_key() -> Result<Key, CryptoError> {
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

/// Encrypt a sensitive string for secure storage.
///
/// Uses XChaCha20-Poly1305 authenticated encryption with a random nonce.
/// The nonce is prepended to the ciphertext for storage.
///
/// # Arguments
///
/// * `verifier` - The plaintext string to encrypt
///
/// # Returns
///
/// Returns the encrypted bytes as `[nonce (24 bytes)][ciphertext]`
///
/// # Errors
///
/// Returns a `CryptoError` if:
/// - The encryption key is not set or invalid
/// - Encryption fails
pub fn encrypt_sensitive_string(verifier: &str) -> Result<Vec<u8>, CryptoError> {
    let key = get_encryption_key()?;
    let cipher = XChaCha20Poly1305::new(&key);

    // Generate a random nonce
    let mut nonce_bytes = [0u8; NONCE_SIZE];
    rand::rng().fill_bytes(&mut nonce_bytes);
    let nonce = XNonce::from_slice(&nonce_bytes);

    // Encrypt the verifier
    let ciphertext = cipher.encrypt(nonce, verifier.as_bytes()).map_err(|e| {
        tracing::error!("PKCE verifier encryption failed: {}", e);
        CryptoError::EncryptionFailed(e.to_string())
    })?;

    // Prepend nonce to ciphertext
    let mut encrypted = Vec::with_capacity(NONCE_SIZE + ciphertext.len());
    encrypted.extend_from_slice(&nonce_bytes);
    encrypted.extend_from_slice(&ciphertext);

    Ok(encrypted)
}

/// Decrypt a sensitive string from encrypted storage.
///
/// # Arguments
///
/// * `encrypted` - The encrypted bytes as `[nonce (24 bytes)][ciphertext]`
///
/// # Returns
///
/// Returns the decrypted plaintext string
///
/// # Errors
///
/// Returns a `CryptoError` if:
/// - The encryption key is not set or invalid
/// - The encrypted data format is invalid
/// - Decryption fails (e.g., tampered data)
pub fn decrypt_sensitive_string(encrypted: &[u8]) -> Result<String, CryptoError> {
    if encrypted.len() < NONCE_SIZE {
        return Err(CryptoError::InvalidFormat);
    }

    let key = get_encryption_key()?;
    let cipher = XChaCha20Poly1305::new(&key);

    // Extract nonce and ciphertext
    let (nonce_bytes, ciphertext) = encrypted.split_at(NONCE_SIZE);
    let nonce = XNonce::from_slice(nonce_bytes);

    // Decrypt
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

    // Mutex to serialize tests that modify the environment variable
    static ENV_MUTEX: Mutex<()> = Mutex::new(());

    const TEST_KEY: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let _lock = ENV_MUTEX.lock().unwrap();

        // Set a test encryption key
        // SAFETY: This is a test-only operation protected by mutex
        unsafe {
            std::env::set_var("PKCE_ENCRYPTION_KEY", TEST_KEY);
        }

        let verifier = "test_pkce_verifier_12345";

        // Encrypt
        let encrypted = encrypt_sensitive_string(verifier).expect("Encryption should succeed");

        // Verify encrypted data has correct format (nonce + ciphertext)
        assert!(encrypted.len() > NONCE_SIZE);

        // Decrypt
        let decrypted = decrypt_sensitive_string(&encrypted).expect("Decryption should succeed");

        assert_eq!(verifier, decrypted);

        // Clean up
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

        // Tamper with the ciphertext
        if let Some(byte) = encrypted.last_mut() {
            *byte ^= 0xFF;
        }

        // Decryption should fail
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
