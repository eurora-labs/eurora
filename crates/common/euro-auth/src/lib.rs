use serde::{Deserialize, Serialize};

/// JWT Claims structure used across all services
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,        // Subject (user ID)
    pub username: String,   // Username
    pub email: String,      // Email
    pub exp: i64,           // Expiration time
    pub iat: i64,           // Issued at
    pub token_type: String, // "access" or "refresh"
}
