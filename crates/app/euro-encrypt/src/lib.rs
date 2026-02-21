use std::{fs, path::Path};

use base64::prelude::*;
use chacha20poly1305::{
    Key, XChaCha20Poly1305, XNonce,
    aead::{Aead, KeyInit, Payload},
};
use euro_secret::{self, Sensitive, secret};
use hkdf::Hkdf;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

mod error;

pub use error::{EncryptError, EncryptResult};

const MAGIC: &[u8; 8] = b"EURFILES";
const VERSION: u8 = 1;
pub const USER_MAIN_KEY_HANDLE: &str = "USER_MAIN_KEY_HANDLE";

#[derive(Zeroize, ZeroizeOnDrop, Clone, Serialize, Deserialize)]
pub struct MainKey(pub [u8; 32]);

impl std::fmt::Debug for MainKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("MainKey([REDACTED 32 bytes])")
    }
}

#[repr(u8)]
pub enum AeadAlg {
    XChaCha20Poly1305 = 1,
}

#[derive(Debug)]
pub struct FileHeader {
    pub version: u8,
    pub tag: String,
    pub salt: [u8; 32],
    pub nonce: [u8; 24],
}

impl MainKey {
    pub fn new() -> EncryptResult<Self> {
        if let Ok(Some(key)) = secret::retrieve(USER_MAIN_KEY_HANDLE, secret::Namespace::Global) {
            let decoded = BASE64_STANDARD
                .decode(key.0)
                .map_err(EncryptError::Base64Decode)?;
            let key_bytes: [u8; 32] = decoded
                .try_into()
                .map_err(|_| EncryptError::InvalidKeyLength)?;

            let main_key = MainKey(key_bytes);
            main_key.validate()?;
            Ok(main_key)
        } else {
            generate_new_main_key()
        }
    }
}

impl Default for MainKey {
    fn default() -> Self {
        // This is a fallback that should not be used in production
        let mut temp_key = [0u8; 32];
        rand::rng().fill_bytes(&mut temp_key);
        let key = MainKey(temp_key);
        temp_key.zeroize();
        key
    }
}

impl MainKey {
    pub fn derive_fek(&self, salt: &[u8; 32]) -> EncryptResult<Key> {
        if salt.iter().all(|&b| b == 0) {
            return Err(EncryptError::Format("Salt cannot be all zeros".to_string()));
        }

        let hk = Hkdf::<Sha256>::new(Some(salt), &self.0);
        let mut out = [0u8; 32];

        // Domain separation so FEKs can't collide with other derived keys
        let info = b"EURORA-FEK-v1";
        hk.expand(info, &mut out).map_err(|e| {
            tracing::error!("Failed to derive FEK: {}", e);
            EncryptError::Format(format!("FEK derivation failed: {}", e))
        })?;

        let key = Key::from(out);
        out.zeroize();
        Ok(key)
    }

    pub fn validate(&self) -> EncryptResult<()> {
        if self.0.iter().all(|&b| b == 0) {
            return Err(EncryptError::Format(
                "Main key cannot be all zeros".to_string(),
            ));
        }

        let first_byte = self.0[0];
        if self.0.iter().all(|&b| b == first_byte) {
            return Err(EncryptError::Format(
                "Main key has insufficient entropy".to_string(),
            ));
        }

        Ok(())
    }
}

pub fn generate_new_main_key() -> EncryptResult<MainKey> {
    let mut mk = [0u8; 32];
    rand::rng().fill_bytes(&mut mk);

    let main_key = MainKey(mk);
    // Should never fail with proper RNG, but safety first
    main_key.validate()?;

    let encoded = BASE64_STANDARD.encode(mk);
    secret::persist(
        USER_MAIN_KEY_HANDLE,
        &Sensitive(encoded),
        secret::Namespace::Global,
    )
    .map_err(|e| {
        tracing::error!("Failed to persist main key: {}", e);
        EncryptError::Key(e)
    })?;

    mk.zeroize();
    Ok(main_key)
}

pub async fn load_file_and_header(path: &Path) -> EncryptResult<(FileHeader, Vec<u8>)> {
    let buf = fs::read(path).map_err(|e| {
        tracing::error!("Failed to read file: {}", e);
        EncryptError::Io(e)
    })?;

    let header = parse_header(&buf)?;
    Ok((header, buf))
}

pub async fn decrypt_file<T>(
    mk: &MainKey,
    header: FileHeader,
    mut bytes: Vec<u8>,
) -> EncryptResult<T>
where
    T: serde::de::DeserializeOwned,
{
    mk.validate()?;

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

    if bytes.len() <= header_len {
        return Err(EncryptError::Format(
            "File too short to contain encrypted data".to_string(),
        ));
    }

    let decrypted_bytes = Zeroizing::new(
        cipher
            .decrypt(
                xnonce,
                Payload {
                    msg: &bytes[header_len..],
                    aad: &bytes[..header_len],
                },
            )
            .map_err(EncryptError::Encryption)?,
    );

    bytes.zeroize();

    let val = serde_json::from_slice::<T>(&decrypted_bytes).map_err(|e| {
        tracing::error!("Failed to deserialize JSON: {}", e);
        EncryptError::Json(e)
    })?;

    Ok(val)
}

pub async fn load_encrypted_file<T>(mk: &MainKey, path: &Path) -> EncryptResult<T>
where
    T: serde::de::DeserializeOwned,
{
    let (header, bytes) = load_file_and_header(path).await?;
    decrypt_file::<T>(mk, header, bytes).await
}

pub async fn encrypt_file_contents(
    mk: &MainKey,
    bytes: &[u8],
    tag: &str,
) -> EncryptResult<Vec<u8>> {
    if tag.is_empty() {
        return Err(EncryptError::Format("Tag cannot be empty".to_string()));
    }
    if tag.len() > u16::MAX as usize {
        return Err(EncryptError::Format("Tag too long".to_string()));
    }
    if bytes.is_empty() {
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
                msg: bytes,
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

pub fn build_header(tag: &str, salt: &[u8; 32], nonce: &[u8; 24]) -> EncryptResult<Vec<u8>> {
    if tag.is_empty() {
        return Err(EncryptError::Format("Tag cannot be empty".to_string()));
    }

    let tag_bytes = tag.as_bytes();
    if tag_bytes.len() > u16::MAX as usize {
        return Err(EncryptError::Format("Tag too long".to_string()));
    }

    if tag_bytes.len() > 1024 {
        return Err(EncryptError::Format(
            "Tag length exceeds maximum allowed".to_string(),
        ));
    }

    if !tag.chars().all(|c| c.is_ascii() && !c.is_control()) {
        return Err(EncryptError::Format(
            "Tag contains invalid characters".to_string(),
        ));
    }

    if salt.iter().all(|&b| b == 0) {
        return Err(EncryptError::Format("Salt cannot be all zeros".to_string()));
    }

    if nonce.iter().all(|&b| b == 0) {
        return Err(EncryptError::Format(
            "Nonce cannot be all zeros".to_string(),
        ));
    }

    let mut hdr = Vec::with_capacity(MAGIC.len() + 1 + 2 + tag_bytes.len() + 32 + 24);
    hdr.extend_from_slice(MAGIC);
    hdr.push(VERSION);
    hdr.extend_from_slice(&(tag_bytes.len() as u16).to_be_bytes());
    hdr.extend_from_slice(tag_bytes);
    hdr.extend_from_slice(salt);
    hdr.extend_from_slice(nonce);

    Ok(hdr)
}

pub fn parse_header(buf: &[u8]) -> EncryptResult<FileHeader> {
    let min_header_len = MAGIC.len() + 1 + 2 + 32 + 24;
    if buf.len() < min_header_len {
        return Err(EncryptError::Format("Header too short".to_string()));
    }

    // Constant-time comparison to prevent timing attacks
    let mut magic_match = true;
    if buf.len() >= MAGIC.len() {
        for (a, b) in buf[0..MAGIC.len()].iter().zip(MAGIC.iter()) {
            if a != b {
                magic_match = false;
            }
        }
    } else {
        magic_match = false;
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

    let tag_len = u16::from_be_bytes([buf[9], buf[10]]);
    let tag_len_size = tag_len as usize;

    // Prevent integer overflow in total_header_len calculation
    if tag_len_size > 1024 {
        return Err(EncryptError::Format("Tag length too large".to_string()));
    }

    let total_header_len = 8 + 1 + 2 + tag_len_size + 32 + 24;
    if buf.len() < total_header_len {
        return Err(EncryptError::Format(
            "Header too short with tag".to_string(),
        ));
    }

    let tag_start = 11;
    let tag_end = tag_start + tag_len_size;
    let tag = std::str::from_utf8(&buf[tag_start..tag_end])
        .map_err(|_| EncryptError::Format("Invalid UTF-8 in tag".to_string()))?;

    if !tag.chars().all(|c| c.is_ascii() && !c.is_control()) {
        return Err(EncryptError::Format(
            "Tag contains invalid characters".to_string(),
        ));
    }

    let mut salt = [0u8; 32];
    let salt_start = tag_end;
    salt.copy_from_slice(&buf[salt_start..salt_start + 32]);

    let mut nonce = [0u8; 24];
    let nonce_start = salt_start + 32;
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

    #[tokio::test]
    async fn test_encrypt_decrypt_roundtrip() {
        let test_data = "Hello, World! This is a test message.";
        let tag = "test_file";

        let mut key_bytes = [0u8; 32];
        rand::rng().fill_bytes(&mut key_bytes);
        let main_key = MainKey(key_bytes);

        let json_bytes = serde_json::to_vec(&test_data).expect("JSON serialization should succeed");

        let encrypted = encrypt_file_contents(&main_key, &json_bytes, tag)
            .await
            .expect("Encryption should succeed");

        let header = parse_header(&encrypted).expect("Header parsing should succeed");
        assert_eq!(header.version, VERSION);
        assert_eq!(header.tag, tag);

        let decrypted: String = decrypt_file(&main_key, header, encrypted)
            .await
            .expect("Decryption should succeed");

        assert_eq!(decrypted, test_data);
    }

    #[test]
    fn test_main_key_validation() {
        let weak_key = MainKey([0u8; 32]);
        assert!(weak_key.validate().is_err());

        let same_byte_key = MainKey([0x42u8; 32]);
        assert!(same_byte_key.validate().is_err());

        let mut valid_key_bytes = [0u8; 32];
        rand::rng().fill_bytes(&mut valid_key_bytes);
        let valid_key = MainKey(valid_key_bytes);
        assert!(valid_key.validate().is_ok());
    }

    #[test]
    fn test_header_validation() {
        let mut invalid_header = vec![0u8; 100];
        invalid_header[0..8].copy_from_slice(b"INVALID!");
        assert!(parse_header(&invalid_header).is_err());

        let short_header = vec![0u8; 10];
        assert!(parse_header(&short_header).is_err());
    }

    #[tokio::test]
    async fn test_input_validation() {
        let mut key_bytes = [0u8; 32];
        rand::rng().fill_bytes(&mut key_bytes);
        let main_key = MainKey(key_bytes);

        let result = encrypt_file_contents(&main_key, b"test", "").await;
        assert!(result.is_err());

        let result = encrypt_file_contents(&main_key, b"", "test").await;
        assert!(result.is_err());

        let result = encrypt_file_contents(&main_key, b"test", "test\x00").await;
        assert!(result.is_err());
    }
}
