use eur_encrypt::MainKey;
use orion::{
    aead,
    kdf::{Password, Salt, derive_key},
};
use rand::RngCore;

use crate::ActivityResult;

pub async fn encrypt_bytes(mk: &MainKey, bytes: &[u8]) -> ActivityResult<Vec<u8>> {
    let mut salt = [0u8; 32];
    rand::rng().fill_bytes(&mut salt);

    let key = mk.derive_fek(&Salt::from_slice(&salt).unwrap()).unwrap();
    let blob = aead::seal(&key, bytes).unwrap();
    let mut out = Vec::with_capacity(32 + blob.len());
    out.extend_from_slice(&salt);
    out.extend_from_slice(&blob);

    Ok(out)
}
