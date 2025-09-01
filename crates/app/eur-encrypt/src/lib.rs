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
        Password::from_slice(&value.0).unwrap()
    }
}

impl MainKey {
    pub fn new() -> Self {
        if let Ok(key) = secret::retrieve(USER_MAIN_KEY_HANDLE, secret::Namespace::Global)
            && let Some(key) = key
        {
            let key: [u8; 32] = BASE64_STANDARD.decode(key.0).unwrap().try_into().unwrap();
            MainKey(key)
        } else {
            generate_new_main_key().unwrap()
        }
    }
}

impl Default for MainKey {
    fn default() -> Self {
        Self::new()
    }
}

impl MainKey {
    pub fn derive_fek(&self, salt: &Salt) -> EncryptResult<aead::SecretKey> {
        let key = derive_key(&self.clone().into(), salt, 3, 1 << 16, 32);
        if let Err(e) = key {
            error!("Failed to derive key: {}", e);
            return aead::SecretKey::from_slice(&[0u8; 32]).map_err(EncryptError::CryptoError);
        }
        key.map_err(EncryptError::CryptoError)
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
    );

    Ok(MainKey(mk))
}
