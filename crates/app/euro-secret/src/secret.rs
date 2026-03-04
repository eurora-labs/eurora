use std::path::PathBuf;

use anyhow::Result;
use secrecy::{ExposeSecret, SecretString};

use crate::file_store;

const PREFIX: &str = "eurora";

pub fn init_file_store(encryption_key: [u8; 32], data_dir: impl Into<PathBuf>) -> Result<()> {
    file_store::init(encryption_key, data_dir.into())
}

pub fn persist(handle: &str, secret: &SecretString) -> Result<()> {
    if file_store::is_initialized() {
        let qh = qualified_handle(handle);
        if secret.expose_secret().is_empty() {
            file_store::remove(&qh)?;
        } else {
            file_store::set(&qh, secret.expose_secret())?;
        }
        Ok(())
    } else {
        keyring_persist(handle, secret)
    }
}

pub fn retrieve(handle: &str) -> Result<Option<SecretString>> {
    if file_store::is_initialized() {
        let qh = qualified_handle(handle);

        if let Some(value) = file_store::get(&qh)? {
            return Ok(Some(SecretString::from(value)));
        }

        match keyring_retrieve(handle) {
            Ok(Some(secret)) => {
                tracing::debug!(handle, "migrating secret from keychain to file store");
                file_store::set(&qh, secret.expose_secret())?;
                let _ = keyring_delete(handle);
                Ok(Some(secret))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(e),
        }
    } else {
        keyring_retrieve(handle)
    }
}

pub fn delete(handle: &str) -> Result<()> {
    if file_store::is_initialized() {
        let qh = qualified_handle(handle);
        file_store::remove(&qh)?;
    } else {
        let _ = keyring_delete(handle);
    }
    Ok(())
}

pub fn keyring_persist(handle: &str, secret: &SecretString) -> Result<()> {
    let entry = keyring_entry_for(handle)?;
    if secret.expose_secret().is_empty() {
        entry.delete_credential()?;
    } else {
        entry.set_password(secret.expose_secret())?;
    }
    Ok(())
}

pub fn keyring_retrieve(handle: &str) -> Result<Option<SecretString>> {
    match keyring_entry_for(handle)?.get_password() {
        Ok(secret) => Ok(Some(SecretString::from(secret))),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(err) => Err(err.into()),
    }
}

pub fn keyring_delete(handle: &str) -> Result<()> {
    match keyring_entry_for(handle)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(err) => Err(err.into()),
    }
}

fn qualified_handle(handle: &str) -> String {
    format!("{PREFIX}-{handle}")
}

fn keyring_entry_for(handle: &str) -> Result<keyring::Entry> {
    let service = qualified_handle(handle);
    Ok(keyring::Entry::new(&service, "Eurora")?)
}
