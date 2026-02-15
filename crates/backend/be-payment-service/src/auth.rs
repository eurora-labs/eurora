use axum::extract::FromRequestParts;
use axum::http::request::Parts;

use crate::error::PaymentError;

pub struct AuthUser(pub be_auth_core::Claims);

impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = PaymentError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let claims = parts
            .extensions
            .get::<be_auth_core::Claims>()
            .cloned()
            .ok_or_else(|| PaymentError::Unauthorized("Not authenticated".to_string()))?;

        Ok(AuthUser(claims))
    }
}
