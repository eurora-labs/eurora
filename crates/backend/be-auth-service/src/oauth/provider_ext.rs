//! [`OAuthProviderExt`]: per-provider variance behind the shared
//! OAuth flow.
//!
//! Implemented by [`GoogleOAuthClient`], [`GitHubOAuthClient`], and
//! [`AppleOAuthClient`]. Adding the third provider tipped the
//! enum-match-per-method approach in `oauth_flow.rs` past the point
//! where each addition was structurally cheap — the trait pulls all
//! per-provider variance into a single impl block per provider so
//! callers depend on a uniform interface.
//!
//! Layering note: the trait deliberately returns **decrypted** provider
//! tokens ([`RawOAuthTokens`]) rather than already-encrypted bytes.
//! Encryption is an orchestrator concern that needs the
//! [`crate::crypto`] keyring; doing it inside the trait would force
//! `OAuthProviderExt` to depend on the auth-service crypto layer and
//! would have to launder its `AuthError`s back into `OAuthError`s. The
//! orchestrator (see [`crate::oauth_flow`]) encrypts the bundle
//! immediately on return, before persistence.
//!
//! The trait deliberately does **not** abstract the native-ID-token
//! path (Google's `login_google_id_token`, Apple's
//! `login_apple_id_token`). Those flows differ enough between
//! providers that a shared trait method would degenerate into a
//! parameter pile-up; they live as free functions on
//! [`crate::AuthService`].

use async_trait::async_trait;
use be_remote_db::OAuthProvider;
use chrono::{DateTime, Utc};
use openidconnect::{Nonce, PkceCodeChallenge};
use secrecy::SecretString;

use crate::oauth::OAuthError;
use crate::oauth::apple::AppleOAuthClient;
use crate::oauth::github::GitHubOAuthClient;
use crate::oauth::google::GoogleOAuthClient;

/// Provider-agnostic identity shape consumed by the orchestrator in
/// [`crate::oauth_flow`]. Carries provider-issued secrets in
/// plaintext ([`SecretString`]) so the orchestrator can encrypt them
/// against the auth-service keyring in a single place.
///
/// Provider-specific overrides (Apple's `display_name_override` from
/// the form-post `user` blob) are layered on by the orchestrator
/// before this is projected into [`crate::oauth::NewOAuthIdentity`].
pub struct OAuthIdentityRaw {
    pub provider_user_id: String,
    pub email: String,
    pub email_verified: bool,
    pub display_name: Option<String>,
    /// Plaintext provider tokens. `None` `access_token` means the
    /// provider doesn't hand the relying party a usable token
    /// (Apple's web flow, Google's native ID-token flow).
    pub tokens: RawOAuthTokens,
}

/// Plaintext provider tokens awaiting encryption. The orchestrator
/// projects this into [`crate::oauth::OAuthTokenBundle`] (encrypted
/// bytes) immediately on receipt.
pub struct RawOAuthTokens {
    pub access_token: Option<SecretString>,
    pub refresh_token: Option<SecretString>,
    pub access_token_expiry: Option<DateTime<Utc>>,
    pub scope: String,
}

#[async_trait]
pub trait OAuthProviderExt: Send + Sync {
    fn provider(&self) -> OAuthProvider;

    fn web_redirect_uri(&self) -> &str;
    fn mobile_redirect_uri(&self) -> Option<&str>;

    /// Name of the environment variable that configures
    /// [`Self::mobile_redirect_uri`]. Surfaced through the trait so the
    /// orchestrator can build a `MissingEnvVar` error without
    /// re-deriving provider → env-var-name mappings.
    fn mobile_redirect_env_var(&self) -> &'static str;

    /// Build the authorization URL for the web/desktop redirect flow.
    fn authorization_url(
        &self,
        state: &str,
        pkce_challenge: &PkceCodeChallenge,
        nonce: &Nonce,
    ) -> String;

    /// Build the authorization URL for the mobile redirect flow.
    /// Returns `None` when the provider's mobile redirect URI isn't
    /// configured.
    fn mobile_authorization_url(
        &self,
        state: &str,
        pkce_challenge: &PkceCodeChallenge,
        nonce: &Nonce,
    ) -> Option<String>;

    /// Exchange an authorization code for an identity using the
    /// web-flow redirect URI.
    ///
    /// `nonce` is supplied unconditionally — providers that don't use
    /// it (currently GitHub) ignore it. Threading a single shape
    /// keeps the orchestrator branch-free.
    async fn exchange_code(
        &self,
        code: &str,
        pkce_verifier: String,
        nonce: &Nonce,
    ) -> Result<OAuthIdentityRaw, OAuthError>;

    /// Exchange against the mobile redirect URI. Returns
    /// `Err(MissingEnvVar(...))` when the mobile redirect isn't
    /// configured.
    async fn mobile_exchange_code(
        &self,
        code: &str,
        pkce_verifier: String,
        nonce: &Nonce,
    ) -> Result<OAuthIdentityRaw, OAuthError>;
}

#[async_trait]
impl OAuthProviderExt for GoogleOAuthClient {
    fn provider(&self) -> OAuthProvider {
        OAuthProvider::Google
    }

    fn web_redirect_uri(&self) -> &str {
        GoogleOAuthClient::redirect_uri(self)
    }

    fn mobile_redirect_uri(&self) -> Option<&str> {
        GoogleOAuthClient::mobile_redirect_uri(self)
    }

    fn mobile_redirect_env_var(&self) -> &'static str {
        "GOOGLE_MOBILE_REDIRECT_URI"
    }

    fn authorization_url(
        &self,
        state: &str,
        pkce_challenge: &PkceCodeChallenge,
        nonce: &Nonce,
    ) -> String {
        // The OIDC builder consumes both challenge and nonce; clone
        // here so the trait can stay reference-based.
        GoogleOAuthClient::authorization_url(self, state, pkce_challenge.clone(), nonce.clone())
    }

    fn mobile_authorization_url(
        &self,
        state: &str,
        pkce_challenge: &PkceCodeChallenge,
        nonce: &Nonce,
    ) -> Option<String> {
        GoogleOAuthClient::mobile_authorization_url(
            self,
            state,
            pkce_challenge.clone(),
            nonce.clone(),
        )
    }

    async fn exchange_code(
        &self,
        code: &str,
        pkce_verifier: String,
        nonce: &Nonce,
    ) -> Result<OAuthIdentityRaw, OAuthError> {
        let user_info = GoogleOAuthClient::exchange_code(self, code, pkce_verifier, nonce).await?;
        Ok(google_user_info_to_raw(user_info))
    }

    async fn mobile_exchange_code(
        &self,
        code: &str,
        pkce_verifier: String,
        nonce: &Nonce,
    ) -> Result<OAuthIdentityRaw, OAuthError> {
        let user_info =
            GoogleOAuthClient::mobile_exchange_code(self, code, pkce_verifier, nonce).await?;
        Ok(google_user_info_to_raw(user_info))
    }
}

#[async_trait]
impl OAuthProviderExt for GitHubOAuthClient {
    fn provider(&self) -> OAuthProvider {
        OAuthProvider::Github
    }

    fn web_redirect_uri(&self) -> &str {
        GitHubOAuthClient::redirect_uri(self)
    }

    fn mobile_redirect_uri(&self) -> Option<&str> {
        GitHubOAuthClient::mobile_redirect_uri(self)
    }

    fn mobile_redirect_env_var(&self) -> &'static str {
        "GITHUB_MOBILE_REDIRECT_URI"
    }

    fn authorization_url(
        &self,
        state: &str,
        pkce_challenge: &PkceCodeChallenge,
        _nonce: &Nonce,
    ) -> String {
        // GitHub doesn't support OIDC nonces — silently ignored.
        GitHubOAuthClient::authorization_url(self, state, pkce_challenge.as_str())
    }

    fn mobile_authorization_url(
        &self,
        state: &str,
        pkce_challenge: &PkceCodeChallenge,
        _nonce: &Nonce,
    ) -> Option<String> {
        GitHubOAuthClient::mobile_authorization_url(self, state, pkce_challenge.as_str())
    }

    async fn exchange_code(
        &self,
        code: &str,
        pkce_verifier: String,
        _nonce: &Nonce,
    ) -> Result<OAuthIdentityRaw, OAuthError> {
        let user_info = GitHubOAuthClient::exchange_code(self, code, &pkce_verifier).await?;
        Ok(github_user_info_to_raw(user_info))
    }

    async fn mobile_exchange_code(
        &self,
        code: &str,
        pkce_verifier: String,
        _nonce: &Nonce,
    ) -> Result<OAuthIdentityRaw, OAuthError> {
        let user_info = GitHubOAuthClient::mobile_exchange_code(self, code, &pkce_verifier).await?;
        Ok(github_user_info_to_raw(user_info))
    }
}

#[async_trait]
impl OAuthProviderExt for AppleOAuthClient {
    fn provider(&self) -> OAuthProvider {
        OAuthProvider::Apple
    }

    fn web_redirect_uri(&self) -> &str {
        AppleOAuthClient::web_redirect_uri(self)
    }

    fn mobile_redirect_uri(&self) -> Option<&str> {
        AppleOAuthClient::mobile_redirect_uri(self)
    }

    fn mobile_redirect_env_var(&self) -> &'static str {
        "APPLE_MOBILE_REDIRECT_URI"
    }

    fn authorization_url(
        &self,
        state: &str,
        pkce_challenge: &PkceCodeChallenge,
        nonce: &Nonce,
    ) -> String {
        AppleOAuthClient::authorization_url(self, state, pkce_challenge, nonce)
    }

    fn mobile_authorization_url(
        &self,
        state: &str,
        pkce_challenge: &PkceCodeChallenge,
        nonce: &Nonce,
    ) -> Option<String> {
        AppleOAuthClient::mobile_authorization_url(self, state, pkce_challenge, nonce)
    }

    async fn exchange_code(
        &self,
        code: &str,
        pkce_verifier: String,
        nonce: &Nonce,
    ) -> Result<OAuthIdentityRaw, OAuthError> {
        let user_info = AppleOAuthClient::exchange_code(self, code, pkce_verifier, nonce).await?;
        Ok(apple_user_info_to_raw(user_info))
    }

    async fn mobile_exchange_code(
        &self,
        code: &str,
        pkce_verifier: String,
        nonce: &Nonce,
    ) -> Result<OAuthIdentityRaw, OAuthError> {
        let user_info =
            AppleOAuthClient::mobile_exchange_code(self, code, pkce_verifier, nonce).await?;
        Ok(apple_user_info_to_raw(user_info))
    }
}

fn google_user_info_to_raw(user_info: crate::oauth::google::GoogleUserInfo) -> OAuthIdentityRaw {
    let access_token_expiry = user_info
        .expires_in
        .map(|d| Utc::now() + chrono::Duration::seconds(d.as_secs() as i64));

    OAuthIdentityRaw {
        provider_user_id: user_info.id,
        email: user_info.email,
        email_verified: user_info.verified_email,
        display_name: user_info.display_name,
        tokens: RawOAuthTokens {
            access_token: user_info.access_token,
            refresh_token: user_info.refresh_token,
            access_token_expiry,
            scope: user_info.scope,
        },
    }
}

fn github_user_info_to_raw(user_info: crate::oauth::github::GitHubUserInfo) -> OAuthIdentityRaw {
    OAuthIdentityRaw {
        provider_user_id: user_info.id,
        email: user_info.email,
        email_verified: user_info.verified_email,
        display_name: user_info.display_name,
        tokens: RawOAuthTokens {
            access_token: Some(user_info.access_token),
            refresh_token: None,
            access_token_expiry: None,
            scope: user_info.scope,
        },
    }
}

/// Apple never hands the relying party a usable access token — only
/// an ID token (verified locally) and an opaque refresh token we
/// don't persist. The token bundle is empty by design.
fn apple_user_info_to_raw(user_info: crate::oauth::apple::AppleUserInfo) -> OAuthIdentityRaw {
    OAuthIdentityRaw {
        provider_user_id: user_info.sub,
        email: user_info.email,
        email_verified: user_info.email_verified,
        // Display name is layered on by the orchestrator from the
        // form-post `user` blob; never carried by Apple's ID token.
        display_name: user_info.display_name,
        tokens: RawOAuthTokens {
            access_token: None,
            refresh_token: None,
            access_token_expiry: None,
            scope: String::new(),
        },
    }
}
