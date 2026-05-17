//! Device-pairing login-token flow.
//!
//! The desktop / CLI client generates a PKCE pair, sends the challenge
//! to the web app via deep-link, and polls
//! `/auth/login-token/exchange` with its verifier. Once the web user
//! signs in, the web client posts to `/auth/login-token/associate`
//! which links the *challenge* (already in the device's hands) to the
//! authenticated user. The next poll then completes the device login.
//!
//! Storing `sha256(challenge)` keeps stolen DB rows from giving an
//! attacker the verifier.

use auth_core::TokenResponse;
use chrono::{Duration, Utc};
use openidconnect::{PkceCodeChallenge, PkceCodeVerifier};
use uuid::Uuid;

use crate::LOGIN_TOKEN_EXPIRY_MINUTES;
use crate::error::{AuthError, AuthResult};
use crate::service::{AuthService, MintedSession, user_info_from_row};
use crate::tokens::{generate_jwt_pair, sha256_token};

impl AuthService {
    /// Exchange a PKCE verifier for a session token pair.
    pub async fn login_by_login_token(&self, code_verifier: &str) -> AuthResult<MintedSession> {
        if code_verifier.is_empty() {
            return Err(AuthError::InvalidInput("Login token is required".into()));
        }

        let code_challenge = code_verifier_to_challenge(code_verifier);
        let login_token_hash = sha256_token(&code_challenge);

        let login_token = self
            .db()
            .get_login_token_by_hash()
            .token_hash(&login_token_hash)
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
            .id(login_token.user_id)
            .call()
            .await
            .map_err(|e| {
                if e.is_not_found() {
                    AuthError::InvalidToken
                } else {
                    AuthError::Database(e)
                }
            })?;

        let role = self.resolve_role(user.id).await?;

        let pair = generate_jwt_pair(
            self.jwt_config(),
            user.id,
            &user.email,
            user.display_name.clone(),
            role.clone(),
            user.email_verified,
        )?;

        // Atomic: consume the login-token row and create the
        // refresh-token row in the same transaction.
        self.db()
            .consume_login_token_and_create_refresh_token()
            .login_token_hash(&login_token_hash)
            .user_id(user.id)
            .refresh_token_hash(pair.refresh_token_hash)
            .refresh_token_expires_at(pair.refresh_expires_at)
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

    /// Public: link a code-challenge (already in the device's hands)
    /// to the authenticated user.
    pub async fn associate_login_token(
        &self,
        user_id: Uuid,
        code_challenge: &str,
    ) -> AuthResult<()> {
        if !is_valid_code_challenge(code_challenge) {
            return Err(AuthError::InvalidInput("Invalid code challenge".into()));
        }

        let token_hash = sha256_token(code_challenge);
        self.db()
            .create_login_token()
            .token_hash(token_hash)
            .user_id(user_id)
            .expires_at(Utc::now() + Duration::minutes(LOGIN_TOKEN_EXPIRY_MINUTES))
            .call()
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "failed to associate login token");
                AuthError::Database(e)
            })?;

        Ok(())
    }

    /// Internal: invoked by the OAuth flow when the redirect URL
    /// included a `login_token` (the device's PKCE challenge). Failures
    /// must propagate — silently dropping the row leaves the polling
    /// device waiting forever.
    pub(crate) async fn associate_login_token_with_user(
        &self,
        user: &be_remote_db::User,
        code_challenge: &str,
    ) -> AuthResult<()> {
        let token_hash = sha256_token(code_challenge);
        self.db()
            .create_login_token()
            .token_hash(token_hash)
            .user_id(user.id)
            .expires_at(Utc::now() + Duration::minutes(LOGIN_TOKEN_EXPIRY_MINUTES))
            .call()
            .await
            .map_err(|e| {
                tracing::error!(
                    user_id = %user.id,
                    error = %e,
                    "failed to associate login token during OAuth login",
                );
                AuthError::Database(e)
            })?;

        tracing::info!(user_id = %user.id, "associated login token with user");
        Ok(())
    }
}

/// Derive `BASE64URL(SHA256(verifier))` per RFC 7636 from the verifier.
fn code_verifier_to_challenge(code_verifier: &str) -> String {
    let verifier = PkceCodeVerifier::new(code_verifier.to_string());
    PkceCodeChallenge::from_code_verifier_sha256(&verifier)
        .as_str()
        .to_string()
}

/// Per RFC 7636 §4.2: the S256 code challenge is exactly 43
/// base64url-without-padding characters from the unreserved set.
pub(crate) fn is_valid_code_challenge(s: &str) -> bool {
    s.len() == 43
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_wrong_length_challenges() {
        assert!(!is_valid_code_challenge(""));
        assert!(!is_valid_code_challenge(&"a".repeat(42)));
        assert!(!is_valid_code_challenge(&"a".repeat(44)));
    }

    #[test]
    fn rejects_disallowed_chars() {
        let mut s = "a".repeat(43);
        s.replace_range(0..1, "+");
        assert!(!is_valid_code_challenge(&s));
        s.replace_range(0..1, "/");
        assert!(!is_valid_code_challenge(&s));
        s.replace_range(0..1, "=");
        assert!(!is_valid_code_challenge(&s));
    }

    #[test]
    fn accepts_well_formed_challenge() {
        // 43 chars, all unreserved.
        let s = "Y7-_aAbCdEfGhIjKlMnOpQrStUvWxYz0123456789AB";
        assert_eq!(s.len(), 43);
        assert!(is_valid_code_challenge(s));
    }

    #[test]
    fn challenge_matches_pkce_spec() {
        let verifier = "test_verifier_string_long_enough_for_pkce_oauth_flow";
        let derived = code_verifier_to_challenge(verifier);
        // S256 challenge is 43 chars.
        assert_eq!(derived.len(), 43);
        assert!(is_valid_code_challenge(&derived));
    }
}
