use orion::{
    aead,
    kdf::{Password, Salt, derive_key},
};
use rand::RngCore;
use tracing::error;

use crate::ActivityResult;
use crate::SaveableAsset;

pub fn derive_fek(mk: &[u8; 32], salt: &[u8; 32]) -> aead::SecretKey {
    let key = derive_key(
        &Password::from_slice(mk).unwrap(),
        &Salt::from_slice(salt).unwrap(),
        3,
        1 << 16,
        32,
    );
    if let Err(e) = key {
        error!("Failed to derive key: {}", e);
        return aead::SecretKey::from_slice(&[0u8; 32]).unwrap();
    }
    key.unwrap()
}

pub async fn encrypt_bytes(bytes: &[u8]) -> ActivityResult<Vec<u8>> {
    let mut placeholder_mk = [0u8; 32];
    rand::rng().fill_bytes(&mut placeholder_mk);
    let mut salt = [0u8; 32];
    rand::rng().fill_bytes(&mut salt);
    let key = derive_fek(&placeholder_mk, &salt);
    let blob = aead::seal(&key, &bytes).unwrap();
    let mut out = Vec::with_capacity(32 + blob.len());
    out.extend_from_slice(&salt);
    out.extend_from_slice(&blob);

    Ok(out)
}
