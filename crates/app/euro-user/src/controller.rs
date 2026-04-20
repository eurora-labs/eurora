use crate::{User, storage::Storage};
use anyhow::{Context, Result};
use euro_auth::{AuthManager, Claims};
use euro_secret::{SecretString, secret};
use std::path::PathBuf;

#[derive(Clone)]
pub struct UserController {
    pub auth_manager: AuthManager,
    storage: Storage,
}

impl UserController {
    pub fn new(path: impl Into<PathBuf>, auth_manager: AuthManager) -> UserController {
        UserController {
            auth_manager,
            storage: Storage::from_path(path),
        }
    }

    pub fn get_or_create_user(&self) -> Result<User> {
        let user = self.get_user()?;
        if let Some(user) = user {
            return Ok(user);
        }
        let user = User::default();
        self.set_user(&user)?;
        Ok(user)
    }

    pub fn get_user(&self) -> Result<Option<User>> {
        let user = self.storage.get().context("failed to get user")?;
        Ok(user)
    }

    pub fn set_user(&self, user: &User) -> Result<()> {
        self.storage.set(user).context("failed to set user")
    }

    pub fn delete_user(&self) -> Result<()> {
        self.storage.delete().context("failed to delete user")?;
        secret::delete(crate::ACCESS_TOKEN_HANDLE).ok();
        secret::delete(crate::REFRESH_TOKEN_HANDLE).ok();
        Ok(())
    }

    pub async fn register(
        &self,
        email: impl Into<String>,
        password: impl Into<String>,
    ) -> Result<SecretString> {
        self.auth_manager.register(email, password).await
    }

    pub async fn login(
        &self,
        login: impl Into<String>,
        password: impl Into<String>,
    ) -> Result<SecretString> {
        self.auth_manager.login(login, password).await
    }

    pub async fn get_or_refresh_access_token(&self) -> Result<SecretString> {
        self.auth_manager.get_or_refresh_access_token().await
    }

    pub fn get_access_token_payload(&self) -> Result<Claims> {
        self.auth_manager.get_access_token_payload()
    }

    pub fn get_refresh_token_payload(&self) -> Result<Claims> {
        self.auth_manager.get_refresh_token_payload()
    }

    pub async fn refresh_tokens(&self) -> Result<SecretString> {
        self.auth_manager.refresh_tokens().await
    }

    pub async fn get_login_tokens(&self) -> Result<(String, String)> {
        self.auth_manager.get_login_tokens().await
    }

    pub async fn resend_verification_email(&self) -> Result<()> {
        self.auth_manager.resend_verification_email().await
    }

    pub async fn login_by_login_token(
        &self,
        login_token: impl Into<String>,
    ) -> Result<SecretString> {
        self.auth_manager
            .login_by_login_token(login_token.into())
            .await
    }
}
