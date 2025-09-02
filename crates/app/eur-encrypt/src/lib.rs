use base64::prelude::*;
use eur_secret::{self, Sensitive, secret};
use orion::{
    aead,
    kdf::{Password, Salt, derive_key},
};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use tracing::error;
use zeroize::{Zeroize, ZeroizeOnDrop};

mod error;

pub use error::{EncryptError, EncryptResult};

pub const USER_MAIN_KEY_HANDLE: &str = "USER_MAIN_KEY_HANDLE";

#[derive(Zeroize, ZeroizeOnDrop, Clone, Debug, Serialize, Deserialize)]
pub struct MainKey(pub [u8; 32]);

impl From<MainKey> for Password {
    fn from(value: MainKey) -> Self {
        Password::from_slice(&value.0).expect("Failed to create password")
    }
}

impl MainKey {
    pub fn new() -> EncryptResult<Self> {
        if let Ok(Some(key)) = secret::retrieve(USER_MAIN_KEY_HANDLE, secret::Namespace::Global) {
            let decoded = BASE64_STANDARD
                .decode(key.0)
                .map_err(EncryptError::Base64DecodeError)?;
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
    pub fn derive_fek(&self, salt: &Salt) -> EncryptResult<aead::SecretKey> {
        derive_key(&self.clone().into(), salt, 3, 1 << 16, 32).map_err(|e| {
            error!("Failed to derive key: {}", e);
            EncryptError::CryptoError(e)
        })
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
        EncryptError::KeyError(e)
    })?;

    Ok(MainKey(mk))
}
