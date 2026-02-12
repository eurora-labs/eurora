use serde::{Deserialize, Serialize};

#[cfg(feature = "specta")]
use specta::Type;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "specta", derive(Type))]
pub enum Role {
    Free,
    Tier1,
    Enterprise,
}

impl Role {
    pub fn rank(&self) -> u8 {
        match self {
            Role::Free => 0,
            Role::Tier1 => 1,
            Role::Enterprise => 2,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct Claims {
    pub sub: String,
    pub username: String,
    pub email: String,
    pub exp: i64,
    pub iat: i64,
    pub token_type: String,
    pub role: Role,
}
