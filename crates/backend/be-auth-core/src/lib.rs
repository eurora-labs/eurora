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
        let access_private = std::env::var("JWT_ACCESS_PRIVATE_KEY")
            .expect("JWT_ACCESS_PRIVATE_KEY must be set (PEM-encoded EC private key)");
        let access_public = std::env::var("JWT_ACCESS_PUBLIC_KEY")
            .expect("JWT_ACCESS_PUBLIC_KEY must be set (PEM-encoded EC public key)");
        let refresh_private = std::env::var("JWT_REFRESH_PRIVATE_KEY")
            .expect("JWT_REFRESH_PRIVATE_KEY must be set (PEM-encoded EC private key)");
        let refresh_public = std::env::var("JWT_REFRESH_PUBLIC_KEY")
            .expect("JWT_REFRESH_PUBLIC_KEY must be set (PEM-encoded EC public key)");

        Self {
            access_token_encoding_key: EncodingKey::from_ec_pem(access_private.as_bytes())
                .expect("JWT_ACCESS_PRIVATE_KEY is not a valid EC PEM key"),
            access_token_decoding_key: DecodingKey::from_ec_pem(access_public.as_bytes())
                .expect("JWT_ACCESS_PUBLIC_KEY is not a valid EC PEM key"),
            refresh_token_encoding_key: EncodingKey::from_ec_pem(refresh_private.as_bytes())
                .expect("JWT_REFRESH_PRIVATE_KEY is not a valid EC PEM key"),
            refresh_token_decoding_key: DecodingKey::from_ec_pem(refresh_public.as_bytes())
                .expect("JWT_REFRESH_PUBLIC_KEY is not a valid EC PEM key"),

            access_token_expiry_hours: 1,
            refresh_token_expiry_days: 7,

            validation: Validation::new(Algorithm::ES256),

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
