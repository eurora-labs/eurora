//! Axum handlers for the auth HTTP API.
//!
//! Each handler is a thin adapter that maps a JSON request body to the
//! corresponding [`crate::AuthService`] method and serialises the
//! response. All non-2xx responses go through [`crate::AuthError`] which
//! produces the canonical [`auth_core::AuthErrorResponse`] envelope.
//!
//! Session-minting handlers dispatch on [`AuthMode`]: browser SPA
//! requests get HttpOnly cookies plus a `UserResponse` body; desktop /
//! mobile clients get the legacy `TokenResponse` body and no cookies.

use std::sync::Arc;

use auth_core::{
    AssociateLoginTokenRequest, AuthSuccessResponse, CheckEmailRequest, CheckEmailResponse,
    GoogleIdTokenLoginRequest, LoginByLoginTokenRequest, LoginRequest,
    MobileThirdPartyAuthUrlRequest, Provider, RegisterRequest, ThirdPartyAuthUrlRequest,
    ThirdPartyAuthUrlResponse, TokenResponse, UserResponse, VerifyEmailRequest,
};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Redirect},
};
use axum_extra::extract::cookie::CookieJar;
use serde::Deserialize;
use uuid::Uuid;

use crate::cookies::{self, AuthMode};
use crate::{
    AppState, AuthResult,
    auth::{AccessClaims, RefreshClaims},
    error::AuthError,
    service::{MintedSession, user_info_from_claims},
};

/// Wrap a freshly minted session in the wire-level response a given
/// client expects. Cookie-mode clients get `Set-Cookie` headers + a
/// `UserResponse` body; bearer-mode clients get a `TokenResponse` body
/// and no cookies.
fn session_response(
    state: &AppState,
    headers: &HeaderMap,
    jar: CookieJar,
    session: MintedSession,
) -> (CookieJar, Json<AuthSuccessResponse>) {
    match AuthMode::from_headers(&state.cookies, headers) {
        AuthMode::Cookie => {
            let access_max_age = session.tokens.expires_in;
            let refresh_max_age = state.jwt_config().refresh_token_expiry_days * 86_400;
            let jar = jar
                .add(cookies::access_cookie(
                    &state.cookies,
                    session.tokens.access_token,
                    access_max_age,
                ))
                .add(cookies::refresh_cookie(
                    &state.cookies,
                    session.tokens.refresh_token,
                    refresh_max_age,
                ));
            (
                jar,
                Json(AuthSuccessResponse::Cookie(UserResponse {
                    user: session.user,
                })),
            )
        }
        AuthMode::Bearer => (jar, Json(AuthSuccessResponse::Bearer(session.tokens))),
    }
}

#[tracing::instrument(skip_all)]
pub async fn login(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    jar: CookieJar,
    Json(body): Json<LoginRequest>,
) -> AuthResult<(CookieJar, Json<AuthSuccessResponse>)> {
    let session = match body {
        LoginRequest::EmailPassword { login, password } => {
            state.auth.login_email_password(&login, &password).await?
        }
        LoginRequest::ThirdParty {
            provider,
            code,
            state: oauth_state,
            login_token,
        } => {
            state
                .auth
                .login_third_party(provider, &code, &oauth_state, login_token)
                .await?
        }
    };
    Ok(session_response(&state, &headers, jar, session))
}

#[tracing::instrument(skip_all, fields(email = %body.email))]
pub async fn register(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    jar: CookieJar,
    Json(body): Json<RegisterRequest>,
) -> AuthResult<(CookieJar, Json<AuthSuccessResponse>)> {
    let session = state
        .auth
        .register_user(&body.email, &body.password, body.display_name)
        .await?;
    Ok(session_response(&state, &headers, jar, session))
}

#[tracing::instrument(skip_all)]
pub async fn refresh(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    jar: CookieJar,
    refresh: RefreshClaims,
) -> AuthResult<(CookieJar, Json<AuthSuccessResponse>)> {
    let _ = refresh.claims;
    let session = state.auth.refresh_access_token(&refresh.raw_token).await?;
    Ok(session_response(&state, &headers, jar, session))
}

#[tracing::instrument(skip_all)]
pub async fn logout(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    refresh: RefreshClaims,
) -> AuthResult<(CookieJar, StatusCode)> {
    state.auth.logout(&refresh.raw_token).await?;
    let jar = cookies::clear_all(&state.cookies, jar);
    Ok((jar, StatusCode::NO_CONTENT))
}

#[tracing::instrument(skip_all)]
pub async fn me(
    State(_state): State<Arc<AppState>>,
    AccessClaims(claims): AccessClaims,
) -> AuthResult<Json<UserResponse>> {
    let user = user_info_from_claims(&claims)?;
    Ok(Json(UserResponse { user }))
}

#[tracing::instrument(skip_all, fields(provider = ?body.provider))]
pub async fn oauth_url(
    State(state): State<Arc<AppState>>,
    Json(body): Json<ThirdPartyAuthUrlRequest>,
) -> AuthResult<Json<ThirdPartyAuthUrlResponse>> {
    let url = state.auth.third_party_auth_url(body.provider).await?;
    Ok(Json(ThirdPartyAuthUrlResponse { url }))
}

/// Mobile OAuth start: device generates a PKCE pair locally, sends the
/// challenge here, gets back the provider-authorisation URL pointing at
/// the backend's mobile-callback endpoint.
#[tracing::instrument(skip_all, fields(provider = ?body.provider))]
pub async fn mobile_oauth_url(
    State(state): State<Arc<AppState>>,
    Json(body): Json<MobileThirdPartyAuthUrlRequest>,
) -> AuthResult<Json<ThirdPartyAuthUrlResponse>> {
    let url = state
        .auth
        .mobile_third_party_auth_url(
            body.provider,
            &body.code_challenge,
            &body.code_challenge_method,
        )
        .await?;
    Ok(Json(ThirdPartyAuthUrlResponse { url }))
}

/// Query params Google / GitHub send to our mobile-callback endpoint.
/// `code` + `state` on success; `error` on user-cancel / provider rejection.
#[derive(Debug, Deserialize)]
pub struct MobileOAuthCallbackQuery {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
}

/// Path params for `/auth/oauth/{provider}/mobile-callback`.
#[derive(Debug, Deserialize)]
pub struct MobileOAuthCallbackPath {
    provider: String,
}

/// Mobile OAuth callback: invoked by the provider, *not* by the device.
///
/// Always 302s to the device's custom-scheme handler so the in-app
/// browser session resolves cleanly — even on error. The device side
/// inspects the resulting `status=` query to decide between completing
/// the login (`ok`) or surfacing a friendly error (`error`).
#[tracing::instrument(skip_all, fields(provider = %path.provider))]
pub async fn mobile_oauth_callback(
    State(state): State<Arc<AppState>>,
    Path(path): Path<MobileOAuthCallbackPath>,
    Query(query): Query<MobileOAuthCallbackQuery>,
) -> Redirect {
    let provider = match path.provider.as_str() {
        "google" => Provider::Google,
        "github" => Provider::Github,
        other => {
            tracing::warn!(provider = %other, "mobile OAuth callback for unknown provider");
            return Redirect::to(&device_redirect_error("invalid_provider"));
        }
    };

    if let Some(error) = &query.error {
        tracing::warn!(
            ?provider,
            error = %error,
            description = ?query.error_description,
            "mobile OAuth callback: provider returned error",
        );
        return Redirect::to(&device_redirect_error(error));
    }

    let (Some(code), Some(state_param)) = (query.code, query.state) else {
        tracing::warn!(?provider, "mobile OAuth callback missing code or state");
        return Redirect::to(&device_redirect_error("invalid_callback"));
    };

    match state
        .auth
        .login_third_party_mobile(provider, &code, &state_param)
        .await
    {
        Ok(()) => Redirect::to(DEVICE_REDIRECT_OK),
        Err(e) => {
            let kind = e.error_kind();
            tracing::warn!(?provider, error = %e, kind = %kind, "mobile OAuth completion failed");
            Redirect::to(&device_redirect_error(kind))
        }
    }
}

/// Custom-scheme URL the device's `ASWebAuthenticationSession` /
/// Android `BrowserSessionActivity` is bound to. The mobile crate
/// configures the in-app browser session to listen for `eurora://`
/// — the OS captures the redirect and resolves the awaited future.
const DEVICE_REDIRECT_OK: &str = "eurora://mobile/callback?status=ok";

fn device_redirect_error(kind: &str) -> String {
    let mut url = url::Url::parse("eurora://mobile/callback").expect("static URL must parse");
    url.query_pairs_mut()
        .append_pair("status", "error")
        .append_pair("error", kind);
    url.into()
}

/// Native Google sign-in: device hands us an ID token from the iOS or
/// Android Google SDK; we verify it locally and mint a session.
#[tracing::instrument(skip_all)]
pub async fn google_id_token_login(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    jar: CookieJar,
    Json(body): Json<GoogleIdTokenLoginRequest>,
) -> AuthResult<(CookieJar, Json<AuthSuccessResponse>)> {
    let session = state
        .auth
        .login_google_id_token(&body.id_token, body.nonce)
        .await?;
    Ok(session_response(&state, &headers, jar, session))
}

#[tracing::instrument(skip_all)]
pub async fn login_token_exchange(
    State(state): State<Arc<AppState>>,
    Json(body): Json<LoginByLoginTokenRequest>,
) -> AuthResult<Json<TokenResponse>> {
    // The desktop client exchanges a PKCE verifier for tokens; this
    // endpoint is bearer-only by definition (the polling client never
    // has a browser session) so no cookie dispatch is needed.
    let session = state.auth.login_by_login_token(&body.token).await?;
    Ok(Json(session.tokens))
}

#[tracing::instrument(skip_all)]
pub async fn login_token_associate(
    State(state): State<Arc<AppState>>,
    AccessClaims(claims): AccessClaims,
    Json(body): Json<AssociateLoginTokenRequest>,
) -> AuthResult<()> {
    tracing::info!(
        sub = %claims.sub,
        challenge = %body.code_challenge,
        "login_token_associate: handler entered"
    );
    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| AuthError::InvalidToken)?;
    let result = state
        .auth
        .associate_login_token(user_id, &body.code_challenge)
        .await;
    match &result {
        Ok(()) => tracing::info!(%user_id, "login_token_associate: row created"),
        Err(e) => tracing::error!(%user_id, error = %e, "login_token_associate: failed"),
    }
    result
}

#[tracing::instrument(skip_all)]
pub async fn email_check(
    State(state): State<Arc<AppState>>,
    Json(body): Json<CheckEmailRequest>,
) -> AuthResult<Json<CheckEmailResponse>> {
    let (status, provider) = state.auth.check_email(&body.email).await?;
    Ok(Json(CheckEmailResponse { status, provider }))
}

#[tracing::instrument(skip_all)]
pub async fn email_verify(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    jar: CookieJar,
    Json(body): Json<VerifyEmailRequest>,
) -> AuthResult<(CookieJar, Json<AuthSuccessResponse>)> {
    let session = state.auth.verify_email(&body.token).await?;
    Ok(session_response(&state, &headers, jar, session))
}

#[tracing::instrument(skip_all)]
pub async fn email_resend_verification(
    State(state): State<Arc<AppState>>,
    AccessClaims(claims): AccessClaims,
) -> AuthResult<()> {
    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| AuthError::InvalidToken)?;
    state.auth.resend_verification_email(user_id).await
}

// Re-exported for tests / IDE jump-to-definition; `IntoResponse` is
// implemented on the `(CookieJar, …)` tuple by axum so handlers above
// don't need an explicit `into_response()` call.
#[allow(dead_code)]
fn _into_response<T: IntoResponse>(_: T) {}
