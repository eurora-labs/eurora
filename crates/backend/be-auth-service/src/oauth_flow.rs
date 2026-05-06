//! Third-party (OAuth) authentication flow.
//!
//! Lives behind two public entry points on [`AuthService`]:
//!
//! - [`AuthService::third_party_auth_url`] mints the authorisation URL
//!   the desktop / web client redirects the user to. Stores a single
//!   `oauth_state` row containing the encrypted PKCE verifier and (for
//!   OIDC providers) the encrypted nonce.
//! - [`AuthService::login_third_party`] handles the callback: consumes
//!   the state row, exchanges the code with the provider, resolves /
//!   creates the user, and mints a session.
//!
//! The shared post-resolution tail (`complete_oauth_login`) runs once
//! per provider so token-issuing logic stays in lockstep.

use auth_core::{Provider, TokenResponse};
use be_remote_db::{DbError, OAuthProvider};
use chrono::{Duration, Utc};
use openidconnect::{Nonce, PkceCodeChallenge};
use secrecy::ExposeSecret;

use crate::OAUTH_STATE_EXPIRY_MINUTES;
use crate::USERS_EMAIL_UNIQUE_CONSTRAINT;
use crate::crypto::{decrypt_sensitive_string, encrypt_sensitive_string};
use crate::error::{AuthError, AuthResult};
use crate::log_redaction::hash_email_for_log;
use crate::oauth::{NewOAuthIdentity, OAuthTokenBundle};
use crate::service::{AuthService, MintedSession, user_info_from_row};
use crate::tokens::random_hex;

const OAUTH_STATE_BYTES: usize = 16; // 32 hex chars, ~128 bits of entropy

impl AuthService {
    pub async fn third_party_auth_url(&self, provider: Provider) -> AuthResult<String> {
        match provider {
            Provider::Google => self.google_auth_url().await,
            Provider::Github => self.github_auth_url().await,
        }
    }

    pub async fn login_third_party(
        &self,
        provider: Provider,
        code: &str,
        state: &str,
        login_token: Option<String>,
    ) -> AuthResult<MintedSession> {
        match provider {
            Provider::Google => self.handle_google_login(code, state, login_token).await,
            Provider::Github => self.handle_github_login(code, state, login_token).await,
        }
    }

    async fn google_auth_url(&self) -> AuthResult<String> {
        tracing::info!("Generating Google OAuth URL");
        let google = self.google_oauth()?;

        let state = random_hex(OAUTH_STATE_BYTES);
        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
        let nonce = Nonce::new_random();

        let encrypted_pkce_verifier = encrypt_sensitive_string(pkce_verifier.secret())?;
        let encrypted_nonce = encrypt_sensitive_string(nonce.secret())?;

        self.db()
            .create_oauth_state()
            .state(state.clone())
            .pkce_verifier(encrypted_pkce_verifier)
            .redirect_uri(google.redirect_uri().to_string())
            .expires_at(Utc::now() + Duration::minutes(OAUTH_STATE_EXPIRY_MINUTES))
            .nonce(encrypted_nonce)
            .call()
            .await?;

        Ok(google.authorization_url(&state, pkce_challenge, nonce))
    }

    async fn github_auth_url(&self) -> AuthResult<String> {
        tracing::info!("Generating GitHub OAuth URL");
        let github = self.github_oauth()?;

        let state = random_hex(OAUTH_STATE_BYTES);
        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

        let encrypted_pkce_verifier = encrypt_sensitive_string(pkce_verifier.secret())?;

        self.db()
            .create_oauth_state()
            .state(state.clone())
            .pkce_verifier(encrypted_pkce_verifier)
            .redirect_uri(github.redirect_uri().to_string())
            .expires_at(Utc::now() + Duration::minutes(OAUTH_STATE_EXPIRY_MINUTES))
            .nonce(Vec::new())
            .call()
            .await?;

        Ok(github.authorization_url(&state, pkce_challenge.as_str()))
    }

    async fn handle_google_login(
        &self,
        code: &str,
        state: &str,
        login_token: Option<String>,
    ) -> AuthResult<MintedSession> {
        if code.is_empty() {
            return Err(AuthError::InvalidInput(
                "Authorization code is required".into(),
            ));
        }
        if state.is_empty() {
            return Err(AuthError::InvalidInput(
                "State parameter is required".into(),
            ));
        }

        let oauth_state = self.consume_oauth_state(state, "Google").await?;

        let pkce_verifier = decrypt_sensitive_string(&oauth_state.pkce_verifier)?;

        let nonce_bytes = oauth_state
            .nonce
            .as_deref()
            .ok_or_else(|| AuthError::InvalidInput("Missing nonce for OIDC verification".into()))?;
        let nonce = Nonce::new(decrypt_sensitive_string(nonce_bytes)?);

        let google = self.google_oauth()?;
        let user_info = google.exchange_code(code, pkce_verifier, &nonce).await?;

        if !user_info.verified_email {
            tracing::warn!(
                email_log = ?hash_email_for_log(&user_info.email),
                "Google login rejected: email not verified",
            );
            return Err(AuthError::EmailNotVerified);
        }

        let encrypted_access_token =
            encrypt_sensitive_string(user_info.access_token.expose_secret())?;
        let encrypted_refresh_token = user_info
            .refresh_token
            .as_ref()
            .map(|t| encrypt_sensitive_string(t.expose_secret()))
            .transpose()?;
        let access_token_expiry = user_info
            .expires_in
            .map(|d| Utc::now() + Duration::seconds(d.as_secs() as i64));

        let identity = NewOAuthIdentity {
            provider: OAuthProvider::Google,
            provider_user_id: user_info.id,
            email: user_info.email,
            email_verified: user_info.verified_email,
            display_name: user_info.display_name,
            tokens: OAuthTokenBundle {
                encrypted_access_token,
                encrypted_refresh_token,
                access_token_expiry,
                scope: user_info.scope,
            },
        };

        self.complete_oauth_login(identity, login_token).await
    }

    async fn handle_github_login(
        &self,
        code: &str,
        state: &str,
        login_token: Option<String>,
    ) -> AuthResult<MintedSession> {
        if code.is_empty() {
            return Err(AuthError::InvalidInput(
                "Authorization code is required".into(),
            ));
        }
        if state.is_empty() {
            return Err(AuthError::InvalidInput(
                "State parameter is required".into(),
            ));
        }

        let oauth_state = self.consume_oauth_state(state, "GitHub").await?;

        let pkce_verifier = decrypt_sensitive_string(&oauth_state.pkce_verifier)?;

        let github = self.github_oauth()?;
        let user_info = github.exchange_code(code, &pkce_verifier).await?;

        if !user_info.verified_email {
            tracing::warn!(
                email_log = ?hash_email_for_log(&user_info.email),
                "GitHub login rejected: email not verified",
            );
            return Err(AuthError::EmailNotVerified);
        }

        let encrypted_access_token =
            encrypt_sensitive_string(user_info.access_token.expose_secret())?;

        let identity = NewOAuthIdentity {
            provider: OAuthProvider::Github,
            provider_user_id: user_info.id,
            email: user_info.email,
            email_verified: user_info.verified_email,
            display_name: user_info.display_name,
            tokens: OAuthTokenBundle {
                encrypted_access_token,
                encrypted_refresh_token: None,
                access_token_expiry: None,
                scope: user_info.scope,
            },
        };

        self.complete_oauth_login(identity, login_token).await
    }

    /// Consume a previously-issued `oauth_state` row, rejecting anything
    /// that isn't a fresh, unexpired match. The provider name is used
    /// only for the warn-log so SecOps can tell which callback path
    /// failed.
    async fn consume_oauth_state(
        &self,
        state: &str,
        provider: &'static str,
    ) -> AuthResult<be_remote_db::OAuthState> {
        self.db()
            .consume_oauth_state()
            .state(state)
            .call()
            .await
            .map_err(|e| {
                if e.is_not_found() {
                    tracing::warn!(provider, "Invalid or expired OAuth state");
                    AuthError::InvalidInput("Invalid or expired state parameter".into())
                } else {
                    AuthError::Database(e)
                }
            })
    }

    /// Shared tail for every OAuth-flow handler:
    ///
    /// 1. resolve / create the user,
    /// 2. reconcile `email_verified` with what the provider asserted,
    /// 3. (best-effort) attach a device-pairing login token,
    /// 4. ensure the user has a plan row and resolve their role,
    /// 5. mint and persist an access/refresh token pair.
    async fn complete_oauth_login(
        &self,
        identity: NewOAuthIdentity,
        login_token: Option<String>,
    ) -> AuthResult<MintedSession> {
        let provider_verified = identity.email_verified;
        let user = self.resolve_oauth_user(identity).await?;
        let email_verified = self
            .sync_oauth_email_verified(&user, provider_verified)
            .await?;

        if let Some(token) = login_token {
            // Device-pairing failures must propagate — silently dropping
            // the row leaves the polling client waiting forever.
            self.associate_login_token_with_user(&user, &token).await?;
        }

        let role = self
            .ensure_plan_and_resolve_role(user.id, &user.email)
            .await?;
        self.mint_session_with_verified_override(&user, role, email_verified)
            .await
    }

    /// Resolve the user for an OAuth-provided identity.
    ///
    /// Returns the matching user if one exists for this
    /// `(provider, provider_user_id)` pair (refreshing stored tokens as
    /// a side-effect), or creates a brand-new user.
    ///
    /// Returns [`AuthError::OAuthEmailConflict`] if the email belongs to
    /// an existing account under a different identity (password
    /// credentials or another OAuth provider). Linking new providers to
    /// an existing account must go through an explicit, authenticated
    /// flow — not through an anonymous OAuth callback. Detection is
    /// race-free: the conflict surfaces from the `users.email`
    /// unique-index violation at insert time.
    async fn resolve_oauth_user(
        &self,
        identity: NewOAuthIdentity,
    ) -> AuthResult<be_remote_db::User> {
        if let Some(user) = self
            .lookup_existing_oauth_identity(identity.provider, &identity.provider_user_id)
            .await?
        {
            self.refresh_oauth_credentials(user.id, identity.provider, identity.tokens)
                .await;
            return Ok(user);
        }

        self.create_oauth_user(identity).await
    }

    async fn lookup_existing_oauth_identity(
        &self,
        provider: OAuthProvider,
        provider_user_id: &str,
    ) -> AuthResult<Option<be_remote_db::User>> {
        match self
            .db()
            .get_user_by_oauth_provider()
            .provider(provider)
            .provider_user_id(provider_user_id)
            .call()
            .await
        {
            Ok(user) => Ok(Some(user)),
            Err(e) if e.is_not_found() => Ok(None),
            Err(e) => Err(AuthError::Database(e)),
        }
    }

    /// Rotate stored OAuth credentials in-place. Failures are logged but
    /// not propagated — the user is already authenticated via the
    /// provider and a transient credential-update failure must not
    /// break login. The next refresh from the provider will overwrite
    /// the stale row.
    async fn refresh_oauth_credentials(
        &self,
        user_id: uuid::Uuid,
        provider: OAuthProvider,
        tokens: OAuthTokenBundle,
    ) {
        let oauth_creds = match self
            .db()
            .get_oauth_credentials_by_provider_and_user()
            .provider(provider)
            .user_id(user_id)
            .call()
            .await
        {
            Ok(creds) => creds,
            Err(e) => {
                tracing::warn!(
                    %user_id,
                    ?provider,
                    error = %e,
                    "failed to locate OAuth credentials for refresh",
                );
                return;
            }
        };

        if let Err(e) = self
            .db()
            .update_oauth_credentials()
            .id(oauth_creds.id)
            .access_token(tokens.encrypted_access_token)
            .maybe_refresh_token(tokens.encrypted_refresh_token)
            .maybe_access_token_expiry(tokens.access_token_expiry)
            .scope(tokens.scope)
            .call()
            .await
        {
            tracing::warn!(
                %user_id,
                ?provider,
                error = %e,
                "failed to update OAuth credentials",
            );
        }
    }

    async fn create_oauth_user(
        &self,
        identity: NewOAuthIdentity,
    ) -> AuthResult<be_remote_db::User> {
        let NewOAuthIdentity {
            provider,
            provider_user_id,
            email,
            email_verified,
            display_name,
            tokens,
        } = identity;

        let result = self
            .db()
            .create_user_with_oauth()
            .email(email.clone())
            .maybe_display_name(display_name)
            .email_verified(email_verified)
            .provider(provider)
            .provider_user_id(provider_user_id)
            .access_token(tokens.encrypted_access_token)
            .maybe_refresh_token(tokens.encrypted_refresh_token)
            .maybe_access_token_expiry(tokens.access_token_expiry)
            .scope(tokens.scope)
            .call()
            .await;

        match result {
            Ok(user) => Ok(user),
            Err(DbError::UniqueViolation { ref constraint })
                if constraint == USERS_EMAIL_UNIQUE_CONSTRAINT =>
            {
                tracing::warn!(
                    ?provider,
                    email_log = ?hash_email_for_log(&email),
                    "OAuth login rejected: email already registered under a different identity",
                );
                Err(AuthError::OAuthEmailConflict)
            }
            Err(e) => Err(AuthError::Database(e)),
        }
    }

    /// Reconcile `email_verified` between our DB row and what the
    /// provider just told us. If the provider says verified but we
    /// don't, persist the change; only return `true` once the DB
    /// actually agrees, so the JWT we mint never asserts something the
    /// DB doesn't.
    async fn sync_oauth_email_verified(
        &self,
        user: &be_remote_db::User,
        provider_verified: bool,
    ) -> AuthResult<bool> {
        if user.email_verified || !provider_verified {
            return Ok(user.email_verified || provider_verified);
        }

        self.db()
            .set_email_verified()
            .user_id(user.id)
            .call()
            .await?;
        Ok(true)
    }

    /// Same as `mint_session` but stamps the JWT with
    /// `email_verified = override_email_verified` rather than the value
    /// on the user row. Used by the OAuth flow because the OIDC
    /// `email_verified` claim may have just flipped the user's status,
    /// and we don't want a stale read of the row to suppress that.
    async fn mint_session_with_verified_override(
        &self,
        user: &be_remote_db::User,
        role: auth_core::Role,
        override_email_verified: bool,
    ) -> AuthResult<MintedSession> {
        let pair = crate::tokens::generate_jwt_pair(
            self.jwt_config(),
            user.id,
            &user.email,
            user.display_name.clone(),
            role.clone(),
            override_email_verified,
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
        let user_info = user_info_from_row(user, role, override_email_verified);
        Ok(MintedSession::new(tokens, user_info))
    }
}
