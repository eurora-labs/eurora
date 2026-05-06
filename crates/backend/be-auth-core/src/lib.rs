mod extract;
mod web_origins;

use std::collections::HashSet;

use anyhow::{Result, anyhow};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Validation, decode};

pub use auth_core::{Claims, Role};
pub use extract::{AuthUser, InvalidUserId, MissingClaims};
pub use web_origins::{
    WEB_ALLOWED_ORIGINS_ENV, default_web_origins, parse_web_origins, web_origins_from_env,
};

#[derive(Clone)]
pub struct JwtConfig {
    pub access_token_encoding_key: EncodingKey,
    pub access_token_decoding_key: DecodingKey,

    pub refresh_token_encoding_key: EncodingKey,
    pub refresh_token_decoding_key: DecodingKey,

    pub access_token_expiry_hours: i64,
    pub refresh_token_expiry_days: i64,

    pub validation: Validation,

    /// Emails (lower-cased) that should be promoted to a paid tier on
    /// account creation. Stored as a `HashSet` so the admission check is
    /// O(1) on every login / registration / refresh.
    pub approved_emails: HashSet<String>,
}

impl Default for JwtConfig {
    fn default() -> Self {
        let access_secret = std::env::var("JWT_ACCESS_SECRET")
            .expect("JWT_ACCESS_SECRET must be set at runtime for secure token validation");
        let refresh_secret = std::env::var("JWT_REFRESH_SECRET")
            .expect("JWT_REFRESH_SECRET must be set at runtime for secure token validation");

        Self {
            access_token_encoding_key: EncodingKey::from_secret(access_secret.as_bytes()),
            access_token_decoding_key: DecodingKey::from_secret(access_secret.as_bytes()),
            refresh_token_encoding_key: EncodingKey::from_secret(refresh_secret.as_bytes()),
            refresh_token_decoding_key: DecodingKey::from_secret(refresh_secret.as_bytes()),

            access_token_expiry_hours: 1,
            refresh_token_expiry_days: 7,

            validation: {
                let mut v = Validation::new(Algorithm::HS256);
                v.set_audience(&["eurora"]);
                v.required_spec_claims.insert("aud".to_string());
                v
            },

            approved_emails: std::env::var("APPROVED_EMAILS")
                .unwrap_or_default()
                .split(',')
                .map(|s| s.trim().to_ascii_lowercase())
                .filter(|s| !s.is_empty())
                .collect(),
        }
    }
}

impl JwtConfig {
    pub fn validate_access_token(&self, token: &str) -> Result<Claims> {
        let token_data = decode::<Claims>(token, &self.access_token_decoding_key, &self.validation)
            .map_err(|e| anyhow!("Invalid token: {}", e))?;

        if token_data.claims.token_type != "access" {
            return Err(anyhow!("Invalid token type: expected access token"));
        }

        Ok(token_data.claims)
    }

    pub fn validate_refresh_token(&self, token: &str) -> Result<Claims> {
        let token_data =
            decode::<Claims>(token, &self.refresh_token_decoding_key, &self.validation)
                .map_err(|e| anyhow!("Invalid token: {}", e))?;

        if token_data.claims.token_type != "refresh" {
            return Err(anyhow!("Invalid token type: expected refresh token"));
        }

        Ok(token_data.claims)
    }

    /// Returns `true` when `email` (case-insensitively) appears on the
    /// upgraded-tier allow-list configured via `APPROVED_EMAILS`.
    pub fn is_approved_email(&self, email: &str) -> bool {
        self.approved_emails.contains(&email.to_ascii_lowercase())
    }
}
