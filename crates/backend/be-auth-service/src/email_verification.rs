//! Email-verification token issue / verify / resend.

use auth_core::TokenResponse;
use chrono::{Duration, Utc};
use uuid::Uuid;

use crate::VERIFICATION_RESEND_COOLDOWN_SECONDS;
use crate::VERIFICATION_TOKEN_BYTES;
use crate::VERIFICATION_TOKEN_EXPIRY_HOURS;
use crate::error::{AuthError, AuthResult};
use crate::service::AuthService;
use crate::tokens::{random_hex, sha256_token};

impl AuthService {
    /// Issue a new email-verification token for `user` and dispatch the
    /// verification email. No-ops with a warn-level log when the email
    /// service is unconfigured (typical local-dev mode).
    pub(crate) async fn send_verification_email(
        &self,
        user: &be_remote_db::User,
    ) -> AuthResult<()> {
        let Some(email_service) = self.email_service() else {
            tracing::warn!("email service not configured, skipping verification email");
            return Ok(());
        };

        let raw_token = random_hex(VERIFICATION_TOKEN_BYTES);
        let token_hash = sha256_token(&raw_token);

        self.db()
            .create_email_verification_token()
            .user_id(user.id)
            .token_hash(token_hash)
            .expires_at(Utc::now() + Duration::hours(VERIFICATION_TOKEN_EXPIRY_HOURS))
            .call()
            .await?;

        email_service
            .send_verification_email(&user.email, &raw_token, user.display_name.as_deref())
            .await
            .map_err(|e| AuthError::Internal(format!("Failed to send verification email: {e}")))?;

        Ok(())
    }

    /// Atomically consume a verification token and mark the matching
    /// user verified, then mint a fresh session.
    pub async fn verify_email(&self, token: &str) -> AuthResult<TokenResponse> {
        if token.is_empty() {
            return Err(AuthError::InvalidInput(
                "Verification token is required".into(),
            ));
        }

        let token_hash = sha256_token(token);

        let user = self
            .db()
            .consume_email_verification_token()
            .token_hash(&token_hash)
            .call()
            .await
            .map_err(|e| {
                if e.is_not_found() {
                    tracing::warn!("Email verification token not found or expired");
                    AuthError::InvalidInput("Invalid or expired verification token".into())
                } else {
                    AuthError::Database(e)
                }
            })?;

        let role = self
            .ensure_plan_and_resolve_role(user.id, &user.email)
            .await?;

        // The DB row now says verified=true; reflect that in the JWT.
        let mut user_for_session = user.clone();
        user_for_session.email_verified = true;

        tracing::info!(user_id = %user.id, "Email verified successfully");

        self.mint_session(&user_for_session, role).await
    }

    pub async fn resend_verification_email(&self, user_id: Uuid) -> AuthResult<()> {
        let user = self.db().get_user().id(user_id).call().await.map_err(|e| {
            if e.is_not_found() {
                AuthError::InvalidCredentials
            } else {
                AuthError::Database(e)
            }
        })?;

        if user.email_verified {
            return Err(AuthError::EmailAlreadyVerified);
        }

        match self
            .db()
            .get_latest_verification_token_for_user()
            .user_id(user.id)
            .call()
            .await
        {
            Ok(latest) => {
                let elapsed = Utc::now() - latest.created_at;
                if elapsed.num_seconds() < VERIFICATION_RESEND_COOLDOWN_SECONDS {
                    return Err(AuthError::VerificationResendCooldown);
                }
            }
            Err(e) if e.is_not_found() => {
                // First resend; no cooldown to enforce.
            }
            Err(e) => return Err(AuthError::Database(e)),
        }

        self.send_verification_email(&user).await
    }
}
