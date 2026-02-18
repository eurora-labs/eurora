//! This module contains facilities to handle the persistence of secrets.
//!
//! These are stateless and global, while discouraging storing secrets
//! in memory beyond their use.

use anyhow::Result;
use co_utils::Sensitive;
use std::sync::Mutex;

/// Determines how a secret's name should be modified to produce a namespace.
///
/// Namespaces can be used to partition secrets, depending on some criteria.
#[derive(Debug, Clone, Copy)]
pub enum Namespace {
    /// Each application build, like `dev`, `production` and `nightly` have their
    /// own set of secrets. They do not overlap, which reflects how data-files
    /// are stored as well.
    BuildKind,
    /// All secrets are in a single namespace. There is no partitioning.
    /// This can be useful for secrets to be shared across all build kinds.
    Global,
}

/// Persist `secret` in `namespace` so that it can be retrieved by the given `handle`.
pub fn persist(handle: &str, secret: &Sensitive<String>, namespace: Namespace) -> Result<()> {
    let entry = entry_for(handle, namespace)?;
    if secret.0.is_empty() {
        entry.delete_credential()?;
    } else {
        entry.set_password(&secret.0)?;
    }
    Ok(())
}

/// Obtain the previously [stored](persist()) secret known as `handle` from `namespace`.
pub fn retrieve(handle: &str, namespace: Namespace) -> Result<Option<Sensitive<String>>> {
    match entry_for(handle, namespace)?.get_password() {
        Ok(secret) => Ok(Some(Sensitive(secret))),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(err) => Err(err.into()),
    }
}

/// Delete the secret at `handle` permanently from `namespace`.
pub fn delete(handle: &str, namespace: Namespace) -> Result<()> {
    Ok(entry_for(handle, namespace)?.delete_credential()?)
}

/// Use this `identifier` as 'namespace' for identifying secrets.
/// Each namespace has its own set of secrets, useful for different application versions.
///
/// Note that the namespace will be `development` if `identifier` is empty (or wasn't set).
pub fn set_application_namespace(identifier: impl Into<String>) {
    *NAMESPACE.lock().unwrap() = identifier.into()
}

fn entry_for(handle: &str, namespace: Namespace) -> Result<keyring::Entry> {
    let ns = match namespace {
        Namespace::BuildKind => NAMESPACE.lock().unwrap().clone(),
        Namespace::Global => "eurora".into(),
    };
    Ok(keyring::Entry::new(
        &format!(
            "{prefix}-{handle}",
            prefix = if ns.is_empty() { "development" } else { &ns }
        ),
        "Eurora",
    )?)
}

/// How to further specialize secrets to avoid name clashes in the globally shared keystore.
static NAMESPACE: Mutex<String> = Mutex::new(String::new());
