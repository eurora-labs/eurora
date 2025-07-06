use anyhow::{Context, Result};
use chrono::prelude::*;
use eur_secret::{Sensitive, secret};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Default)]
pub enum HotkeyFunction {
    #[default]
    OpenLauncher,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Hotkey {
    pub key: String,
    pub modifiers: Vec<String>,
    pub function: HotkeyFunction,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Hotkeys {
    pub open_launcher: Hotkey,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct User {
    pub id: Uuid,
    pub login: String,
    pub email: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,

    pub hotkeys: Hotkeys,

    #[serde(skip_serializing)]
    pub(super) access_token: RefCell<Option<Sensitive<String>>>,

    #[serde(skip_serializing)]
    pub(super) refresh_token: RefCell<Option<Sensitive<String>>>,
}

impl User {
    pub fn access_token(&self) -> Result<Sensitive<String>> {
        if let Some(token) = self.access_token.borrow().as_ref() {
            return Ok(token.clone());
        }
        let err_msg = "access token for user was deleted from keychain - login is now invalid";
        let secret = secret::retrieve(
            crate::auth::AuthManager::ACCESS_TOKEN_HANDLE,
            secret::Namespace::BuildKind,
        )?
        .context(err_msg)?;
        *self.access_token.borrow_mut() = Some(secret.clone());
        Ok(secret)
    }

    pub fn refresh_token(&self) -> Result<Sensitive<String>> {
        if let Some(token) = self.refresh_token.borrow().as_ref() {
            return Ok(token.clone());
        }
        let err_msg = "refresh token for user was deleted from keychain - login is now invalid";
        let secret = secret::retrieve(
            crate::auth::AuthManager::REFRESH_TOKEN_HANDLE,
            secret::Namespace::BuildKind,
        )?
        .context(err_msg)?;
        *self.refresh_token.borrow_mut() = Some(secret.clone());
        Ok(secret)
    }
}
