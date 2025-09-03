use std::{
    fs::File,
    io::{Read, Write},
};

use base64::prelude::*;
use chacha20poly1305::{
    Key, XChaCha20Poly1305, XNonce,
    aead::{Aead, KeyInit, Payload},
};
use eur_secret::{self, Sensitive, secret};
// use orion::{
//     aead,
//     kdf::{Password, Salt, derive_key},
// };
use hkdf::Hkdf;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use tracing::error;
use zeroize::{Zeroize, ZeroizeOnDrop};

mod error;

pub use error::{EncryptError, EncryptResult};

const MAGIC: &[u8; 8] = b"EURFILE\x01";
const VERSION: u8 = 1;

pub const USER_MAIN_KEY_HANDLE: &str = "USER_MAIN_KEY_HANDLE";

#[derive(Zeroize, ZeroizeOnDrop, Clone, Debug, Serialize, Deserialize)]
pub struct MainKey(pub [u8; 32]);

#[repr(u8)]
pub enum AeadAlg {
    XChaCha20Poly1305 = 1,
}

pub struct FileHeader {
    pub version: u8,
    pub tag: String,
    pub salt: [u8; 32],
    pub nonce: [u8; 24],
}

// impl From<MainKey> for Password {
//     fn from(value: MainKey) -> Self {
//         Password::from_slice(&value.0).expect("Failed to create password")
//     }
// }

impl MainKey {
    pub fn new() -> EncryptResult<Self> {
        if let Ok(Some(key)) = secret::retrieve(USER_MAIN_KEY_HANDLE, secret::Namespace::Global) {
            let decoded = BASE64_STANDARD
                .decode(key.0)
                .map_err(EncryptError::Base64Decode)?;
            let key: [u8; 32] = decoded
                .try_into()
                .map_err(|_| EncryptError::InvalidKeyLength)?;
            Ok(MainKey(key))
        } else {
            generate_new_main_key()
        }
    }
}

impl Default for MainKey {
    fn default() -> Self {
        Self::new().expect("Failed to generate default main key")
    }
}

impl MainKey {
    pub fn derive_fek(&self, salt: &[u8; 32]) -> EncryptResult<Key> {
        let hk = Hkdf::<Sha256>::new(Some(salt), &self.0);
        let mut out = [0u8; 32];
        hk.expand(MAGIC, &mut out).map_err(|e| {
            error!("Failed to derive FEK: {}", e);
            EncryptError::InvalidKeyLength
        })?;
        Ok(Key::from(out))
    }
}

pub fn generate_new_main_key() -> EncryptResult<MainKey> {
    let mut mk = [0u8; 32];
    rand::rng().fill_bytes(&mut mk);

    let encoded = BASE64_STANDARD.encode(mk);
    secret::persist(
        USER_MAIN_KEY_HANDLE,
        &Sensitive(encoded),
        secret::Namespace::Global,
    )
    .map_err(|e| {
        error!("Failed to persist main key: {}", e);
        EncryptError::Key(e)
    })?;

    Ok(MainKey(mk))
}

pub async fn encrypt_file_contents(
    mk: &MainKey,
    bytes: &[u8],
    tag: &str,
) -> EncryptResult<Vec<u8>> {
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
                msg: &bytes,
                aad: &header,
            },
        )
        .map_err(|e| EncryptError::Encryption(e))?;

    let mut out = Vec::with_capacity(header.len() + ciphertext.len());
    out.extend_from_slice(&header);
    out.extend_from_slice(&ciphertext);

    Ok(out)
}

pub fn build_header(tag: &str, salt: &[u8; 32], nonce: &[u8; 24]) -> EncryptResult<Vec<u8>> {
    let tag_bytes = tag.as_bytes();
    if tag_bytes.len() > u16::MAX as usize {
        return Err(EncryptError::Format("Tag too long".to_string()));
    }

    let mut hdr = Vec::with_capacity(4 + 1 + 2 + tag_bytes.len() + 32 + 24);
    hdr.extend_from_slice(MAGIC);
    hdr.push(VERSION);
    hdr.extend_from_slice(&(tag_bytes.len() as u16).to_be_bytes());
    hdr.extend_from_slice(tag_bytes);
    hdr.extend_from_slice(salt);
    hdr.extend_from_slice(nonce);

    Ok(hdr)
}

pub fn parse_header(buf: &[u8]) -> EncryptResult<FileHeader> {
    let min = 4 + 1 + 2 + 32 + 24;
    if buf.len() < min {
        return Err(EncryptError::Format("Header too short".to_string()));
    }
    if &buf[0..4] != MAGIC {
        return Err(EncryptError::Format("Invalid magic number".to_string()));
    }
    let version = buf[4];
    if version != VERSION {
        return Err(EncryptError::Format("Invalid version".to_string()));
    }
    let tag_len = u16::from_be_bytes([buf[5], buf[6]]);
    let tag_len_size = tag_len as usize;
    let need = 4 + 1 + 2 + tag_len_size + 32 + 24;
    if buf.len() < need {
        return Err(EncryptError::Format(
            "Header too short with tag".to_string(),
        ));
    }
    let tag = std::str::from_utf8(&buf[7..7 + tag_len_size])
        .map_err(|_| EncryptError::Format("Invalid tag".to_string()))?;
    let mut salt = [0u8; 32];
    salt.copy_from_slice(&buf[7 + tag_len_size..7 + tag_len_size + 32]);
    let mut nonce = [0u8; 24];
    let nstart = 7 + tag_len_size + 32;
    nonce.copy_from_slice(&buf[nstart..nstart + 24]);

    Ok(FileHeader {
        version,
        tag: tag.to_string(),
        salt,
        nonce,
    })
}
