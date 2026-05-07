mod extract;

use std::collections::HashSet;

use anyhow::{Result, anyhow};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Validation, decode};

pub use auth_core::{Claims, Role};
pub use extract::{AuthUser, InvalidUserId, MissingClaims};

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

/// Failure modes when reading JWT configuration from the environment.
///
/// Carries the variable name so callers (e.g. `be-monolith`'s startup
/// error printer) can produce remediation messages without re-deriving
/// which secret was missing.
#[derive(Debug, thiserror::Error)]
pub enum JwtConfigError {
    #[error("JWT signing secret `{name}` is unset or empty")]
    MissingSecret { name: &'static str },
}

impl JwtConfig {
    /// Build [`JwtConfig`] strictly from environment variables.
    ///
    /// Returns [`JwtConfigError::MissingSecret`] if either
    /// `JWT_ACCESS_SECRET` or `JWT_REFRESH_SECRET` is unset or blank.
    /// Callers that want a debug-build placeholder should layer that
    /// policy on top of this constructor — the goal here is to keep the
    /// secret-loading path obvious in code, not bury the fallback behind
    /// a convenience trait.
    pub fn try_from_env() -> std::result::Result<Self, JwtConfigError> {
        let access_secret = require_secret("JWT_ACCESS_SECRET")?;
        let refresh_secret = require_secret("JWT_REFRESH_SECRET")?;
        Ok(Self::from_secrets(&access_secret, &refresh_secret))
    }

    /// Construct a [`JwtConfig`] from explicit secrets. Used both by
    /// [`try_from_env`](Self::try_from_env) and by callers that source
    /// the secrets from elsewhere (e.g. a debug-build fallback to stable
    /// placeholders).
    pub fn from_secrets(access_secret: &str, refresh_secret: &str) -> Self {
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

fn require_secret(name: &'static str) -> std::result::Result<String, JwtConfigError> {
    match std::env::var(name) {
        Ok(v) if !v.is_empty() => Ok(v),
        _ => Err(JwtConfigError::MissingSecret { name }),
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
