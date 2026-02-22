//! Two-tier secret storage.
//!
//! Only the master encryption key stays in the OS keychain.  This reduces
//! macOS Keychain "Allow" prompts from one per secret to exactly one per
//! application update.
//!
//! Secrets still in the keychain from a previous version are lazily migrated
//! into the file store on first [`retrieve`].

use std::path::PathBuf;

use anyhow::Result;
use co_utils::Sensitive;

use crate::file_store;

/// Kept constant so that migration from the keychain maps 1-to-1 with the
/// file-store backend.
const PREFIX: &str = "eurora";

pub fn init_file_store(encryption_key: [u8; 32], data_dir: impl Into<PathBuf>) -> Result<()> {
    file_store::init(encryption_key, data_dir.into())
}

pub fn persist(handle: &str, secret: &Sensitive<String>) -> Result<()> {
    if file_store::is_initialized() {
        let qh = qualified_handle(handle);
        if secret.0.is_empty() {
            file_store::remove(&qh)?;
        } else {
            file_store::set(&qh, &secret.0)?;
        }
        Ok(())
    } else {
        keyring_persist(handle, secret)
    }
}

pub fn retrieve(handle: &str) -> Result<Option<Sensitive<String>>> {
    if file_store::is_initialized() {
        let qh = qualified_handle(handle);

        if let Some(value) = file_store::get(&qh)? {
            return Ok(Some(Sensitive(value)));
        }

        match keyring_retrieve(handle) {
            Ok(Some(secret)) => {
                tracing::debug!(handle, "migrating secret from keychain to file store");
                file_store::set(&qh, &secret.0)?;
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

        // Don't touch the keychain here — `retrieve()` already handles
        // migration and cleanup lazily.  Hitting the keychain directly
        // would bypass that path and could trigger an extra macOS "Allow"
        // prompt for an entry that hasn't been migrated yet.
    } else {
        let _ = keyring_delete(handle);
    }
    Ok(())
}

pub fn keyring_persist(handle: &str, secret: &Sensitive<String>) -> Result<()> {
    let entry = keyring_entry_for(handle)?;
    if secret.0.is_empty() {
        entry.delete_credential()?;
    } else {
        entry.set_password(&secret.0)?;
    }
    Ok(())
}

pub fn keyring_retrieve(handle: &str) -> Result<Option<Sensitive<String>>> {
    match keyring_entry_for(handle)?.get_password() {
        Ok(secret) => Ok(Some(Sensitive(secret))),
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
