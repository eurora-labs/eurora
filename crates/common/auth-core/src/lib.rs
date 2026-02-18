use serde::{Deserialize, Serialize};

#[cfg(feature = "specta")]
use specta::Type;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "specta", derive(Type))]
pub enum Role {
    Free,
    Tier1,
}

impl Role {
    pub fn rank(&self) -> u8 {
        match self {
            Role::Free => 0,
            Role::Tier1 => 1,
        }
    }
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Role::Free => write!(f, "Free"),
            Role::Tier1 => write!(f, "Tier1"),
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
