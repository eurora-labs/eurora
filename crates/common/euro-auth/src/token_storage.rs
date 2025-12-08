//! Token storage abstraction for secure JWT token management.

use anyhow::Result;
use async_trait::async_trait;
use euro_secret::{
    Sensitive,
    secret::{self, Namespace},
};

/// Trait for secure token storage implementations
#[async_trait]
pub trait TokenStorage: Send + Sync {
    async fn store_access_token(&self, token: &str) -> Result<()>;
    async fn store_refresh_token(&self, token: &str) -> Result<()>;
    async fn get_access_token(&self) -> Result<Option<String>>;
    async fn get_refresh_token(&self) -> Result<Option<String>>;
    async fn clear_tokens(&self) -> Result<()>;
}

/// Implementation using euro-secret for OS-level secure storage
pub struct SecureTokenStorage;

impl SecureTokenStorage {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl TokenStorage for SecureTokenStorage {
    async fn store_access_token(&self, token: &str) -> Result<()> {
        secret::persist(
            "AUTH_ACCESS_TOKEN",
            &Sensitive(token.to_string()),
            Namespace::Global,
        )?;
        Ok(())
    }

    async fn store_refresh_token(&self, token: &str) -> Result<()> {
        secret::persist(
            "AUTH_REFRESH_TOKEN",
            &Sensitive(token.to_string()),
            Namespace::Global,
        )?;
        Ok(())
    }

    async fn get_access_token(&self) -> Result<Option<String>> {
        match secret::retrieve("AUTH_ACCESS_TOKEN", Namespace::Global)? {
            Some(sensitive_token) => Ok(Some(sensitive_token.0)),
            None => Ok(None),
        }
    }

    async fn get_refresh_token(&self) -> Result<Option<String>> {
        match secret::retrieve("AUTH_REFRESH_TOKEN", Namespace::Global)? {
            Some(sensitive_token) => Ok(Some(sensitive_token.0)),
            None => Ok(None),
        }
    }

    async fn clear_tokens(&self) -> Result<()> {
        if let Err(e) = secret::delete("AUTH_ACCESS_TOKEN", Namespace::Global) {
            match e.downcast_ref::<euro_secret::Error>() {
                Some(euro_secret::Error::NoEntry) => {}
                _ => Err(e)?,
            }
        }
        if let Err(e) = secret::delete("AUTH_REFRESH_TOKEN", Namespace::Global) {
            match e.downcast_ref::<euro_secret::Error>() {
                Some(euro_secret::Error::NoEntry) => {}
                _ => Err(e)?,
            }
        }
        Ok(())
    }
}

impl Default for SecureTokenStorage {
    fn default() -> Self {
        Self::new()
    }
}
