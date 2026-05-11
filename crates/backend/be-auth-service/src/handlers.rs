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
    Form, Json,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Redirect, Response},
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
        } => {
            state
                .auth
                .login_third_party(provider, &code, &oauth_state)
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
    let url = state
        .auth
        .third_party_auth_url(body.provider, body.login_token)
        .await?;
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
        // Apple is intentionally excluded: its mobile callback uses
        // `response_mode=form_post` (POST, not GET) and is handled by a
        // separate route — see `apple_mobile_callback` (PR 3). Falling
        // through here would mismatch verb and shape.
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

/// Body Apple POSTs to `/auth/oauth/apple/web-callback`.
///
/// Apple uses `response_mode=form_post` (required when scopes include
/// `name`/`email`), so the callback arrives as
/// `application/x-www-form-urlencoded` rather than as query params on
/// a GET. The `user` field is a JSON-encoded string carrying first /
/// last name — only present on the very first sign-in; absent on
/// every subsequent one.
///
/// Apple also includes an `id_token` in this body. We deliberately
/// **don't** deserialise it: trusting an SPA-facing claim would
/// bypass the back-channel code exchange where signature + audience
/// are verified against Apple's JWKS. Serde silently drops unknown
/// fields, so the extra field is harmless on the wire.
#[derive(Debug, Deserialize)]
pub struct AppleFormPost {
    #[serde(default)]
    code: Option<String>,
    #[serde(default)]
    state: Option<String>,
    #[serde(default)]
    error: Option<String>,
    /// Stringified JSON like
    /// `{"name":{"firstName":"…","lastName":"…"},"email":"…"}`. We
    /// only consume the name portion; the email comes from the
    /// verified ID token claims, never from this untrusted body.
    #[serde(default)]
    user: Option<String>,
}

/// Apple web-callback completion.
///
/// Apple POSTs here directly (not via the SPA). On success we set the
/// session cookies and 303 the browser to `${web_base}/auth/apple/done`
/// — that SPA route rehydrates auth state from `/auth/me` and
/// completes any pending desktop-pairing redirect.
#[tracing::instrument(skip_all)]
pub async fn apple_web_callback(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    Form(form): Form<AppleFormPost>,
) -> Response {
    let web_base = state.web_base();

    if let Some(err) = form.error {
        tracing::warn!(error = %err, "Apple returned error on web-callback");
        return Redirect::to(&format!("{web_base}/login?error=oauth_failed")).into_response();
    }

    let (code, oauth_state) = match (form.code, form.state) {
        (Some(c), Some(s)) => (c, s),
        _ => {
            tracing::warn!("Apple web-callback missing code or state");
            return Redirect::to(&format!("{web_base}/login?error=invalid_callback"))
                .into_response();
        }
    };
    let display_name = parse_apple_form_user(form.user.as_deref());

    match state
        .auth
        .handle_apple_login(&code, &oauth_state, display_name)
        .await
    {
        Ok(session) => {
            let target = if session.was_paired {
                format!("{web_base}/auth/apple/done?paired=1")
            } else {
                format!("{web_base}/auth/apple/done")
            };
            apple_cookies_then_redirect(&state, jar, session, &target)
        }
        Err(e) => {
            let kind = e.error_kind();
            tracing::warn!(error = %e, kind = %kind, "Apple web-callback failed");
            Redirect::to(&format!("{web_base}/login?error={kind}")).into_response()
        }
    }
}

/// Form-post completion variant of `session_response`.
///
/// The shared `session_response` returns `(CookieJar, Json<…>)`
/// because every other session-minting endpoint responds to a
/// JSON-over-XHR caller. Apple's form-post is a top-level navigation
/// that needs cookies + a 303 to the SPA. Cookie mode is the only
/// sane mode for a browser-redirect flow, so we don't dispatch on
/// `AuthMode` here.
fn apple_cookies_then_redirect(
    state: &AppState,
    jar: CookieJar,
    session: MintedSession,
    target: &str,
) -> Response {
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
    (jar, Redirect::to(target)).into_response()
}

/// Maximum length (Unicode scalars) of a display name extracted from
/// Apple's form-post `user` blob. Apple's UI imposes a much tighter
/// limit; we cap defensively because Apple doesn't sign the `user`
/// blob.
const APPLE_DISPLAY_NAME_MAX: usize = 128;

/// Byte-size cap on the entire `user` JSON blob before we hand it to
/// `serde_json`. Apple's real payload is ~100 bytes; 4 KiB is two
/// orders of magnitude of headroom while still bounding the parser's
/// work against a maliciously fabricated body. Axum has its own outer
/// body-size limit but it applies to the whole form, not to this
/// inner field specifically.
const APPLE_FORM_USER_MAX_BYTES: usize = 4096;

/// Typed projection of the Apple form-post `user` blob. Apple's
/// payload looks like:
/// `{"name":{"firstName":"…","lastName":"…"},"email":"…"}`. We only
/// consume the name portion; the email is trusted only via the
/// verified ID-token claim, never via this body.
#[derive(Debug, Deserialize)]
struct AppleFormUserBlob {
    name: Option<AppleFormUserName>,
    // Apple also sends `email` here on first sign-in. Intentionally
    // not deserialised: trusting an unsigned blob's email field would
    // be an account-takeover vector. Treating it as unknown-and-
    // ignored (rather than `#[serde(deny_unknown_fields)]`) keeps
    // forwards-compat with future Apple additions.
}

#[derive(Debug, Deserialize)]
struct AppleFormUserName {
    #[serde(rename = "firstName", default)]
    first_name: Option<String>,
    #[serde(rename = "lastName", default)]
    last_name: Option<String>,
}

/// Project the Apple form-post `user` blob onto an `Option<String>`
/// display-name override.
///
/// Treats the input as fully untrusted: rejects oversize bodies,
/// oversize or control-character-bearing names, and returns `None`
/// for any structural failure rather than propagating an error (a
/// missing display name must not fail the login).
pub(crate) fn parse_apple_form_user(raw: Option<&str>) -> Option<String> {
    let raw = raw?;
    if raw.is_empty() || raw.len() > APPLE_FORM_USER_MAX_BYTES {
        return None;
    }
    let blob: AppleFormUserBlob = serde_json::from_str(raw).ok()?;
    let name = blob.name?;
    let first = name.first_name.as_deref().unwrap_or("");
    let last = name.last_name.as_deref().unwrap_or("");
    let combined = format!("{first} {last}").trim().to_string();
    if combined.is_empty() {
        return None;
    }
    // Reject control characters (XSS / log-injection vectors) and cap
    // length. Don't HTML-escape here — that's the rendering layer's
    // job, and pre-escaping in storage corrupts user names that
    // legitimately contain `&` / `<`.
    if combined.chars().any(char::is_control) {
        return None;
    }
    if combined.chars().count() > APPLE_DISPLAY_NAME_MAX {
        return None;
    }
    Some(combined)
}

// Re-exported for tests / IDE jump-to-definition; `IntoResponse` is
// implemented on the `(CookieJar, …)` tuple by axum so handlers above
// don't need an explicit `into_response()` call.
#[allow(dead_code)]
fn _into_response<T: IntoResponse>(_: T) {}

#[cfg(test)]
mod tests {
    use super::parse_apple_form_user;

    #[test]
    fn returns_none_for_no_input() {
        assert!(parse_apple_form_user(None).is_none());
    }

    #[test]
    fn returns_none_for_empty_string() {
        assert!(parse_apple_form_user(Some("")).is_none());
    }

    #[test]
    fn returns_none_for_malformed_json() {
        assert!(parse_apple_form_user(Some("{not json")).is_none());
    }

    #[test]
    fn returns_none_when_name_key_missing() {
        let raw = r#"{"email":"u@e.com"}"#;
        assert!(parse_apple_form_user(Some(raw)).is_none());
    }

    #[test]
    fn returns_none_when_both_names_are_empty() {
        let raw = r#"{"name":{"firstName":"","lastName":""}}"#;
        assert!(parse_apple_form_user(Some(raw)).is_none());
    }

    #[test]
    fn returns_none_when_firstname_is_null() {
        let raw = r#"{"name":{"firstName":null,"lastName":null}}"#;
        assert!(parse_apple_form_user(Some(raw)).is_none());
    }

    #[test]
    fn rejects_non_string_name_field() {
        // Typed deserialization rejects the whole blob if any name
        // field has the wrong shape. Better than silently falling
        // back to the well-typed half: a body that violates the
        // contract is more likely to be tampered with than partially
        // wrong.
        let raw = r#"{"name":{"firstName":42,"lastName":"Doe"}}"#;
        assert!(parse_apple_form_user(Some(raw)).is_none());
    }

    #[test]
    fn handles_first_only() {
        let raw = r#"{"name":{"firstName":"Ada","lastName":""}}"#;
        assert_eq!(parse_apple_form_user(Some(raw)), Some("Ada".to_string()));
    }

    #[test]
    fn handles_last_only() {
        let raw = r#"{"name":{"firstName":"","lastName":"Lovelace"}}"#;
        assert_eq!(
            parse_apple_form_user(Some(raw)),
            Some("Lovelace".to_string())
        );
    }

    #[test]
    fn handles_unicode() {
        let raw = r#"{"name":{"firstName":"José","lastName":"García"}}"#;
        assert_eq!(
            parse_apple_form_user(Some(raw)),
            Some("José García".to_string())
        );
    }

    #[test]
    fn handles_embedded_quotes() {
        let raw = r#"{"name":{"firstName":"O\"Brien","lastName":"Smith"}}"#;
        assert_eq!(
            parse_apple_form_user(Some(raw)),
            Some(r#"O"Brien Smith"#.to_string())
        );
    }

    #[test]
    fn rejects_control_characters() {
        let raw = r#"{"name":{"firstName":"Ada\nLovelace","lastName":""}}"#;
        assert!(parse_apple_form_user(Some(raw)).is_none());
    }

    #[test]
    fn rejects_oversize_display_name() {
        let huge = "a".repeat(super::APPLE_DISPLAY_NAME_MAX + 1);
        let raw = format!(r#"{{"name":{{"firstName":"{huge}","lastName":""}}}}"#);
        assert!(parse_apple_form_user(Some(&raw)).is_none());
    }

    #[test]
    fn rejects_oversize_input_before_parsing() {
        // Outer-body cap kicks in before the JSON parser sees the
        // input — verify a malformed (would-otherwise-error) body
        // larger than the cap returns None without crashing the
        // parser on something a forgery could exploit.
        let huge = "x".repeat(super::APPLE_FORM_USER_MAX_BYTES + 1);
        assert!(parse_apple_form_user(Some(&huge)).is_none());
    }
}
