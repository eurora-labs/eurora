//! HTTP authentication service.
//!
//! Exposes an Axum router under `/auth` that handles email+password and
//! third-party (Google, GitHub) authentication, refresh-token rotation,
//! email verification, and the device-pairing login-token flow.
//!
//! Unlike the activity / asset services, the global `authz_middleware`
//! bypasses the `/auth/*` prefix entirely so unauthenticated callers
//! can reach login / register / refresh. Routes that *do* require a
//! token validate it inline via the [`auth::AccessClaims`] /
//! [`auth::RefreshClaims`] extractors using the shared
//! [`be_auth_core::JwtConfig`].

pub mod apple_notifications;
pub mod auth;
pub mod cookies;
pub mod crypto;
mod email_check;
mod email_verification;
pub mod error;
pub mod handlers;
mod log_redaction;
mod login_token;
pub mod oauth;
mod oauth_flow;
mod password_auth;
mod passwords;
mod plans;
mod refresh;
pub mod service;
mod tokens;

use std::sync::Arc;

use anyhow::Result;
use axum::{
    Router,
    routing::{get, post},
};
use be_auth_core::JwtConfig;
use be_email_service::EmailService;
use be_remote_db::DatabaseManager;
use tower_http::trace::TraceLayer;

pub use cookies::{ACCESS_COOKIE, AuthMode, CookieConfig, CookieConfigError, REFRESH_COOKIE};
pub use error::{AuthError, AuthResult};
pub use service::{AppState, AuthService, AuthServiceConfig, build_oauth_clients};

pub use auth_core::{Claims, Role};
pub use oauth::{NewOAuthIdentity, OAuthError, OAuthTokenBundle};

/// Login-token TTL — long enough for the desktop client to round-trip a
/// browser-based OAuth flow on a slow network, short enough that an
/// abandoned pairing doesn't sit on the table indefinitely.
pub(crate) const LOGIN_TOKEN_EXPIRY_MINUTES: i64 = 20;

/// OAuth-state TTL. The window between "user clicked sign in" and "user
/// hits our callback" is normally a few seconds; ten minutes gives
/// generous headroom for slow networks / redirect chains.
pub(crate) const OAUTH_STATE_EXPIRY_MINUTES: i64 = 10;

pub(crate) const VERIFICATION_TOKEN_EXPIRY_HOURS: i64 = 24;
pub(crate) const VERIFICATION_RESEND_COOLDOWN_SECONDS: i64 = 60;

/// Bytes of OS randomness in a verification token (hex-encoded; the
/// final string is twice this length).
pub(crate) const VERIFICATION_TOKEN_BYTES: usize = 32;

/// PostgreSQL auto-generates this name for the `UNIQUE` constraint on
/// `users.email` (format: `<table>_<column>_key`). If the column or
/// constraint is ever renamed, this constant must be updated to match.
pub(crate) const USERS_EMAIL_UNIQUE_CONSTRAINT: &str = "users_email_key";

/// Build the auth router with the supplied dependencies.
///
/// Returns the bare router; the caller is expected to apply the
/// cross-cutting layers (CORS, body limit, auth-middleware bypass for
/// `/auth/*`) at the monolith level so all REST services share the same
/// outer pipeline.
pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/auth/login", post(handlers::login))
        .route("/auth/register", post(handlers::register))
        .route("/auth/refresh", post(handlers::refresh))
        .route("/auth/logout", post(handlers::logout))
        .route("/auth/me", get(handlers::me))
        .route("/auth/oauth/url", post(handlers::oauth_url))
        // Mobile OAuth: device hits `/auth/oauth/mobile/url` with its
        // PKCE challenge, opens the returned authorisation URL in an
        // in-app browser, then the provider 302s the user to
        // `/auth/oauth/{provider}/mobile-callback` — which completes
        // login and 302s to the device's `eurora://` deep-link.
        .route("/auth/oauth/mobile/url", post(handlers::mobile_oauth_url))
        .route(
            "/auth/oauth/{provider}/mobile-callback",
            get(handlers::mobile_oauth_callback),
        )
        // Native Google sign-in (iOS GoogleSignIn SDK / Android
        // Credential Manager): device hands us an ID token that the
        // backend verifies against Google's JWKS. No browser involved.
        .route(
            "/auth/oauth/google/id-token",
            post(handlers::google_id_token_login),
        )
        // Apple Sign In web-callback. Apple form-posts here directly
        // (not via the SPA) using `response_mode=form_post`. The
        // handler sets session cookies and 303s to the SPA success
        // page. The mobile-callback / native-iOS routes land in
        // later PRs.
        .route(
            "/auth/oauth/apple/web-callback",
            post(handlers::apple_web_callback),
        )
        // Apple Sign In server-to-server notifications. Apple POSTs
        // a single `application/x-www-form-urlencoded` body with a
        // signed JWT payload to inform us of consent revocation,
        // account deletion, and Hide-My-Email flag toggles. The
        // handler verifies the JWT and tears down the user's Apple
        // sessions when applicable — see
        // [`apple_notifications`] for the full status-code policy.
        .route(
            "/auth/oauth/apple/notifications",
            post(handlers::apple_notifications),
        )
        .route(
            "/auth/login-token/exchange",
            post(handlers::login_token_exchange),
        )
        .route(
            "/auth/login-token/associate",
            post(handlers::login_token_associate),
        )
        .route("/auth/email/check", post(handlers::email_check))
        .route("/auth/email/verify", post(handlers::email_verify))
        .route(
            "/auth/email/resend-verification",
            post(handlers::email_resend_verification),
        )
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

/// Convenience constructor mirroring `be-payment-service::init_payment_service`
/// and `be-activity-service::init_activity_service`. Eagerly constructs
/// all dependencies (including OAuth client discovery) so the binary
/// fails fast on misconfiguration instead of surfacing it on the first
/// request.
pub async fn init_auth_service(
    db: Arc<DatabaseManager>,
    jwt_config: JwtConfig,
    email_service: Option<Arc<EmailService>>,
    cookie_config: CookieConfig,
) -> Result<Router> {
    tracing::debug!("Initializing auth service");
    let oauth_clients = build_oauth_clients().await?;
    // Cross-config validation: cookie scope + OAuth providers must
    // agree (e.g. Apple form-post needs at least one SPA web origin
    // to redirect to). Surface misconfigurations at boot rather than
    // on the first sign-in.
    oauth_clients.validate(&cookie_config)?;
    let auth = AuthService::new(db, jwt_config, email_service, oauth_clients);
    let state = Arc::new(AppState::new(auth, cookie_config));
    Ok(create_router(state))
}
