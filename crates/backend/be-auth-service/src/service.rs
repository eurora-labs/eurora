//! `AuthService` core type and Axum-shared `AppState`.
//!
//! Per-flow methods live in sibling modules (`password_auth`, `refresh`,
//! `oauth_flow`, `email_verification`, `login_token`, `email_check`,
//! `plans`) — each adds its own `impl AuthService` block. This file is
//! the home of the struct definition, dependency wiring, and the
//! accessor methods those flows use to reach back into the shared
//! state.

use std::collections::HashMap;
use std::sync::Arc;

use auth_core::{Claims, Provider, Role, TokenResponse, UserInfo};
use be_auth_core::JwtConfig;
use be_email_service::EmailService;
use be_remote_db::DatabaseManager;
use uuid::Uuid;

use crate::cookies::CookieConfig;
use crate::error::{AuthError, AuthResult};
use crate::oauth::apple::AppleOAuthClient;
use crate::oauth::github::GitHubOAuthClient;
use crate::oauth::google::GoogleOAuthClient;
use crate::oauth::provider_ext::OAuthProviderExt;
use crate::oauth::{OAuthError, apple, github, google};

/// A freshly minted session: the bearer-mode token envelope alongside
/// the public user profile. Handlers serialise one or the other (or
/// both, with the access token going to a cookie) depending on
/// [`crate::cookies::AuthMode`].
pub struct MintedSession {
    pub tokens: TokenResponse,
    pub user: UserInfo,
    /// True iff the OAuth-completion tail attached a desktop-pairing
    /// `login_token` during this minting. Drives the `?paired=1`
    /// query marker on the Apple web-callback redirect — the SPA
    /// uses it as a signal to consume any stashed `redirect_uri` and
    /// hand control back to the desktop client.
    ///
    /// Defaults to `false` everywhere except the OAuth completion
    /// path; non-OAuth session mints (password login, email
    /// verification, refresh) leave it unchanged.
    pub was_paired: bool,
}

impl MintedSession {
    pub(crate) fn new(tokens: TokenResponse, user: UserInfo) -> Self {
        Self {
            tokens,
            user,
            was_paired: false,
        }
    }

    /// Flag this session as the result of a desktop-pairing flow.
    /// Callers invoke this only when they've already attached a
    /// `login_tokens` row to the user; the boolean isn't taken as a
    /// parameter because "mark unpaired" would be a meaningless
    /// operation (the constructor's default is already `false`).
    pub(crate) fn mark_paired(mut self) -> Self {
        self.was_paired = true;
        self
    }
}

pub(crate) fn user_info_from_row(
    user: &be_remote_db::User,
    role: Role,
    email_verified: bool,
) -> UserInfo {
    UserInfo {
        id: user.id.to_string(),
        email: user.email.clone(),
        display_name: user.display_name.clone(),
        email_verified,
        role,
    }
}

pub(crate) fn user_info_from_claims(claims: &Claims) -> AuthResult<UserInfo> {
    Uuid::parse_str(&claims.sub).map_err(|_| AuthError::InvalidToken)?;
    Ok(UserInfo {
        id: claims.sub.clone(),
        email: claims.email.clone(),
        display_name: claims.display_name.clone(),
        email_verified: claims.email_verified,
        role: claims.role.clone(),
    })
}

/// Shared state injected into Axum handlers via `State<Arc<AppState>>`.
pub struct AppState {
    pub auth: AuthService,
    pub cookies: CookieConfig,
}

impl AppState {
    pub fn new(auth: AuthService, cookies: CookieConfig) -> Self {
        Self { auth, cookies }
    }
}

/// Shorthand: `state.jwt_config()` → `state.auth.jwt_config()`.
impl AppState {
    pub fn jwt_config(&self) -> &JwtConfig {
        self.auth.jwt_config()
    }

    /// Web base URL for redirect targets emitted by the Apple
    /// form-post handler. Sourced from the SPA origin allow-list:
    /// every entry in `web_origins` is the *exact* URL the SPA
    /// publishes itself as (scheme + host + optional port). We pick
    /// deterministically — sorting and taking the first — so the
    /// choice is stable across restarts.
    ///
    /// In production there is exactly one SPA origin
    /// (`https://www.eurora-labs.com`); local dev may have one or
    /// two.
    ///
    /// **Precondition**: `web_origins` is non-empty whenever Apple is
    /// configured. Enforced at startup in
    /// [`crate::init_auth_service`]; if anyone reaches this with an
    /// empty allow-list they bypassed that validator, so we emit a
    /// loud `error!` and return an empty string. Concatenated with
    /// the suffix (`"/login?error=..."`) it produces a relative URL
    /// the browser interprets as same-origin — `api.eurora-labs.com`
    /// for the form-post handler — which is wrong but visibly so,
    /// rather than a malformed `https:///` URL the browser silently
    /// mangles.
    pub(crate) fn web_base(&self) -> String {
        let mut origins: Vec<&String> = self.cookies.web_origins.iter().collect();
        origins.sort();
        match origins.first() {
            Some(origin) => origin.trim_end_matches('/').to_string(),
            None => {
                tracing::error!(
                    "AppState::web_base called with empty web_origins; this is a startup misconfiguration"
                );
                String::new()
            }
        }
    }
}

/// Owns all backend dependencies needed by the auth flows. Constructed
/// once at boot — OAuth clients are eagerly built so missing
/// configuration fails fast (instead of surfacing at the first OAuth
/// click) and so the OIDC discovery round-trip happens before we start
/// serving traffic.
pub struct AuthService {
    db: Arc<DatabaseManager>,
    jwt_config: JwtConfig,
    email_service: Option<Arc<EmailService>>,
    /// Trait-object view of the OAuth clients, indexed by
    /// [`Provider`]. Built once at startup so per-request dispatch is
    /// a single hash lookup. Owning `Arc`s here means dropping
    /// `AuthService` drops the underlying clients.
    oauth_providers: HashMap<Provider, Arc<dyn OAuthProviderExt>>,
    /// Concrete-typed handle on the Google client, kept alongside the
    /// trait-object map because the native ID-token flow (see
    /// [`Self::login_google_id_token`]) calls
    /// [`GoogleOAuthClient::verify_id_token`] — a method outside the
    /// [`OAuthProviderExt`] surface.
    google_oauth_client: Option<Arc<GoogleOAuthClient>>,
    /// Concrete-typed handle on the Apple client.
    ///
    /// Mirrors [`Self::google_oauth_client`]: the
    /// [`OAuthProviderExt`] trait covers the redirect-flow surface
    /// but Apple has two methods that don't fit it —
    /// `verify_id_token` for the native iOS path (lands in a later
    /// PR) and `verify_notification` for the server-to-server
    /// notifications path. Reaching for the concrete client here
    /// avoids smearing those methods onto the trait with `Option` /
    /// `unused_variables` for the other providers.
    apple_oauth_client: Option<Arc<AppleOAuthClient>>,
}

#[derive(Default)]
pub struct AuthServiceConfig {
    pub google: Option<GoogleOAuthClient>,
    pub github: Option<GitHubOAuthClient>,
    pub apple: Option<AppleOAuthClient>,
}

impl AuthServiceConfig {
    /// Validate that the OAuth provider set is consistent with the
    /// surrounding [`CookieConfig`].
    ///
    /// Today the only cross-config rule is: **Apple Sign In requires
    /// at least one SPA origin in `web_origins`.** Apple form-posts
    /// to the backend and the backend 303s to the SPA at
    /// `${web_origin}/auth/apple/done`. With no web origin we have
    /// nowhere to redirect, and the SPA would never see the cookies
    /// either. Failing here makes the misconfiguration loud at
    /// startup instead of producing broken `https:///…` redirects on
    /// the first sign-in.
    pub fn validate(&self, cookies: &CookieConfig) -> Result<(), OAuthError> {
        if self.apple.is_some() && cookies.web_origins.is_empty() {
            return Err(OAuthError::InvalidConfig(
                "Apple Sign In is configured but no SPA web origins are set; \
                 set BACKEND_URL/WEB_URL so the form-post handler has somewhere to redirect",
            ));
        }
        Ok(())
    }
}

impl AuthService {
    pub fn new(
        db: Arc<DatabaseManager>,
        jwt_config: JwtConfig,
        email_service: Option<Arc<EmailService>>,
        oauth_clients: AuthServiceConfig,
    ) -> Self {
        let google_oauth_client = oauth_clients.google.map(Arc::new);
        let github_oauth_client = oauth_clients.github.map(Arc::new);
        let apple_oauth_client = oauth_clients.apple.map(Arc::new);

        let mut oauth_providers: HashMap<Provider, Arc<dyn OAuthProviderExt>> = HashMap::new();
        if let Some(g) = &google_oauth_client {
            oauth_providers.insert(Provider::Google, g.clone());
        }
        if let Some(g) = github_oauth_client {
            oauth_providers.insert(Provider::Github, g);
        }
        if let Some(a) = &apple_oauth_client {
            oauth_providers.insert(Provider::Apple, a.clone());
        }

        Self {
            db,
            jwt_config,
            email_service,
            oauth_providers,
            google_oauth_client,
            apple_oauth_client,
        }
    }

    pub(crate) fn db(&self) -> &Arc<DatabaseManager> {
        &self.db
    }

    pub fn jwt_config(&self) -> &JwtConfig {
        &self.jwt_config
    }

    pub(crate) fn email_service(&self) -> Option<&Arc<EmailService>> {
        self.email_service.as_ref()
    }

    /// Dev mode is tied to the build profile: debug builds skip
    /// payment/email/update wiring, release builds do not. There is no
    /// runtime override.
    pub(crate) fn dev_mode(&self) -> bool {
        cfg!(debug_assertions)
    }

    /// Concrete-typed access to the Google client, needed for the
    /// native ID-token path (which calls `verify_id_token`, a method
    /// outside the [`OAuthProviderExt`] surface). Most callers should
    /// reach for [`Self::oauth_provider`] instead.
    pub(crate) fn google_oauth(&self) -> AuthResult<&GoogleOAuthClient> {
        self.google_oauth_client.as_deref().ok_or_else(|| {
            AuthError::OAuth(OAuthError::MissingEnvVar(missing_env_var_hint(
                Provider::Google,
            )))
        })
    }

    /// Concrete-typed access to the Apple client. Used by the
    /// notifications handler (and, later, the native iOS path) to
    /// reach `verify_notification` / `verify_id_token` — methods
    /// outside [`OAuthProviderExt`]. The error path mirrors
    /// [`Self::google_oauth`] so callers see a consistent "Apple is
    /// not configured" message regardless of entry point.
    pub(crate) fn apple_oauth(&self) -> AuthResult<&AppleOAuthClient> {
        self.apple_oauth_client.as_deref().ok_or_else(|| {
            AuthError::OAuth(OAuthError::MissingEnvVar(missing_env_var_hint(
                Provider::Apple,
            )))
        })
    }

    /// Look up the [`OAuthProviderExt`] trait object for `provider`,
    /// or return a clean `MissingEnvVar` describing what to set if
    /// the provider isn't configured.
    pub(crate) fn oauth_provider(
        &self,
        provider: Provider,
    ) -> AuthResult<Arc<dyn OAuthProviderExt>> {
        match self.oauth_providers.get(&provider) {
            Some(p) => Ok(p.clone()),
            // Provider not configured. We emit the same env-var hint
            // the typed accessors emit so operators see a consistent
            // remediation message — but as a direct error
            // construction, not via the typed accessor (which would
            // return `Ok` once the map entry exists, leaving the
            // caller to invent a never-reached fallback).
            None => Err(AuthError::OAuth(OAuthError::MissingEnvVar(
                missing_env_var_hint(provider),
            ))),
        }
    }
}

/// Env-var hint surfaced to operators when a provider is requested
/// but not configured. Kept next to the typed accessors so all three
/// hints live in one place and stay in lockstep.
fn missing_env_var_hint(provider: Provider) -> &'static str {
    match provider {
        Provider::Google => "GOOGLE_CLIENT_ID/GOOGLE_CLIENT_SECRET/GOOGLE_REDIRECT_URI",
        Provider::Github => "GITHUB_CLIENT_ID/GITHUB_CLIENT_SECRET/GITHUB_REDIRECT_URI",
        Provider::Apple => {
            "APPLE_TEAM_ID/APPLE_SERVICE_ID/APPLE_KEY_ID/APPLE_PRIVATE_KEY/APPLE_WEB_REDIRECT_URI"
        }
    }
}

/// Eagerly construct the OAuth clients from environment configuration.
/// A provider is `None` only if its mandatory env vars are absent —
/// callers attempting to use that provider get a clear `MissingEnvVar`
/// error instead of a 500. Any other discovery / network failure
/// propagates so the binary refuses to start, matching the rest of the
/// monolith's "fail-fast at boot" policy.
pub async fn build_oauth_clients() -> Result<AuthServiceConfig, OAuthError> {
    let google = match google::GoogleOAuthConfig::from_env() {
        Ok(cfg) => Some(GoogleOAuthClient::discover(cfg).await?),
        Err(OAuthError::MissingEnvVar(_)) => {
            tracing::info!("Google OAuth not configured; skipping");
            None
        }
        Err(e) => return Err(e),
    };

    let github = match github::GitHubOAuthConfig::from_env() {
        Ok(cfg) => Some(GitHubOAuthClient::new(cfg)?),
        Err(OAuthError::MissingEnvVar(_)) => {
            tracing::info!("GitHub OAuth not configured; skipping");
            None
        }
        Err(e) => return Err(e),
    };

    let apple = match apple::AppleOAuthConfig::from_env() {
        Ok(cfg) => Some(AppleOAuthClient::discover(cfg).await?),
        Err(OAuthError::MissingEnvVar(_)) => {
            tracing::info!("Apple OAuth not configured; skipping");
            None
        }
        Err(e) => return Err(e),
    };

    Ok(AuthServiceConfig {
        google,
        github,
        apple,
    })
}
