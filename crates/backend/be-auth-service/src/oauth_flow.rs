//! Third-party (OAuth) authentication flow.
//!
//! Two public entry points on [`AuthService`]:
//!
//! - [`AuthService::third_party_auth_url`] mints the authorisation URL
//!   the desktop / web client redirects the user to. Stores a single
//!   `oauth_state` row containing the encrypted PKCE verifier, the
//!   encrypted OIDC nonce, and the optional encrypted desktop-pairing
//!   `login_token`. All three live on the row; the callback never has
//!   to thread any of them through user-controlled state.
//! - [`AuthService::login_third_party`] handles the callback: consumes
//!   the state row, exchanges the code with the provider via the
//!   shared [`OAuthProviderExt`] trait, resolves / creates the user,
//!   and mints a session.
//!
//! The shared post-resolution tail (`complete_oauth_login`) runs once
//! per provider so token-issuing logic stays in lockstep. Apple's
//! form-post flow has one extra entry point —
//! [`AuthService::handle_apple_login`] — that layers a
//! `display_name_override` on top of the trait result before delegating
//! to the same tail.
//!
//! Encryption boundary: the [`OAuthProviderExt`] trait hands the
//! orchestrator decrypted [`RawOAuthTokens`]. Encryption happens here,
//! in [`AuthService::exchange_authorization_code`], so the
//! crypto-keyring dependency stays out of the provider-client layer.

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
use crate::oauth::provider_ext::{OAuthIdentityRaw, OAuthProviderExt, RawOAuthTokens};
use crate::oauth::{NewOAuthIdentity, OAuthTokenBundle};
use crate::service::{AuthService, MintedSession, user_info_from_row};
use crate::tokens::random_hex;

const OAUTH_STATE_BYTES: usize = 16; // 32 hex chars, ~128 bits of entropy

impl AuthService {
    /// Mint the authorization URL for a web/desktop OAuth start.
    ///
    /// The optional `login_token` (the desktop client's PKCE
    /// challenge) is encrypted and stamped onto the new `oauth_state`
    /// row. On callback the value is read back off the row inside
    /// `complete_oauth_login`'s pairing step — there is no user-
    /// controlled path threading it through the callback body any
    /// more.
    pub async fn third_party_auth_url(
        &self,
        provider: Provider,
        login_token: Option<String>,
    ) -> AuthResult<String> {
        tracing::info!(?provider, "Generating OAuth URL");
        let client = self.oauth_provider(provider)?;

        let state = random_hex(OAUTH_STATE_BYTES);
        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
        let nonce = Nonce::new_random();

        let url = client.authorization_url(&state, &pkce_challenge, &nonce);

        let encrypted_pkce_verifier = encrypt_sensitive_string(pkce_verifier.secret())?;
        let encrypted_nonce = Some(encrypt_sensitive_string(nonce.secret())?);
        let encrypted_login_token = login_token
            .as_deref()
            .map(encrypt_sensitive_string)
            .transpose()?;

        self.db()
            .create_oauth_state()
            .state(state)
            .pkce_verifier(encrypted_pkce_verifier)
            .redirect_uri(client.web_redirect_uri().to_string())
            .expires_at(Utc::now() + Duration::minutes(OAUTH_STATE_EXPIRY_MINUTES))
            .maybe_nonce(encrypted_nonce)
            .maybe_login_token(encrypted_login_token)
            .call()
            .await?;

        Ok(url)
    }

    /// Web/desktop OAuth callback completion. State + PKCE verifier +
    /// pairing token all come off the consumed `oauth_state` row.
    pub async fn login_third_party(
        &self,
        provider: Provider,
        code: &str,
        state: &str,
    ) -> AuthResult<MintedSession> {
        let client = self.oauth_provider(provider)?;
        let (raw, login_token) = self
            .exchange_authorization_code(client.as_ref(), code, state, false)
            .await?;
        let identity = build_new_identity(provider.into(), raw, None)?;
        self.complete_oauth_login(identity, login_token).await
    }

    /// Mobile entry point: build a provider authorisation URL whose
    /// `state` parameter equals the device's PKCE challenge, and whose
    /// callback hits the backend's mobile-callback endpoint (not the
    /// web SPA).
    ///
    /// Reusing the device challenge as OAuth `state` removes the need
    /// for a separate column on `oauth_state`: the same value
    /// double-duties as CSRF token (provider echoes it back) and as
    /// the `login_tokens` row key (the backend stamps it after a
    /// successful callback so the device's pending poll can complete).
    pub async fn mobile_third_party_auth_url(
        &self,
        provider: Provider,
        device_challenge: &str,
        challenge_method: &str,
    ) -> AuthResult<String> {
        if challenge_method != "S256" {
            return Err(AuthError::InvalidInput(
                "Only S256 PKCE method is supported".into(),
            ));
        }
        if !crate::login_token::is_valid_code_challenge(device_challenge) {
            return Err(AuthError::InvalidInput("Invalid code challenge".into()));
        }

        let client = self.oauth_provider(provider)?;
        let env_var = client.mobile_redirect_env_var();
        let mobile_redirect = client
            .mobile_redirect_uri()
            .ok_or_else(|| AuthError::OAuth(crate::oauth::OAuthError::MissingEnvVar(env_var)))?;

        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
        let nonce = Nonce::new_random();

        let url = client
            .mobile_authorization_url(device_challenge, &pkce_challenge, &nonce)
            .ok_or_else(|| AuthError::OAuth(crate::oauth::OAuthError::MissingEnvVar(env_var)))?;

        let encrypted_pkce_verifier = encrypt_sensitive_string(pkce_verifier.secret())?;
        let encrypted_nonce = Some(encrypt_sensitive_string(nonce.secret())?);

        self.db()
            .create_oauth_state()
            .state(device_challenge.to_string())
            .pkce_verifier(encrypted_pkce_verifier)
            .redirect_uri(mobile_redirect.to_string())
            .expires_at(Utc::now() + Duration::minutes(OAUTH_STATE_EXPIRY_MINUTES))
            .maybe_nonce(encrypted_nonce)
            // No login_token column for mobile — the `state` *is* the
            // pairing token; we hand it back to `complete_oauth_login`
            // explicitly in `login_third_party_mobile`.
            .maybe_login_token(None)
            .call()
            .await?;

        Ok(url)
    }

    /// Mobile callback completion: exchanges the authorisation code,
    /// runs the shared `complete_oauth_login` tail passing the
    /// device's stashed challenge (= the OAuth `state`) as the
    /// pairing `login_token`, and discards the minted session. The
    /// device picks tokens up via the existing
    /// `/auth/login-token/exchange` poll.
    pub async fn login_third_party_mobile(
        &self,
        provider: Provider,
        code: &str,
        state: &str,
    ) -> AuthResult<()> {
        let client = self.oauth_provider(provider)?;
        let (raw, _login_token_on_row) = self
            .exchange_authorization_code(client.as_ref(), code, state, true)
            .await?;
        let identity = build_new_identity(provider.into(), raw, None)?;
        // For mobile the device's challenge *is* the OAuth state;
        // pass it through as the pairing token directly. The
        // `oauth_state.login_token` column is intentionally `None`
        // for mobile rows.
        let _session = self
            .complete_oauth_login(identity, Some(state.to_string()))
            .await?;
        Ok(())
    }

    /// Apple web-callback completion.
    ///
    /// Apple's form-post flow needs a thin wrapper around
    /// [`login_third_party`] because the `user` blob (containing
    /// first/last name) is only present in the form-post body — never
    /// in the ID token, never on subsequent sign-ins. The override is
    /// only honored at first sign-in: when an existing user matches
    /// `(Apple, sub)`, `complete_oauth_login` skips the resolve-new
    /// path so the override never reaches the user row. That guard
    /// prevents a malicious client from replaying an `id_token` with
    /// a fabricated `user` field to overwrite an established display
    /// name.
    pub async fn handle_apple_login(
        &self,
        code: &str,
        state: &str,
        display_name_override: Option<String>,
    ) -> AuthResult<MintedSession> {
        let client = self.oauth_provider(Provider::Apple)?;
        let (raw, login_token) = self
            .exchange_authorization_code(client.as_ref(), code, state, false)
            .await?;
        let identity = build_new_identity(OAuthProvider::Apple, raw, display_name_override)?;
        self.complete_oauth_login(identity, login_token).await
    }

    /// Shared inner: consume an `oauth_state` row, decrypt PKCE +
    /// nonce + optional login_token, dispatch to the trait's
    /// `exchange_code` (web or mobile), and return the raw identity
    /// plus the decrypted pairing token (if any).
    ///
    /// The trait returns plaintext provider tokens; encryption to
    /// `OAuthTokenBundle` happens downstream in [`build_new_identity`].
    async fn exchange_authorization_code(
        &self,
        client: &dyn OAuthProviderExt,
        code: &str,
        state: &str,
        mobile: bool,
    ) -> AuthResult<(OAuthIdentityRaw, Option<String>)> {
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

        let provider_db = client.provider();
        let provider: Provider = provider_db.into();
        let oauth_state = self.consume_oauth_state(state, provider).await?;

        let pkce_verifier = decrypt_sensitive_string(&oauth_state.pkce_verifier)?;

        // OIDC providers (Google, Apple) require a nonce; GitHub
        // doesn't but the trait method takes one unconditionally —
        // synth a random one when the column is empty so the call
        // stays uniform.
        let nonce = match oauth_state.nonce.as_deref() {
            Some(bytes) => Nonce::new(decrypt_sensitive_string(bytes)?),
            None => Nonce::new_random(),
        };

        let login_token = oauth_state
            .login_token
            .as_deref()
            .map(decrypt_sensitive_string)
            .transpose()?;

        let raw = if mobile {
            client
                .mobile_exchange_code(code, pkce_verifier, &nonce)
                .await?
        } else {
            client.exchange_code(code, pkce_verifier, &nonce).await?
        };

        if !raw.email_verified {
            tracing::warn!(
                ?provider_db,
                email_log = ?hash_email_for_log(&raw.email),
                "OAuth login rejected: email not verified",
            );
            return Err(AuthError::EmailNotVerified);
        }

        Ok((raw, login_token))
    }

    /// Native-mobile entry point: trade a Google ID token (issued
    /// directly by the iOS / Android Google SDKs) for a session.
    ///
    /// No `oauth_state` row is touched — there's no browser round-trip,
    /// so there's no CSRF state to track. The JWT is verified locally
    /// against Google's JWKS, then `complete_oauth_login` runs the same
    /// resolve-or-create-user tail as the redirect flows.
    ///
    /// `login_token` is left as `None` because the device is the
    /// session bearer in this flow — the response carries the access /
    /// refresh tokens directly.
    pub async fn login_google_id_token(
        &self,
        id_token: &str,
        nonce: Option<String>,
    ) -> AuthResult<MintedSession> {
        if id_token.is_empty() {
            return Err(AuthError::InvalidInput("ID token is required".into()));
        }

        let google = self.google_oauth()?;
        let nonce_obj = nonce.map(Nonce::new);
        let user_info = google.verify_id_token(id_token, nonce_obj.as_ref())?;

        if !user_info.verified_email {
            tracing::warn!(
                email_log = ?hash_email_for_log(&user_info.email),
                "Google ID-token login rejected: email not verified",
            );
            return Err(AuthError::EmailNotVerified);
        }

        // Native flow yields no Google access/refresh — the
        // `oauth_credentials` row is a pure provider-link record.
        let raw = OAuthIdentityRaw {
            provider_user_id: user_info.id,
            email: user_info.email,
            email_verified: user_info.verified_email,
            display_name: user_info.display_name,
            tokens: RawOAuthTokens {
                access_token: user_info.access_token,
                refresh_token: user_info.refresh_token,
                access_token_expiry: None,
                scope: user_info.scope,
            },
        };

        let identity = build_new_identity(OAuthProvider::Google, raw, None)?;
        self.complete_oauth_login(identity, None).await
    }

    /// Consume a previously-issued `oauth_state` row, rejecting anything
    /// that isn't a fresh, unexpired match. `provider` is used only
    /// for the warn-log so SecOps can tell which callback path failed.
    async fn consume_oauth_state(
        &self,
        state: &str,
        provider: Provider,
    ) -> AuthResult<be_remote_db::OAuthState> {
        self.db()
            .consume_oauth_state()
            .state(state)
            .call()
            .await
            .map_err(|e| {
                if e.is_not_found() {
                    tracing::warn!(?provider, "Invalid or expired OAuth state");
                    AuthError::InvalidInput("Invalid or expired state parameter".into())
                } else {
                    AuthError::Database(e)
                }
            })
    }

    /// Shared tail for every OAuth-flow handler:
    ///
    /// 1. resolve / create the user (apply `display_name` only on
    ///    create — never overwrite an existing row),
    /// 2. reconcile `email_verified` with what the provider asserted,
    /// 3. attach a device-pairing login token if one was stashed on
    ///    the state row (or supplied by the mobile callback shim),
    /// 4. ensure the user has a plan row and resolve their role,
    /// 5. mint and persist an access/refresh token pair.
    ///
    /// The returned [`MintedSession`] carries
    /// [`MintedSession::was_paired`] so the Apple web-callback can
    /// communicate the pairing outcome through a top-level navigation
    /// (the SPA can't see the response body of the form-post 303).
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

        let paired = login_token.is_some();
        if let Some(token) = login_token {
            // Device-pairing failures must propagate — silently
            // dropping the row leaves the polling client waiting
            // forever.
            self.associate_login_token_with_user(&user, &token).await?;
        }

        let role = self
            .ensure_plan_and_resolve_role(user.id, &user.email)
            .await?;
        let mut session = self
            .mint_session_with_verified_override(&user, role, email_verified)
            .await?;
        if paired {
            session = session.mark_paired();
        }
        Ok(session)
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
        // Apple's web flow / native-ID-token flows hand the relying
        // party no access token; skip the update entirely rather than
        // overwriting a possibly-still-useful past value. The
        // `oauth_credentials` row is preserved as a pure provider-link
        // record.
        let Some(encrypted_access_token) = tokens.encrypted_access_token else {
            return;
        };

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
            .access_token(encrypted_access_token)
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
            .maybe_access_token(tokens.encrypted_access_token)
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

/// Encrypt provider-issued plaintext tokens into the storage shape.
///
/// Sole encryption chokepoint for the OAuth flow: the
/// [`OAuthProviderExt`] trait deliberately hands back plaintext
/// secrets so the crypto-keyring dependency stays out of the
/// provider-client layer. Anyone reaching for these encrypted bytes
/// must go through this function.
fn encrypt_token_bundle(tokens: RawOAuthTokens) -> AuthResult<OAuthTokenBundle> {
    let encrypted_access_token = tokens
        .access_token
        .as_ref()
        .map(|t| encrypt_sensitive_string(t.expose_secret()))
        .transpose()?;
    let encrypted_refresh_token = tokens
        .refresh_token
        .as_ref()
        .map(|t| encrypt_sensitive_string(t.expose_secret()))
        .transpose()?;
    Ok(OAuthTokenBundle {
        encrypted_access_token,
        encrypted_refresh_token,
        access_token_expiry: tokens.access_token_expiry,
        scope: tokens.scope,
    })
}

/// Project an `OAuthIdentityRaw` (returned by `OAuthProviderExt`,
/// plaintext) onto the encrypted [`NewOAuthIdentity`] shape
/// `complete_oauth_login` consumes.
///
/// `display_name_override` is the provider-specific override the
/// caller wants applied — Apple is the only case (today) where
/// display name doesn't come from the ID token but from the
/// form-post / native credential. When `Some`, it supersedes any
/// value the trait already set.
fn build_new_identity(
    provider: OAuthProvider,
    raw: OAuthIdentityRaw,
    display_name_override: Option<String>,
) -> AuthResult<NewOAuthIdentity> {
    let tokens = encrypt_token_bundle(raw.tokens)?;
    Ok(NewOAuthIdentity {
        provider,
        provider_user_id: raw.provider_user_id,
        email: raw.email,
        email_verified: raw.email_verified,
        display_name: display_name_override.or(raw.display_name),
        tokens,
    })
}
