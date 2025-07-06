use std::path::PathBuf;

use crate::{
    User,
    auth::{AuthManager, Claims},
    storage::Storage,
};
use anyhow::{Context, Result};
use eur_secret::{Sensitive, secret};

#[derive(Clone)]
pub struct Controller {
    pub auth_manager: AuthManager,
    storage: Storage,
}

impl Controller {
    pub async fn from_path(path: impl Into<PathBuf>) -> Controller {
        let auth_manager = AuthManager::new()
            .await
            .expect("Failed to create auth manager");
        Controller {
            auth_manager,
            storage: Storage::from_path(path),
        }
    }

    /// Return the current login, or `None` if there is none yet.
    pub fn get_user(&self) -> Result<Option<User>> {
        let user = self.storage.get().context("failed to get user")?;
        if let Some(user) = &user {
            write_without_secrets_if_secrets_present(&self.storage, user.clone())?;
        }
        Ok(user)
    }

    /// Note that secrets are never written in plain text, but we assure they are stored.
    pub fn set_user(&self, user: &User) -> Result<()> {
        if !write_without_secrets_if_secrets_present(&self.storage, user.clone())? {
            self.storage.set(user).context("failed to set user")
        } else {
            Ok(())
        }
    }

    pub fn delete_user(&self) -> Result<()> {
        self.storage.delete().context("failed to delete user")?;
        let namespace = secret::Namespace::BuildKind;
        secret::delete(AuthManager::ACCESS_TOKEN_HANDLE, namespace).ok();
        secret::delete(AuthManager::REFRESH_TOKEN_HANDLE, namespace).ok();
        Ok(())
    }

    pub async fn login(
        &self,
        login: impl Into<String>,
        password: impl Into<String>,
    ) -> Result<Sensitive<String>> {
        self.auth_manager.login(login, password).await
    }

    pub async fn get_or_refresh_access_token(&self) -> Result<Sensitive<String>> {
        self.auth_manager.get_or_refresh_access_token().await
    }

    pub fn get_access_token_payload(&self) -> Result<Claims> {
        self.auth_manager.get_access_token_payload()
    }

    pub fn get_refresh_token_payload(&self) -> Result<Claims> {
        self.auth_manager.get_refresh_token_payload()
    }

    pub async fn refresh_tokens(&self) -> Result<Sensitive<String>> {
        self.auth_manager.refresh_tokens().await
    }

    pub async fn get_login_tokens(&self) -> Result<(String, String)> {
        self.auth_manager.get_login_tokens().await
    }

    pub async fn login_by_login_token(
        &self,
        login_token: impl Into<String>,
    ) -> Result<Sensitive<String>> {
        self.auth_manager
            .login_by_login_token(login_token.into())
            .await
    }
}

/// As `user` sports interior mutability right now, let's play it safe and work with fully owned items only.
fn write_without_secrets_if_secrets_present(storage: &Storage, user: User) -> Result<bool> {
    let mut needs_write = false;
    let namespace = secret::Namespace::BuildKind;
    if let Some(gb_token) = user.access_token.borrow_mut().take() {
        needs_write |=
            secret::persist(AuthManager::ACCESS_TOKEN_HANDLE, &gb_token, namespace).is_ok();
    }
    if let Some(refresh_token) = user.refresh_token.borrow_mut().take() {
        needs_write |=
            secret::persist(AuthManager::REFRESH_TOKEN_HANDLE, &refresh_token, namespace).is_ok();
    }
    if needs_write {
        storage.set(&user)?;
    }
    Ok(needs_write)
}
