//! Internal token storage handles.
//!
//! Kept `pub(crate)` so the manager is the sole writer of session
//! state. External consumers observe transitions through the
//! [`crate::AuthEvent`] bus or `AuthManager`'s typed accessors, not by
//! reaching into the secret store directly.

use anyhow::{Result, anyhow};
use euro_secret::{SecretString, secret};

pub(crate) const ACCESS_TOKEN_HANDLE: &str = "AUTH_ACCESS_TOKEN";
pub(crate) const REFRESH_TOKEN_HANDLE: &str = "AUTH_REFRESH_TOKEN";

pub(crate) fn store_access(token: String) -> Result<()> {
    secret::persist(ACCESS_TOKEN_HANDLE, &SecretString::from(token))
        .map_err(|e| anyhow!("Failed to store access token: {e}"))
}

pub(crate) fn store_refresh(token: String) -> Result<()> {
    secret::persist(REFRESH_TOKEN_HANDLE, &SecretString::from(token))
        .map_err(|e| anyhow!("Failed to store refresh token: {e}"))
}

pub(crate) fn load_access() -> Result<SecretString> {
    secret::retrieve(ACCESS_TOKEN_HANDLE)?.ok_or_else(|| anyhow!("No access token found"))
}

pub(crate) fn load_refresh() -> Result<SecretString> {
    secret::retrieve(REFRESH_TOKEN_HANDLE)?.ok_or_else(|| anyhow!("No refresh token found"))
}

pub(crate) fn clear() {
    if let Err(e) = secret::delete(ACCESS_TOKEN_HANDLE) {
        tracing::warn!(error = %e, "failed to delete access token from secret store");
    }
    if let Err(e) = secret::delete(REFRESH_TOKEN_HANDLE) {
        tracing::warn!(error = %e, "failed to delete refresh token from secret store");
    }
}
