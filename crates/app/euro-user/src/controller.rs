use crate::{User, storage::Storage};
use anyhow::{Context, Result};
use euro_auth::{AuthManager, Claims};
use euro_secret::{Sensitive, secret};
use std::path::PathBuf;
use tokio::sync::watch;
use tonic::transport::Channel;

#[derive(Clone)]
pub struct Controller {
    pub auth_manager: AuthManager,
    storage: Storage,
}

impl Controller {
    pub fn new(
        path: impl Into<PathBuf>,
        channel_rx: watch::Receiver<Channel>,
    ) -> Result<Controller> {
        let auth_manager = AuthManager::new(channel_rx);
        Ok(Controller {
            auth_manager,
            storage: Storage::from_path(path),
        })
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

    pub fn delete_user(&mut self) -> Result<()> {
        self.storage.delete().context("failed to delete user")?;
        let namespace = secret::Namespace::Global;
        secret::delete(crate::ACCESS_TOKEN_HANDLE, namespace).ok();
        secret::delete(crate::REFRESH_TOKEN_HANDLE, namespace).ok();
        Ok(())
    }

    pub async fn register(
        &mut self,
        username: impl Into<String>,
        email: impl Into<String>,
        password: impl Into<String>,
    ) -> Result<Sensitive<String>> {
        self.auth_manager.register(username, email, password).await
    }

    pub async fn login(
        &mut self,
        login: impl Into<String>,
        password: impl Into<String>,
    ) -> Result<Sensitive<String>> {
        self.auth_manager.login(login, password).await
    }

    pub async fn get_or_refresh_access_token(&mut self) -> Result<Sensitive<String>> {
        self.auth_manager.get_or_refresh_access_token().await
    }

    pub fn get_access_token_payload(&mut self) -> Result<Claims> {
        self.auth_manager.get_access_token_payload()
    }

    pub fn get_refresh_token_payload(&mut self) -> Result<Claims> {
        self.auth_manager.get_refresh_token_payload()
    }

    pub async fn refresh_tokens(&mut self) -> Result<Sensitive<String>> {
        self.auth_manager.refresh_tokens().await
    }

    pub async fn get_login_tokens(&mut self) -> Result<(String, String)> {
        self.auth_manager.get_login_tokens().await
    }

    pub async fn login_by_login_token(
        &mut self,
        login_token: impl Into<String>,
    ) -> Result<Sensitive<String>> {
        self.auth_manager
            .login_by_login_token(login_token.into())
            .await
    }
}
