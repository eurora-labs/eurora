//! Shared authentication utilities for the Eurora project.
//!
//! This crate provides common JWT structures and validation functions
//! that can be used across different services in the Eurora ecosystem.

mod auth_manager;
mod grpc_client;
mod token_storage;

pub use auth_manager::{AuthManager, LoginCredentials, RegisterData, UserInfo};
pub use grpc_client::AuthGrpcClient;
pub use token_storage::{SecureTokenStorage, TokenStorage};

use anyhow::{Result, anyhow};
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode};
use serde::{Deserialize, Serialize};

/// JWT Claims structure used across all services
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,        // Subject (user ID)
    pub username: String,   // Username
    pub email: String,      // Email
    pub exp: usize,         // Expiration time
    pub iat: usize,         // Issued at
    pub token_type: String, // "access" or "refresh"
}

/// Configuration for JWT tokens
#[derive(Debug, Clone)]
pub struct JwtConfig {
    pub secret: String,
    pub access_token_expiry_hours: i64,
    pub refresh_token_expiry_days: i64,
}

impl Default for JwtConfig {
    fn default() -> Self {
        Self {
            secret: std::env::var("JWT_SECRET")
                .expect("JWT_SECRET must be set at runtime for secure token validation"),
            access_token_expiry_hours: 1,  // 1 hour
            refresh_token_expiry_days: 30, // 30 days
        }
    }
}

/// Validate and decode a JWT token
pub fn validate_token(token: &str, jwt_config: &JwtConfig) -> Result<Claims> {
    let decoding_key = DecodingKey::from_secret(jwt_config.secret.as_ref());
    let validation = Validation::new(Algorithm::HS256);

    let token_data = decode::<Claims>(token, &decoding_key, &validation)
        .map_err(|e| anyhow!("Invalid token: {}", e))?;

    Ok(token_data.claims)
}

/// Validate an access token specifically (ensures token_type is "access")
pub fn validate_access_token(token: &str, jwt_config: &JwtConfig) -> Result<Claims> {
    let claims = validate_token(token, jwt_config)?;

    // Ensure it's an access token
    if claims.token_type != "access" {
        return Err(anyhow!("Invalid token type: expected access token"));
    }

    Ok(claims)
}

/// Validate a refresh token specifically (ensures token_type is "refresh")
pub fn validate_refresh_token(token: &str, jwt_config: &JwtConfig) -> Result<Claims> {
    let claims = validate_token(token, jwt_config)?;

    // Ensure it's a refresh token
    if claims.token_type != "refresh" {
        return Err(anyhow!("Invalid token type: expected refresh token"));
    }

    Ok(claims)
}
