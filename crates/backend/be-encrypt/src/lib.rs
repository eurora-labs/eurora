//! Backend encryption for assets at rest.
//!
//! Uses XChaCha20-Poly1305 (AEAD) with HKDF-SHA256 key derivation,
//! producing a file format byte-compatible with `euro-encrypt`.
//!
//! ## File Format
//!
//! ```text
//! [MAGIC:8][VERSION:1][TAG_LEN:2][TAG:variable][SALT:32][NONCE:24][CIPHERTEXT:variable]
//! ```

mod error;

pub use error::{EncryptError, EncryptResult};

use base64::prelude::*;
use chacha20poly1305::{
    Key, XChaCha20Poly1305, XNonce,
    aead::{Aead, KeyInit, Payload},
};
use hkdf::Hkdf;
use rand::RngCore;
use sha2::Sha256;
use tracing::error;
use zeroize::{Zeroize, ZeroizeOnDrop};

/// Magic bytes identifying encrypted files (shared with `euro-encrypt`).
pub const MAGIC: &[u8; 8] = b"EURFILES";

/// Current file format version.
const VERSION: u8 = 1;

/// HKDF info parameter for domain separation (shared with `euro-encrypt`).
const FEK_INFO: &[u8] = b"EURORA-FEK-v1";

/// Main encryption key (32 bytes). Automatically zeroized on drop.
#[derive(Zeroize, ZeroizeOnDrop, Clone)]
pub struct MainKey(pub [u8; 32]);

impl std::fmt::Debug for MainKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("MainKey([REDACTED])")
    }
}

impl MainKey {
    /// Generate a new random key.
    pub fn generate() -> EncryptResult<Self> {
        let mut bytes = [0u8; 32];
        rand::rng().fill_bytes(&mut bytes);
        let key = MainKey(bytes);
        bytes.zeroize();
        key.validate()?;
        Ok(key)
    }

    /// Decode a key from a base64 string.
    pub fn from_base64(s: &str) -> EncryptResult<Self> {
        let decoded = BASE64_STANDARD
            .decode(s.trim())
            .map_err(EncryptError::Base64Decode)?;
        let bytes: [u8; 32] = decoded
            .try_into()
            .map_err(|_| EncryptError::InvalidKeyLength)?;
        let key = MainKey(bytes);
        key.validate()?;
        Ok(key)
    }

    /// Encode the key as a base64 string.
    pub fn to_base64(&self) -> String {
        BASE64_STANDARD.encode(self.0)
    }

    /// Validate key is not weak (all zeros or uniform bytes).
    pub fn validate(&self) -> EncryptResult<()> {
        if self.0.iter().all(|&b| b == 0) {
            return Err(EncryptError::Format(
                "Main key cannot be all zeros".to_string(),
            ));
        }
        let first = self.0[0];
        if self.0.iter().all(|&b| b == first) {
            return Err(EncryptError::Format(
                "Main key has insufficient entropy".to_string(),
            ));
        }
        Ok(())
    }

    /// Derive a per-file encryption key via HKDF-SHA256.
    fn derive_fek(&self, salt: &[u8; 32]) -> EncryptResult<Key> {
        if salt.iter().all(|&b| b == 0) {
            return Err(EncryptError::Format("Salt cannot be all zeros".to_string()));
        }

        let hk = Hkdf::<Sha256>::new(Some(salt), &self.0);
        let mut out = [0u8; 32];
        hk.expand(FEK_INFO, &mut out).map_err(|e| {
            error!("Failed to derive FEK: {}", e);
            EncryptError::Format(format!("FEK derivation failed: {}", e))
        })?;

        let key = Key::from(out);
        out.zeroize();
        Ok(key)
    }
}

/// Encrypt plaintext bytes, returning the full encrypted blob (header + ciphertext).
pub fn encrypt(mk: &MainKey, plaintext: &[u8], tag: &str) -> EncryptResult<Vec<u8>> {
    if tag.is_empty() {
        return Err(EncryptError::Format("Tag cannot be empty".to_string()));
    }
    if tag.len() > 1024 {
        return Err(EncryptError::Format("Tag too long".to_string()));
    }
    if !tag.chars().all(|c| c.is_ascii() && !c.is_control()) {
        return Err(EncryptError::Format(
            "Tag contains invalid characters".to_string(),
        ));
    }
    if plaintext.is_empty() {
        return Err(EncryptError::Format(
            "Cannot encrypt empty data".to_string(),
        ));
    }

    let mut salt = [0u8; 32];
    rand::rng().fill_bytes(&mut salt);
    let mut nonce = [0u8; 24];
    rand::rng().fill_bytes(&mut nonce);

    let header = build_header(tag, &salt, &nonce)?;

    let key = mk.derive_fek(&salt)?;
    let cipher = XChaCha20Poly1305::new(&key);
    let xnonce = XNonce::from_slice(&nonce);

    let ciphertext = cipher
        .encrypt(
            xnonce,
            Payload {
                msg: plaintext,
                aad: &header,
            },
        )
        .map_err(EncryptError::Encryption)?;

    let mut out = Vec::with_capacity(header.len() + ciphertext.len());
    out.extend_from_slice(&header);
    out.extend_from_slice(&ciphertext);

    salt.zeroize();
    nonce.zeroize();

    Ok(out)
}

/// Decrypt an encrypted blob (header + ciphertext), returning the plaintext bytes.
pub fn decrypt(mk: &MainKey, data: &[u8]) -> EncryptResult<Vec<u8>> {
    let header = parse_header(data)?;

    if header.version != VERSION {
        return Err(EncryptError::Format(format!(
            "Unsupported version: {}",
            header.version
        )));
    }

    let key = mk.derive_fek(&header.salt)?;
    let cipher = XChaCha20Poly1305::new(&key);
    let xnonce = XNonce::from_slice(&header.nonce);

    let header_len = MAGIC.len() + 1 + 2 + header.tag.len() + 32 + 24;

    if data.len() <= header_len {
        return Err(EncryptError::Format(
            "Data too short to contain encrypted content".to_string(),
        ));
    }

    let plaintext = cipher
        .decrypt(
            xnonce,
            Payload {
                msg: &data[header_len..],
                aad: &data[..header_len],
            },
        )
        .map_err(EncryptError::Encryption)?;

    Ok(plaintext)
}

/// Check whether a byte slice starts with the encrypted file magic bytes.
pub fn is_encrypted(bytes: &[u8]) -> bool {
    bytes.len() >= MAGIC.len() && bytes[..MAGIC.len()] == *MAGIC
}

fn build_header(tag: &str, salt: &[u8; 32], nonce: &[u8; 24]) -> EncryptResult<Vec<u8>> {
    if salt.iter().all(|&b| b == 0) {
        return Err(EncryptError::Format("Salt cannot be all zeros".to_string()));
    }
    if nonce.iter().all(|&b| b == 0) {
        return Err(EncryptError::Format(
            "Nonce cannot be all zeros".to_string(),
        ));
    }

    let tag_bytes = tag.as_bytes();
    let mut hdr = Vec::with_capacity(MAGIC.len() + 1 + 2 + tag_bytes.len() + 32 + 24);
    hdr.extend_from_slice(MAGIC);
    hdr.push(VERSION);
    hdr.extend_from_slice(&(tag_bytes.len() as u16).to_be_bytes());
    hdr.extend_from_slice(tag_bytes);
    hdr.extend_from_slice(salt);
    hdr.extend_from_slice(nonce);

    Ok(hdr)
}

struct FileHeader {
    version: u8,
    tag: String,
    salt: [u8; 32],
    nonce: [u8; 24],
}

fn parse_header(buf: &[u8]) -> EncryptResult<FileHeader> {
    let min_len = MAGIC.len() + 1 + 2 + 32 + 24;
    if buf.len() < min_len {
        return Err(EncryptError::Format("Header too short".to_string()));
    }

    // Constant-time magic comparison
    let mut magic_match = true;
    for (a, b) in buf[..MAGIC.len()].iter().zip(MAGIC.iter()) {
        if a != b {
            magic_match = false;
        }
    }
    if !magic_match {
        return Err(EncryptError::Format("Invalid magic number".to_string()));
    }

    let version = buf[8];
    if version != VERSION {
        return Err(EncryptError::Format(format!(
            "Unsupported version: {}",
            version
        )));
    }

    let tag_len = u16::from_be_bytes([buf[9], buf[10]]) as usize;
    if tag_len > 1024 {
        return Err(EncryptError::Format("Tag length too large".to_string()));
    }

    let total_header_len = 8 + 1 + 2 + tag_len + 32 + 24;
    if buf.len() < total_header_len {
        return Err(EncryptError::Format(
            "Header too short for tag length".to_string(),
        ));
    }

    let tag_start = 11;
    let tag_end = tag_start + tag_len;
    let tag = std::str::from_utf8(&buf[tag_start..tag_end])
        .map_err(|_| EncryptError::Format("Invalid UTF-8 in tag".to_string()))?;

    if !tag.chars().all(|c| c.is_ascii() && !c.is_control()) {
        return Err(EncryptError::Format(
            "Tag contains invalid characters".to_string(),
        ));
    }

    let mut salt = [0u8; 32];
    salt.copy_from_slice(&buf[tag_end..tag_end + 32]);

    let mut nonce = [0u8; 24];
    let nonce_start = tag_end + 32;
    nonce.copy_from_slice(&buf[nonce_start..nonce_start + 24]);

    Ok(FileHeader {
        version,
        tag: tag.to_string(),
        salt,
        nonce,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_key() -> MainKey {
        let mut bytes = [0u8; 32];
        rand::rng().fill_bytes(&mut bytes);
        MainKey(bytes)
    }

    #[test]
    fn encrypt_decrypt_roundtrip() {
        let key = test_key();
        let plaintext = b"Hello, World! This is test data.";

        let encrypted = encrypt(&key, plaintext, "test").unwrap();
        assert!(is_encrypted(&encrypted));

        let decrypted = decrypt(&key, &encrypted).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn encrypt_decrypt_json_roundtrip() {
        let key = test_key();
        let data = serde_json::json!({"name": "test", "value": 42});
        let json_bytes = serde_json::to_vec(&data).unwrap();

        let encrypted = encrypt(&key, &json_bytes, "asset").unwrap();
        let decrypted = decrypt(&key, &encrypted).unwrap();

        let parsed: serde_json::Value = serde_json::from_slice(&decrypted).unwrap();
        assert_eq!(parsed, data);
    }

    #[test]
    fn is_encrypted_check() {
        assert!(!is_encrypted(b""));
        assert!(!is_encrypted(b"SHORT"));
        assert!(!is_encrypted(b"NOT_ENCRYPTED_DATA"));
        assert!(is_encrypted(b"EURFILES_and_more_data"));
    }

    #[test]
    fn key_validation() {
        let weak = MainKey([0u8; 32]);
        assert!(weak.validate().is_err());

        let uniform = MainKey([0x42u8; 32]);
        assert!(uniform.validate().is_err());

        let valid = test_key();
        assert!(valid.validate().is_ok());
    }

    #[test]
    fn key_base64_roundtrip() {
        let key = test_key();
        let encoded = key.to_base64();
        let decoded = MainKey::from_base64(&encoded).unwrap();
        assert_eq!(key.0, decoded.0);
    }

    #[test]
    fn wrong_key_fails_decryption() {
        let key1 = test_key();
        let key2 = test_key();

        let encrypted = encrypt(&key1, b"secret data", "test").unwrap();
        assert!(decrypt(&key2, &encrypted).is_err());
    }

    #[test]
    fn invalid_tag_rejected() {
        let key = test_key();

        assert!(encrypt(&key, b"data", "").is_err());
        assert!(encrypt(&key, b"data", "has\x00null").is_err());
        assert!(encrypt(&key, b"", "tag").is_err());
    }

    #[test]
    fn corrupted_data_fails() {
        let key = test_key();
        let mut encrypted = encrypt(&key, b"data", "test").unwrap();

        // Flip a byte in the ciphertext
        let last = encrypted.len() - 1;
        encrypted[last] ^= 0xFF;

        assert!(decrypt(&key, &encrypted).is_err());
    }

    #[test]
    fn non_encrypted_data_fails_decrypt() {
        let key = test_key();
        assert!(decrypt(&key, b"plain text data").is_err());
    }
}
