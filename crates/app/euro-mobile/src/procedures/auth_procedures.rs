use auth_core::Claims;
use euro_secret::ExposeSecret;
use euro_user::AuthManager;
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_appauth::{AppAuthExt, BrowserOnlyRequest};
use url::Url;

use crate::error::ResultExt;
use crate::shared_types::{SharedAppSettings, SharedUserController};

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

#[taurpc::procedures(
    path = "auth",
    export_to = "../../../apps/mobile/src/lib/bindings/bindings.ts"
)]
pub trait AuthApi {
    #[taurpc(event)]
    async fn auth_state_changed(claims: Option<Claims>);

    async fn start_login<R: Runtime>(app_handle: AppHandle<R>) -> Result<LoginOutcome, String>;

    async fn login<R: Runtime>(
        app_handle: AppHandle<R>,
        login: String,
        password: String,
    ) -> Result<(), String>;

    async fn register<R: Runtime>(
        app_handle: AppHandle<R>,
        email: String,
        password: String,
    ) -> Result<(), String>;

    async fn logout<R: Runtime>(app_handle: AppHandle<R>) -> Result<(), String>;
    async fn is_authenticated<R: Runtime>(app_handle: AppHandle<R>) -> Result<bool, String>;
    async fn get_email<R: Runtime>(app_handle: AppHandle<R>) -> Result<String, String>;
    async fn get_role<R: Runtime>(app_handle: AppHandle<R>) -> Result<String, String>;
    async fn get_display_name<R: Runtime>(
        app_handle: AppHandle<R>,
    ) -> Result<Option<String>, String>;
    async fn refresh_session<R: Runtime>(app_handle: AppHandle<R>) -> Result<(), String>;
}

fn build_auth_url(code_challenge: &str) -> Result<Url, String> {
    let base = std::env::var("AUTH_SERVICE_URL")
        .unwrap_or_else(|_| "https://www.eurora-labs.com".to_string());
    let mut url = Url::parse(&format!("{base}/login")).ctx("Invalid AUTH_SERVICE_URL")?;
    url.query_pairs_mut()
        .append_pair("code_challenge", code_challenge)
        .append_pair("code_challenge_method", "S256")
        .append_pair("redirect_uri", REDIRECT_URI);
    Ok(url)
}

fn user_controller<R: Runtime>(
    app_handle: &AppHandle<R>,
) -> Result<tauri::State<'_, SharedUserController>, String> {
    app_handle
        .try_state::<SharedUserController>()
        .ok_or_else(|| "User controller not available".to_string())
}

/// Briefly lock the shared `UserController`, clone out its `AuthManager`,
/// and return it. The clone is a cheap `Arc` bump; the lock is released
/// before the caller `.await`s, so concurrent requests don't serialize on
/// the outer mutex during network I/O.
async fn auth_manager<R: Runtime>(app_handle: &AppHandle<R>) -> Result<AuthManager, String> {
    let state = user_controller(app_handle)?;
    let controller = state.lock().await;
    Ok(controller.auth_manager.clone())
}

fn emit_auth_state<R: Runtime>(app_handle: &AppHandle<R>, claims: Option<Claims>) {
    let _ = TauRpcAuthApiEventTrigger::new(app_handle.clone()).auth_state_changed(claims);
}

async fn save_settings<R: Runtime>(app_handle: &AppHandle<R>) -> Result<(), String> {
    let state = app_handle.state::<SharedAppSettings>();
    let settings = state.lock().await;
    settings
        .save_to_default_path()
        .ctx("Failed to save settings")
}

#[derive(Clone)]
pub struct AuthApiImpl;

#[taurpc::resolvers]
impl AuthApi for AuthApiImpl {
    async fn start_login<R: Runtime>(
        self,
        app_handle: AppHandle<R>,
    ) -> Result<LoginOutcome, String> {
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
                // bespoke backend protocol uses the redirect purely as a
                // "user finished" signal — the verifier we already hold is
                // the bearer for token exchange.
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

    async fn login<R: Runtime>(
        self,
        app_handle: AppHandle<R>,
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

    async fn register<R: Runtime>(
        self,
        app_handle: AppHandle<R>,
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

    async fn logout<R: Runtime>(self, app_handle: AppHandle<R>) -> Result<(), String> {
        let user_state = user_controller(&app_handle)?;
        let controller = user_state.lock().await;

        controller.delete_user().ctx("Logout failed")?;
        emit_auth_state(&app_handle, None);

        save_settings(&app_handle).await
    }

    async fn is_authenticated<R: Runtime>(self, app_handle: AppHandle<R>) -> Result<bool, String> {
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

    async fn get_email<R: Runtime>(self, app_handle: AppHandle<R>) -> Result<String, String> {
        let auth_manager = auth_manager(&app_handle).await?;
        auth_manager
            .get_or_refresh_access_token()
            .await
            .ctx("Failed to get access token")?;
        let claims = auth_manager
            .get_access_token_payload()
            .ctx("Failed to get access token payload")?;
        Ok(claims.email)
    }

    async fn get_role<R: Runtime>(self, app_handle: AppHandle<R>) -> Result<String, String> {
        let auth_manager = auth_manager(&app_handle).await?;
        auth_manager
            .get_or_refresh_access_token()
            .await
            .ctx("Failed to get access token")?;
        let claims = auth_manager
            .get_access_token_payload()
            .ctx("Failed to get access token payload")?;
        Ok(claims.role.to_string())
    }

    async fn get_display_name<R: Runtime>(
        self,
        app_handle: AppHandle<R>,
    ) -> Result<Option<String>, String> {
        let auth_manager = auth_manager(&app_handle).await?;
        auth_manager
            .get_or_refresh_access_token()
            .await
            .ctx("Failed to get access token")?;
        let claims = auth_manager
            .get_access_token_payload()
            .ctx("Failed to get access token payload")?;
        Ok(claims.display_name)
    }

    async fn refresh_session<R: Runtime>(self, app_handle: AppHandle<R>) -> Result<(), String> {
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
}
