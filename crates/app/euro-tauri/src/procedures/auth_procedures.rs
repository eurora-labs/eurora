use auth_core::Claims;
use euro_secret::{ExposeSecret, SecretString, secret};
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, Manager};
use tauri_specta::Event;
use thiserror::Error;
use url::Url;

use crate::procedures::{auth_manager, user_controller};
use crate::shared_types::SharedSettingsState;

/// Typed error surface for the `auth_*` IPC commands. Externally tagged
/// so the JS side gets `{ type: "NotAuthenticated" }` and can branch on
/// the variant. The split between `NotAuthenticated` (the user must sign
/// in again) and `Backend` (transient — local credentials are intact)
/// matches `euro_auth::AuthError::is_logged_out` / `is_transient`, so
/// the UI can decide whether to bounce the user to the login screen or
/// just toast and retry.
#[derive(Debug, Error, Serialize, Type)]
#[serde(tag = "type", content = "data")]
pub enum AuthError {
    #[error("not authenticated")]
    NotAuthenticated,
    #[error("backend unreachable: {0}")]
    Backend(String),
    #[error("login token expired")]
    LoginTokenExpired,
    #[error("config: {0}")]
    Config(String),
    #[error("persistence: {0}")]
    Persistence(String),
    #[error("state unavailable: {0}")]
    StateUnavailable(&'static str),
    #[error("internal: {0}")]
    Internal(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct LoginToken {
    pub code_challenge: String,
    /// Seconds remaining until the login challenge expires. `u32` so the TS
    /// bindings stay on plain `number` — twenty-minute TTLs fit trivially
    /// and there's no scenario where this needs more than 32 bits.
    pub expires_in: u32,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct AuthStateChanged {
    pub claims: Option<Claims>,
}

const LOGIN_CODE_VERIFIER: &str = "LOGIN_CODE_VERIFIER";

fn emit_auth_state(app_handle: &AppHandle, claims: Option<Claims>) {
    if let Err(e) = (AuthStateChanged { claims }).emit(app_handle) {
        tracing::warn!("Failed to emit auth state event: {e}");
    }
}

async fn resolve_auth_manager(app_handle: &AppHandle) -> Result<euro_user::AuthManager, AuthError> {
    auth_manager(app_handle)
        .await
        .ok_or(AuthError::StateUnavailable("user controller"))
}

/// Categorise an upstream `euro_auth::AuthError` into the IPC error
/// surface so the frontend can branch by `error.type`.
fn classify_auth_error(err: euro_auth::AuthError) -> AuthError {
    if err.is_logged_out() {
        AuthError::NotAuthenticated
    } else if err.is_transient() {
        AuthError::Backend(err.to_string())
    } else {
        AuthError::Internal(err.to_string())
    }
}

/// Classify an `anyhow::Error` returned by `AuthManager` methods that
/// haven't been ported to typed errors yet. The wrapped source is
/// downcast back to [`euro_auth::AuthError`] when possible so transient
/// vs. logged-out classification still works at the IPC boundary.
fn classify_anyhow_auth_error(err: anyhow::Error) -> AuthError {
    if let Some(auth_err) = err.downcast_ref::<euro_auth::AuthError>() {
        if auth_err.is_logged_out() {
            return AuthError::NotAuthenticated;
        }
        if auth_err.is_transient() {
            return AuthError::Backend(err.to_string());
        }
    }
    AuthError::Internal(err.to_string())
}

async fn save_app_settings(app_handle: &AppHandle) -> Result<(), AuthError> {
    let state = app_handle.state::<SharedSettingsState>();
    let settings = state.lock().await;
    settings
        .save_local_to_default_path()
        .map_err(|e| AuthError::Persistence(e.to_string()))?;
    settings
        .save_cache_to_default_path()
        .map_err(|e| AuthError::Persistence(e.to_string()))
}

#[tauri::command]
#[specta::specta]
pub async fn auth_get_login_token(app_handle: AppHandle) -> Result<LoginToken, AuthError> {
    // Honour whatever connection mode the user picked. Earlier builds
    // forcibly switched the endpoint back to the cloud here, which made
    // self-hosted login impossible from a release build; that override
    // has been removed in favour of the explicit Cloud / Local / Custom
    // picker in `APISettings::mode`.
    let auth_manager = resolve_auth_manager(&app_handle).await?;
    let (code_verifier, code_challenge) = auth_manager
        .get_login_tokens()
        .await
        .map_err(classify_anyhow_auth_error)?;
    let expires_in: u32 = 60 * 20;

    // `WEB_URL` is baked into the binary by `build.rs` and injected into
    // the process env by `euro_tauri::load_env`, so a runtime miss here
    // means someone called this entry point before `main` ran.
    let base_url = std::env::var("WEB_URL")
        .map_err(|_| AuthError::Config("WEB_URL not in process env".to_string()))?;
    let mut url = Url::parse(&format!("{base_url}/login"))
        .map_err(|e| AuthError::Config(format!("Invalid WEB_URL: {e}")))?;
    url.query_pairs_mut()
        .append_pair("code_challenge", &code_challenge)
        .append_pair("code_challenge_method", "S256");
    secret::persist(LOGIN_CODE_VERIFIER, &SecretString::from(code_verifier))
        .map_err(|e| AuthError::Persistence(format!("Failed to persist code verifier: {e}")))?;
    Ok(LoginToken {
        code_challenge: code_challenge.to_string(),
        expires_in,
        url: url.to_string(),
    })
}

#[tauri::command]
#[specta::specta]
pub async fn auth_poll_for_login(app_handle: AppHandle) -> Result<bool, AuthError> {
    let auth_manager = resolve_auth_manager(&app_handle).await?;

    let login_token = secret::retrieve(LOGIN_CODE_VERIFIER)
        .map_err(|e| AuthError::Persistence(format!("Failed to retrieve login token: {e}")))?
        .ok_or(AuthError::LoginTokenExpired)?;

    match auth_manager
        .login_by_login_token(login_token.expose_secret().to_owned())
        .await
    {
        Ok(_) => {
            secret::delete(LOGIN_CODE_VERIFIER).map_err(|e| {
                AuthError::Persistence(format!("Failed to remove login token: {e}"))
            })?;

            if let Ok(claims) = auth_manager.get_access_token_payload() {
                emit_auth_state(&app_handle, Some(claims));
            }

            save_app_settings(&app_handle).await?;
            Ok(true)
        }
        Err(e) => {
            tracing::error!("Login by login token failed: {e}");
            Ok(false)
        }
    }
}

#[tauri::command]
#[specta::specta]
pub async fn auth_register(
    app_handle: AppHandle,
    email: String,
    password: String,
) -> Result<(), AuthError> {
    let auth_manager = resolve_auth_manager(&app_handle).await?;

    auth_manager
        .register(&email, &password)
        .await
        .map_err(classify_anyhow_auth_error)?;

    if let Ok(claims) = auth_manager.get_access_token_payload() {
        emit_auth_state(&app_handle, Some(claims));
    }

    save_app_settings(&app_handle).await?;
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn auth_login(
    app_handle: AppHandle,
    login: String,
    password: String,
) -> Result<(), AuthError> {
    let auth_manager = resolve_auth_manager(&app_handle).await?;

    auth_manager
        .login(&login, &password)
        .await
        .map_err(classify_anyhow_auth_error)?;

    if let Ok(claims) = auth_manager.get_access_token_payload() {
        emit_auth_state(&app_handle, Some(claims));
    }

    save_app_settings(&app_handle).await?;
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn auth_logout(app_handle: AppHandle) -> Result<(), AuthError> {
    let user_state =
        user_controller(&app_handle).ok_or(AuthError::StateUnavailable("user controller"))?;
    let controller = user_state.lock().await;

    controller
        .delete_user()
        .map_err(|e| AuthError::Internal(format!("Logout failed: {e}")))?;
    emit_auth_state(&app_handle, None);

    save_app_settings(&app_handle).await?;
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn auth_is_authenticated(app_handle: AppHandle) -> Result<bool, AuthError> {
    let auth_manager = resolve_auth_manager(&app_handle).await?;
    match auth_manager.get_or_refresh_access_token().await {
        Ok(token) => Ok(!token.expose_secret().is_empty()),
        // Definitively logged out — surface as `false` so the frontend
        // shows the login screen.
        Err(e) if e.is_logged_out() => Ok(false),
        // Transient failure (server unreachable etc.) — local credentials
        // are intact. Don't log the user out on connectivity blips; trust
        // the last-known state if we have any token stored.
        Err(e) => {
            tracing::warn!(
                "is_authenticated: transient auth error, assuming last-known state: {e}"
            );
            Ok(auth_manager.get_access_token_payload().is_ok())
        }
    }
}

#[tauri::command]
#[specta::specta]
pub async fn auth_get_access_token_payload(app_handle: AppHandle) -> Result<Claims, AuthError> {
    let auth_manager = resolve_auth_manager(&app_handle).await?;
    auth_manager
        .get_or_refresh_access_token()
        .await
        .map_err(classify_auth_error)?;
    auth_manager
        .get_access_token_payload()
        .map_err(|e| AuthError::Internal(format!("Failed to get access token payload: {e}")))
}

#[tauri::command]
#[specta::specta]
pub async fn auth_refresh_session(app_handle: AppHandle) -> Result<(), AuthError> {
    let auth_manager = resolve_auth_manager(&app_handle).await?;
    auth_manager
        .refresh_tokens()
        .await
        .map_err(classify_auth_error)?;

    if let Ok(claims) = auth_manager.get_access_token_payload() {
        emit_auth_state(&app_handle, Some(claims));
    }

    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn auth_resend_verification_email(app_handle: AppHandle) -> Result<(), AuthError> {
    let auth_manager = resolve_auth_manager(&app_handle).await?;
    auth_manager
        .resend_verification_email()
        .await
        .map_err(classify_anyhow_auth_error)
}
