use chrono::prelude::*;
use eur_secret::Sensitive;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum HotkeyFunction {
    OpenLauncher,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Hotkey {
    pub key: String,
    pub modifiers: Vec<String>,
    pub function: HotkeyFunction,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct User {
    pub id: Uuid,
    pub login: String,
    pub email: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub hotkeys: Vec<Hotkey>,

    #[serde(skip_serializing)]
    pub(super) access_token: RefCell<Option<Sensitive<String>>>,

    #[serde(skip_serializing)]
    pub(super) refresh_token: RefCell<Option<Sensitive<String>>>,
}
