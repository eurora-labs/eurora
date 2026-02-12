use std::sync::Arc;

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use be_auth_core::JwtConfig;

use crate::error::PaymentError;

pub struct AuthUser(pub auth_core::Claims);

impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = PaymentError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let jwt_config = parts.extensions.get::<Arc<JwtConfig>>().ok_or_else(|| {
            PaymentError::Internal(anyhow::anyhow!("JwtConfig not found in extensions"))
        })?;

        let auth_header = parts
            .headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| {
                PaymentError::Unauthorized("Missing authorization header".to_string())
            })?;

        if !auth_header.starts_with("Bearer ") {
            return Err(PaymentError::Unauthorized(
                "Authorization header must start with 'Bearer '".to_string(),
            ));
        }

        let token = &auth_header[7..];
        let claims = jwt_config
            .validate_access_token(token)
            .map_err(|e| PaymentError::Unauthorized(e.to_string()))?;

        Ok(AuthUser(claims))
    }
}
