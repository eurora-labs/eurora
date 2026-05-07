//! `AuthService` core type and Axum-shared `AppState`.
//!
//! Per-flow methods live in sibling modules (`password_auth`, `refresh`,
//! `oauth_flow`, `email_verification`, `login_token`, `email_check`,
//! `plans`) — each adds its own `impl AuthService` block. This file is
//! the home of the struct definition, dependency wiring, and the
//! accessor methods those flows use to reach back into the shared
//! state.

use std::sync::Arc;

use auth_core::{Claims, Role, TokenResponse, UserInfo};
use be_auth_core::JwtConfig;
use be_email_service::EmailService;
use be_remote_db::DatabaseManager;
use uuid::Uuid;

use crate::cookies::CookieConfig;
use crate::error::{AuthError, AuthResult};
use crate::oauth::github::GitHubOAuthClient;
use crate::oauth::google::GoogleOAuthClient;
use crate::oauth::{OAuthError, github, google};

/// A freshly minted session: the bearer-mode token envelope alongside
/// the public user profile. Handlers serialise one or the other (or
/// both, with the access token going to a cookie) depending on
/// [`crate::cookies::AuthMode`].
pub struct MintedSession {
    pub tokens: TokenResponse,
    pub user: UserInfo,
}

impl MintedSession {
    pub(crate) fn new(tokens: TokenResponse, user: UserInfo) -> Self {
        Self { tokens, user }
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
    google_oauth_client: Option<GoogleOAuthClient>,
    github_oauth_client: Option<GitHubOAuthClient>,
}

#[derive(Default)]
pub struct AuthServiceConfig {
    pub google: Option<GoogleOAuthClient>,
    pub github: Option<GitHubOAuthClient>,
}

impl AuthService {
    pub fn new(
        db: Arc<DatabaseManager>,
        jwt_config: JwtConfig,
        email_service: Option<Arc<EmailService>>,
        oauth_clients: AuthServiceConfig,
    ) -> Self {
        Self {
            db,
            jwt_config,
            email_service,
            google_oauth_client: oauth_clients.google,
            github_oauth_client: oauth_clients.github,
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

    pub(crate) fn google_oauth(&self) -> AuthResult<&GoogleOAuthClient> {
        self.google_oauth_client.as_ref().ok_or_else(|| {
            AuthError::OAuth(OAuthError::MissingEnvVar(
                "GOOGLE_CLIENT_ID/GOOGLE_CLIENT_SECRET/GOOGLE_REDIRECT_URI",
            ))
        })
    }

    pub(crate) fn github_oauth(&self) -> AuthResult<&GitHubOAuthClient> {
        self.github_oauth_client.as_ref().ok_or_else(|| {
            AuthError::OAuth(OAuthError::MissingEnvVar(
                "GITHUB_CLIENT_ID/GITHUB_CLIENT_SECRET/GITHUB_REDIRECT_URI",
            ))
        })
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

    Ok(AuthServiceConfig { google, github })
}
