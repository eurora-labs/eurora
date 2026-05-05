//! Axum handlers for the auth HTTP API.
//!
//! Each handler is a thin adapter that maps a JSON request body to the
//! corresponding [`crate::AuthService`] method and serialises the
//! response. All non-2xx responses go through [`crate::AuthError`] which
//! produces the canonical [`auth_core::AuthErrorResponse`] envelope.

use std::sync::Arc;

use auth_core::{
    AssociateLoginTokenRequest, CheckEmailRequest, CheckEmailResponse, LoginByLoginTokenRequest,
    LoginRequest, RegisterRequest, ThirdPartyAuthUrlRequest, ThirdPartyAuthUrlResponse,
    TokenResponse, VerifyEmailRequest,
};
use axum::{Json, extract::State};
use uuid::Uuid;

use crate::{
    AppState, AuthResult,
    auth::{AccessClaims, RefreshClaims},
    error::AuthError,
};

#[tracing::instrument(skip_all)]
pub async fn login(
    State(state): State<Arc<AppState>>,
    Json(body): Json<LoginRequest>,
) -> AuthResult<Json<TokenResponse>> {
    let resp = match body {
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
    Ok(Json(resp))
}

#[tracing::instrument(skip_all, fields(email = %body.email))]
pub async fn register(
    State(state): State<Arc<AppState>>,
    Json(body): Json<RegisterRequest>,
) -> AuthResult<Json<TokenResponse>> {
    let resp = state
        .auth
        .register_user(&body.email, &body.password, body.display_name)
        .await?;
    Ok(Json(resp))
}

#[tracing::instrument(skip_all)]
pub async fn refresh(
    State(state): State<Arc<AppState>>,
    refresh: RefreshClaims,
) -> AuthResult<Json<TokenResponse>> {
    let resp = state.auth.refresh_access_token(&refresh.raw_token).await?;
    Ok(Json(resp))
}

#[tracing::instrument(skip_all)]
pub async fn logout(State(state): State<Arc<AppState>>, refresh: RefreshClaims) -> AuthResult<()> {
    state.auth.logout(&refresh.raw_token).await
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
    let resp = state.auth.login_by_login_token(&body.token).await?;
    Ok(Json(resp))
}

#[tracing::instrument(skip_all)]
pub async fn login_token_associate(
    State(state): State<Arc<AppState>>,
    AccessClaims(claims): AccessClaims,
    Json(body): Json<AssociateLoginTokenRequest>,
) -> AuthResult<()> {
    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| AuthError::InvalidToken)?;
    state
        .auth
        .associate_login_token(user_id, &body.code_challenge)
        .await
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
    Json(body): Json<VerifyEmailRequest>,
) -> AuthResult<Json<TokenResponse>> {
    let resp = state.auth.verify_email(&body.token).await?;
    Ok(Json(resp))
}

#[tracing::instrument(skip_all)]
pub async fn email_resend_verification(
    State(state): State<Arc<AppState>>,
    AccessClaims(claims): AccessClaims,
) -> AuthResult<()> {
    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| AuthError::InvalidToken)?;
    state.auth.resend_verification_email(user_id).await
}
