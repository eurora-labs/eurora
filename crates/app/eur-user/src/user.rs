use chrono::prelude::*;
use serde::{Deserialize, Serialize};
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
}
