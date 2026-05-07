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
    LoginByLoginTokenRequest, LoginRequest, RegisterRequest, ThirdPartyAuthUrlRequest,
    ThirdPartyAuthUrlResponse, TokenResponse, UserResponse, VerifyEmailRequest,
};
use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use axum_extra::extract::cookie::CookieJar;
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
