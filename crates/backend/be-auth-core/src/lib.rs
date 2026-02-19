use anyhow::{Result, anyhow};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Validation, decode};

pub use auth_core::{Claims, Role};

#[derive(Clone)]
pub struct JwtConfig {
    pub access_token_encoding_key: EncodingKey,
    pub access_token_decoding_key: DecodingKey,

    pub refresh_token_encoding_key: EncodingKey,
    pub refresh_token_decoding_key: DecodingKey,

    pub access_token_expiry_hours: i64,
    pub refresh_token_expiry_days: i64,

    pub validation: Validation,

    pub approved_emails: Vec<String>,
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

            validation: Validation::new(Algorithm::HS256),

            approved_emails: std::env::var("APPROVED_EMAILS")
                .unwrap_or_default()
                .split(',')
                .map(|s| s.trim().to_lowercase().to_string())
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>(),
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
}
