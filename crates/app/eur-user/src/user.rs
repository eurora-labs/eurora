use chrono::prelude::*;
use eur_secret::Sensitive;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct User {
    pub id: Uuid,
    pub login: String,
    pub email: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(skip_serializing)]
    pub(super) _access_token: RefCell<Option<Sensitive<String>>>,
}
