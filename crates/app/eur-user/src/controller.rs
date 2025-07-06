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
    pub async fn from_path(path: impl Into<PathBuf>) -> Result<Controller> {
        let auth_manager = AuthManager::new()
            .await
            .context("Failed to create auth manager")?;
        Ok(Controller {
            auth_manager,
            storage: Storage::from_path(path),
        })
    }

    /// Return the current login, or `None` if there is none yet.
    pub fn get_user(&self) -> Result<Option<User>> {
        let user = self.storage.get().context("failed to get user")?;
        Ok(user)
    }

    /// Persist the user to storage.
    pub fn set_user(&self, user: &User) -> Result<()> {
        self.storage.set(user).context("failed to set user")
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
