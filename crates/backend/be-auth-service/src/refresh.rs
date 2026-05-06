//! Refresh-token rotation and logout.

use auth_core::TokenResponse;

use crate::error::{AuthError, AuthResult};
use crate::service::{AuthService, MintedSession, user_info_from_row};
use crate::tokens::{generate_jwt_pair, sha256_token};

impl AuthService {
    /// Rotate a refresh token: validate the inbound token, generate a
    /// new pair, and atomically swap the stored hash so the old token
    /// can never be reused.
    pub async fn refresh_access_token(&self, refresh_token: &str) -> AuthResult<MintedSession> {
        let token_hash = sha256_token(refresh_token);

        let existing = self
            .db()
            .get_refresh_token_by_hash()
            .token_hash(&token_hash)
            .call()
            .await
            .map_err(|e| {
                if e.is_not_found() {
                    AuthError::InvalidToken
                } else {
                    AuthError::Database(e)
                }
            })?;

        let user = self
            .db()
            .get_user()
            .id(existing.user_id)
            .call()
            .await
            .map_err(|e| {
                if e.is_not_found() {
                    AuthError::InvalidToken
                } else {
                    AuthError::Database(e)
                }
            })?;

        let role = self
            .ensure_plan_and_resolve_role(user.id, &user.email)
            .await?;

        let pair = generate_jwt_pair(
            self.jwt_config(),
            user.id,
            &user.email,
            user.display_name.clone(),
            role.clone(),
            user.email_verified,
        )?;

        // Atomic swap. If the row vanished between the lookup above and
        // here (concurrent rotation, manual revocation), surface that as
        // `InvalidToken` — the new pair has not been issued. Any other
        // DB error means we genuinely don't know whether the rotation
        // landed; report the database error so the client can retry.
        self.db()
            .rotate_refresh_token()
            .old_token_hash(&token_hash)
            .new_token_hash(pair.refresh_token_hash)
            .new_expires_at(pair.refresh_expires_at)
            .call()
            .await
            .map_err(|e| {
                if e.is_not_found() {
                    AuthError::InvalidToken
                } else {
                    AuthError::Database(e)
                }
            })?;

        let tokens = TokenResponse {
            access_token: pair.access_token,
            refresh_token: pair.refresh_token,
            expires_in: self.jwt_config().access_token_expiry_hours * 3600,
        };
        let user_info = user_info_from_row(&user, role, user.email_verified);
        Ok(MintedSession::new(tokens, user_info))
    }

    pub async fn logout(&self, refresh_token: &str) -> AuthResult<()> {
        let token_hash = sha256_token(refresh_token);
        self.db()
            .revoke_refresh_token()
            .token_hash(&token_hash)
            .call()
            .await
            .map_err(|e| {
                if e.is_not_found() {
                    AuthError::InvalidToken
                } else {
                    AuthError::Database(e)
                }
            })?;
        Ok(())
    }
}
