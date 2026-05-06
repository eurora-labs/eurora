//! Token-handling primitives shared across auth flows.
//!
//! - [`sha256_token`]: digest applied to refresh tokens, login tokens, and
//!   email-verification tokens before storing them in Postgres. Hashing
//!   makes a stolen DB row useless on its own.
//! - [`random_hex`]: bias-free hex string generated from `byte_len` bytes
//!   of OS randomness.
//! - [`generate_jwt_pair`]: produce an access/refresh JWT pair from a
//!   shared identity, returning the SHA-256 fingerprint of the refresh
//!   token (for DB persistence) and its absolute expiry.

use auth_core::{Claims, Role};
use be_auth_core::JwtConfig;
use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{Algorithm, Header, encode};
use rand::Rng;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::error::{AuthError, AuthResult};

pub(crate) fn sha256_token(token: &str) -> Vec<u8> {
    Sha256::digest(token.as_bytes()).to_vec()
}

/// Generate `byte_len` bytes of OS randomness and hex-encode them.
/// The output is exactly `2 * byte_len` lowercase hex characters.
pub(crate) fn random_hex(byte_len: usize) -> String {
    let mut bytes = vec![0u8; byte_len];
    rand::rng().fill_bytes(&mut bytes);
    hex::encode(bytes)
}

pub(crate) struct JwtPair {
    pub access_token: String,
    pub refresh_token: String,
    pub refresh_token_hash: Vec<u8>,
    pub refresh_expires_at: DateTime<Utc>,
}

pub(crate) fn generate_jwt_pair(
    config: &JwtConfig,
    user_id: Uuid,
    email: &str,
    display_name: Option<String>,
    role: Role,
    email_verified: bool,
) -> AuthResult<JwtPair> {
    let now = Utc::now();
    let access_exp = now + Duration::hours(config.access_token_expiry_hours);
    let refresh_exp = now + Duration::days(config.refresh_token_expiry_days);

    let sub = user_id.to_string();
    let aud = "eurora".to_string();

    let access_claims = Claims {
        sub: sub.clone(),
        email: email.to_string(),
        display_name: display_name.clone(),
        exp: access_exp.timestamp(),
        iat: now.timestamp(),
        token_type: "access".to_string(),
        role: role.clone(),
        aud: aud.clone(),
        email_verified,
        jti: Uuid::now_v7().to_string(),
    };

    let refresh_claims = Claims {
        sub,
        email: email.to_string(),
        display_name,
        exp: refresh_exp.timestamp(),
        iat: now.timestamp(),
        token_type: "refresh".to_string(),
        role,
        aud,
        email_verified,
        jti: Uuid::now_v7().to_string(),
    };

    let header = Header::new(Algorithm::HS256);
    let access_token = encode(&header, &access_claims, &config.access_token_encoding_key)
        .map_err(|e| AuthError::TokenGeneration(e.to_string()))?;
    let refresh_token = encode(&header, &refresh_claims, &config.refresh_token_encoding_key)
        .map_err(|e| AuthError::TokenGeneration(e.to_string()))?;

    let refresh_token_hash = sha256_token(&refresh_token);

    Ok(JwtPair {
        access_token,
        refresh_token,
        refresh_token_hash,
        refresh_expires_at: refresh_exp,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn random_hex_has_expected_length() {
        let s = random_hex(16);
        assert_eq!(s.len(), 32);
        assert!(s.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn random_hex_has_high_entropy() {
        let a = random_hex(16);
        let b = random_hex(16);
        assert_ne!(a, b);
    }

    #[test]
    fn sha256_token_is_deterministic() {
        let a = sha256_token("token-value");
        let b = sha256_token("token-value");
        assert_eq!(a, b);
        assert_eq!(a.len(), 32);
    }
}
