//! Email + password registration and login.

use auth_core::TokenResponse;

use crate::error::{AuthError, AuthResult};
use crate::passwords::{hash_password, validate_email, validate_password, verify_password};
use crate::service::{AuthService, MintedSession, user_info_from_row};
use crate::tokens::generate_jwt_pair;

impl AuthService {
    pub async fn register_user(
        &self,
        email: &str,
        password: &str,
        display_name: Option<String>,
    ) -> AuthResult<MintedSession> {
        let email = email.trim();
        validate_email(email)?;
        validate_password(password)?;

        if self.db().user_exists_by_email().email(email).call().await? {
            return Err(AuthError::InvalidInput("Email already taken".into()));
        }

        let password_hash = hash_password(password)?;

        let mut user = self
            .db()
            .create_user()
            .email(email.to_string())
            .maybe_display_name(display_name)
            .password_hash(password_hash)
            .call()
            .await?;

        if self.email_service().is_some() {
            // Best-effort: a transient email-service failure must not
            // wedge the registration. The user can re-request the
            // verification email later via `resend_verification_email`.
            if let Err(e) = self.send_verification_email(&user).await {
                tracing::error!(user_id = %user.id, error = %e, "send verification email failed");
            }
        } else {
            // No email service configured (typically a debug build).
            // Mark the user verified so subsequent endpoints behave normally.
            self.db()
                .set_email_verified()
                .user_id(user.id)
                .call()
                .await?;
            user.email_verified = true;
        }

        let role = self
            .ensure_plan_and_resolve_role(user.id, &user.email)
            .await?;
        self.mint_session(&user, role).await
    }

    pub async fn login_email_password(
        &self,
        email: &str,
        password: &str,
    ) -> AuthResult<MintedSession> {
        let email = email.trim();
        if email.is_empty() || password.is_empty() {
            return Err(AuthError::InvalidInput(
                "Email and password are required".into(),
            ));
        }

        // Treat "user does not exist" identically to "wrong password" so
        // the response shape doesn't enable account enumeration via
        // timing or status code.
        let user = self
            .db()
            .get_user()
            .email(email.to_string())
            .call()
            .await
            .map_err(|e| {
                if e.is_not_found() {
                    AuthError::InvalidCredentials
                } else {
                    AuthError::Database(e)
                }
            })?;

        let pw_creds = self
            .db()
            .get_password_credentials()
            .user_id(user.id)
            .call()
            .await
            .map_err(|e| {
                if e.is_not_found() {
                    AuthError::InvalidCredentials
                } else {
                    AuthError::Database(e)
                }
            })?;

        verify_password(password, &pw_creds.password_hash)?;

        let role = self
            .ensure_plan_and_resolve_role(user.id, &user.email)
            .await?;
        self.mint_session(&user, role).await
    }

    /// Generate an access/refresh pair, persist the refresh-token hash,
    /// and bundle it with the user profile that handlers will serialise
    /// alongside (or instead of) the tokens.
    pub(crate) async fn mint_session(
        &self,
        user: &be_remote_db::User,
        role: auth_core::Role,
    ) -> AuthResult<MintedSession> {
        let pair = generate_jwt_pair(
            self.jwt_config(),
            user.id,
            &user.email,
            user.display_name.clone(),
            role.clone(),
            user.email_verified,
        )?;

        self.db()
            .create_refresh_token()
            .user_id(user.id)
            .token_hash(pair.refresh_token_hash)
            .expires_at(pair.refresh_expires_at)
            .call()
            .await?;

        let tokens = TokenResponse {
            access_token: pair.access_token,
            refresh_token: pair.refresh_token,
            expires_in: self.jwt_config().access_token_expiry_hours * 3600,
        };
        let user_info = user_info_from_row(user, role, user.email_verified);
        Ok(MintedSession::new(tokens, user_info))
    }
}
