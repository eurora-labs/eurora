use auth_core::Claims;
use euro_secret::ExposeSecret;
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, Manager};
use tauri_plugin_appauth::{AppAuthExt, BrowserOnlyRequest};
use tauri_specta::Event;
use url::Url;

use crate::error::ResultExt;
use crate::procedures::{auth_manager, user_controller};
use crate::shared_types::SharedAppSettings;

/// Custom URL scheme registered with iOS (`Info.plist` `CFBundleURLSchemes`)
/// and Android (`tauriBrowserRedirectScheme` manifest placeholder). The web
/// login page redirects here once the user finishes; `tauri-plugin-appauth`
/// captures the redirect through `ASWebAuthenticationSession` /
/// `BrowserSessionActivity` and resolves the awaited future.
const REDIRECT_URI: &str = "eurora://mobile/callback";

#[derive(Clone, Debug, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum LoginOutcome {
    Success,
    Canceled,
    Rejected,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct AuthStateChanged {
    pub claims: Option<Claims>,
}

fn emit_auth_state(app_handle: &AppHandle, claims: Option<Claims>) {
    if let Err(e) = (AuthStateChanged { claims }).emit(app_handle) {
        tracing::warn!("Failed to emit auth state event: {e}");
    }
}

fn build_auth_url(code_challenge: &str) -> Result<Url, String> {
    // `WEB_URL` is baked at build time by `build.rs` and injected into the
    // process env by `load_env` at startup, so a miss here means this entry
    // point was reached before `run` ran.
    let base = std::env::var("WEB_URL").ctx("WEB_URL not in process env")?;
    let mut url = Url::parse(&format!("{base}/login")).ctx("Invalid WEB_URL")?;
    url.query_pairs_mut()
        .append_pair("code_challenge", code_challenge)
        .append_pair("code_challenge_method", "S256")
        .append_pair("redirect_uri", REDIRECT_URI);
    Ok(url)
}

async fn save_settings(app_handle: &AppHandle) -> Result<(), String> {
    let state = app_handle.state::<SharedAppSettings>();
    let settings = state.lock().await;
    settings
        .save_to_default_path()
        .ctx("Failed to save settings")
}

#[tauri::command]
#[specta::specta]
pub async fn auth_start_login(app_handle: AppHandle) -> Result<LoginOutcome, String> {
    let auth_manager = auth_manager(&app_handle).await?;

    let (code_verifier, code_challenge) = auth_manager
        .get_login_tokens()
        .await
        .ctx("Failed to get login tokens")?;

    let auth_url = build_auth_url(&code_challenge)?;

    // Run the in-app browser session via appauth. The verifier lives only
    // in this awaiting frame — never touches disk — and is consumed by the
    // backend exchange below. If the app is killed mid-flow the OS tears
    // down the browser session anyway, so persistence wouldn't recover us.
    let session = app_handle
        .appauth()
        .authorize_browser_only(BrowserOnlyRequest {
            auth_url: auth_url.to_string(),
            redirect_uri: REDIRECT_URI.to_string(),
            prefers_ephemeral_session: true,
        })
        .await;

    match session {
        Ok(_) => {
            // The captured callback URL carries no payload we need: the
            // bespoke backend protocol uses the redirect purely as a "user
            // finished" signal — the verifier we already hold is the bearer
            // for token exchange.
            match auth_manager.login_by_login_token(code_verifier).await {
                Ok(_) => {
                    if let Ok(claims) = auth_manager.get_access_token_payload() {
                        emit_auth_state(&app_handle, Some(claims));
                    }
                    save_settings(&app_handle).await?;
                    Ok(LoginOutcome::Success)
                }
                Err(e) => {
                    tracing::error!("Login by login token failed: {e}");
                    Ok(LoginOutcome::Rejected)
                }
            }
        }
        Err(err) => {
            let code = err.code();
            if code == "USER_CANCELED" {
                return Ok(LoginOutcome::Canceled);
            }
            Err(format!("[{code}] {err}"))
        }
    }
}

#[tauri::command]
#[specta::specta]
pub async fn auth_login(
    app_handle: AppHandle,
    login: String,
    password: String,
) -> Result<(), String> {
    let auth_manager = auth_manager(&app_handle).await?;

    auth_manager
        .login(&login, &password)
        .await
        .ctx("Login failed")?;

    if let Ok(claims) = auth_manager.get_access_token_payload() {
        emit_auth_state(&app_handle, Some(claims));
    }

    save_settings(&app_handle).await
}

#[tauri::command]
#[specta::specta]
pub async fn auth_register(
    app_handle: AppHandle,
    email: String,
    password: String,
) -> Result<(), String> {
    let auth_manager = auth_manager(&app_handle).await?;

    auth_manager
        .register(&email, &password)
        .await
        .ctx("Registration failed")?;

    if let Ok(claims) = auth_manager.get_access_token_payload() {
        emit_auth_state(&app_handle, Some(claims));
    }

    save_settings(&app_handle).await
}

#[tauri::command]
#[specta::specta]
pub async fn auth_logout(app_handle: AppHandle) -> Result<(), String> {
    let user_state = user_controller(&app_handle)?;
    let controller = user_state.lock().await;

    controller.delete_user().ctx("Logout failed")?;
    emit_auth_state(&app_handle, None);

    save_settings(&app_handle).await
}

#[tauri::command]
#[specta::specta]
pub async fn auth_is_authenticated(app_handle: AppHandle) -> Result<bool, String> {
    use crate::shared_types::SharedUserController;
    use backon::{ConstantBuilder, Retryable};

    let result = (|| async {
        app_handle
            .try_state::<SharedUserController>()
            .ok_or("User state not initialized")
    })
    .retry(
        ConstantBuilder::default()
            .with_delay(std::time::Duration::from_millis(100))
            .with_max_times(50),
    )
    .sleep(tokio::time::sleep)
    .await;

    if result.is_err() {
        return Ok(false);
    }

    let auth_manager = auth_manager(&app_handle).await?;
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
pub async fn auth_get_access_token_payload(app_handle: AppHandle) -> Result<Claims, String> {
    let auth_manager = auth_manager(&app_handle).await?;
    auth_manager
        .get_or_refresh_access_token()
        .await
        .ctx("Failed to get access token")?;
    auth_manager
        .get_access_token_payload()
        .ctx("Failed to get access token payload")
}

#[tauri::command]
#[specta::specta]
pub async fn auth_refresh_session(app_handle: AppHandle) -> Result<(), String> {
    let auth_manager = auth_manager(&app_handle).await?;
    auth_manager
        .refresh_tokens()
        .await
        .ctx("Failed to refresh session")?;

    if let Ok(claims) = auth_manager.get_access_token_payload() {
        emit_auth_state(&app_handle, Some(claims));
    }

    Ok(())
}
