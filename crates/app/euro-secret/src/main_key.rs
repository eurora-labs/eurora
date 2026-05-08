use anyhow::{Context, Result, bail};
use base64::prelude::*;
use rand::Rng;
use secrecy::{ExposeSecret, SecretString};
use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

use crate::secret::{keyring_persist, keyring_retrieve};

const USER_MAIN_KEY_HANDLE: &str = "USER_MAIN_KEY_HANDLE";

#[derive(Zeroize, ZeroizeOnDrop, Clone)]
pub struct MainKey([u8; 32]);

impl std::fmt::Debug for MainKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("MainKey([REDACTED 32 bytes])")
    }
}

impl MainKey {
    pub fn new() -> Result<Self> {
        match keyring_retrieve(USER_MAIN_KEY_HANDLE).context("Cannot access OS keychain")? {
            Some(key) => {
                let decoded = BASE64_STANDARD
                    .decode(key.expose_secret())
                    .context("base64 decode of stored main key failed")?;
                let bytes: [u8; 32] = decoded
                    .try_into()
                    .map_err(|_| anyhow::anyhow!("stored main key has invalid length"))?;
                let main_key = MainKey(bytes);
                main_key.validate()?;
                Ok(main_key)
            }
            None => generate_new_main_key(),
        }
    }

    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        MainKey(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    fn validate(&self) -> Result<()> {
        if self.0.iter().all(|&b| b == 0) {
            bail!("Main key cannot be all zeros");
        }
        let first_byte = self.0[0];
        if self.0.iter().all(|&b| b == first_byte) {
            bail!("Main key has insufficient entropy");
        }
        Ok(())
    }
}

fn generate_new_main_key() -> Result<MainKey> {
    let mut mk = [0u8; 32];
    rand::rng().fill_bytes(&mut mk);

    let main_key = MainKey(mk);
    main_key.validate()?;

    let mut encoded = Zeroizing::new(BASE64_STANDARD.encode(mk));
    keyring_persist(
        USER_MAIN_KEY_HANDLE,
        &SecretString::from(std::mem::take(&mut *encoded)),
    )
    .context("Failed to persist main key to keyring")?;

    mk.zeroize();
    Ok(main_key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_all_zero_key() {
        let weak = MainKey::from_bytes([0u8; 32]);
        assert!(weak.validate().is_err());
    }

    #[test]
    fn rejects_uniform_key() {
        let uniform = MainKey::from_bytes([0x42u8; 32]);
        assert!(uniform.validate().is_err());
    }

    #[test]
    fn accepts_random_key() {
        let mut bytes = [0u8; 32];
        rand::rng().fill_bytes(&mut bytes);
        let key = MainKey::from_bytes(bytes);
        assert!(key.validate().is_ok());
    }
}
