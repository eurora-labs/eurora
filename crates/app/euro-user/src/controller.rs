use crate::{User, storage::Storage};
use anyhow::{Context, Result};
use euro_auth::AuthManager;
use euro_secret::secret;
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
}
