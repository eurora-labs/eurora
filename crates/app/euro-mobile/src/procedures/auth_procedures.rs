use auth_core::Claims;
use euro_secret::{ExposeSecret, SecretString, secret};
use euro_user::AuthManager;
use tauri::{AppHandle, Manager, Runtime};
use url::Url;

use crate::error::ResultExt;
use crate::shared_types::{SharedAppSettings, SharedUserController};

#[taurpc::ipc_type]
pub struct LoginToken {
    pub code_challenge: String,
    pub expires_in: i64,
    pub url: String,
}

#[taurpc::procedures(
    path = "auth",
    export_to = "../../../apps/mobile/src/lib/bindings/bindings.ts"
)]
pub trait AuthApi {
    #[taurpc(event)]
    async fn auth_state_changed(claims: Option<Claims>);

    async fn get_login_token<R: Runtime>(app_handle: AppHandle<R>) -> Result<LoginToken, String>;
    async fn poll_for_login<R: Runtime>(app_handle: AppHandle<R>) -> Result<bool, String>;

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

const LOGIN_CODE_VERIFIER: &str = "LOGIN_CODE_VERIFIER";

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

#[derive(Clone)]
pub struct AuthApiImpl;

#[taurpc::resolvers]
impl AuthApi for AuthApiImpl {
    async fn get_login_token<R: Runtime>(
        self,
        app_handle: AppHandle<R>,
    ) -> Result<LoginToken, String> {
        let auth_manager = auth_manager(&app_handle).await?;

        let (code_verifier, code_challenge) = auth_manager
            .get_login_tokens()
            .await
            .ctx("Failed to get login tokens")?;
        let expires_in: i64 = 60 * 20;

        let base_url = std::env::var("AUTH_SERVICE_URL")
            .unwrap_or_else(|_| "https://www.eurora-labs.com".to_string());
        let mut url = Url::parse(&format!("{base_url}/login")).ctx("Invalid AUTH_SERVICE_URL")?;
        url.query_pairs_mut()
            .append_pair("code_challenge", &code_challenge)
            .append_pair("code_challenge_method", "S256")
            .append_pair("redirect_uri", &format!("{base_url}/mobile/callback"));

        secret::persist(LOGIN_CODE_VERIFIER, &SecretString::from(code_verifier))
            .ctx("Failed to persist code verifier")?;

        Ok(LoginToken {
            code_challenge: code_challenge.to_string(),
            expires_in,
            url: url.to_string(),
        })
    }

    async fn poll_for_login<R: Runtime>(self, app_handle: AppHandle<R>) -> Result<bool, String> {
        if app_handle.try_state::<SharedUserController>().is_none() {
            return Ok(false);
        };

        let auth_manager = auth_manager(&app_handle).await?;

        let login_token = secret::retrieve(LOGIN_CODE_VERIFIER)
            .ctx("Failed to retrieve login token")?
            .ok_or_else(|| "Login token not found".to_string())?;

        match auth_manager
            .login_by_login_token(login_token.expose_secret().to_owned())
            .await
        {
            Ok(_) => {
                secret::delete(LOGIN_CODE_VERIFIER).ctx("Failed to remove login token")?;

                if let Ok(claims) = auth_manager.get_access_token_payload() {
                    emit_auth_state(&app_handle, Some(claims));
                }

                let state = app_handle.state::<SharedAppSettings>();
                let settings = state.lock().await;
                settings
                    .save_to_default_path()
                    .ctx("Failed to save settings")?;

                Ok(true)
            }
            Err(e) => {
                tracing::error!("Login by login token failed: {e}");
                Ok(false)
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

        let state = app_handle.state::<SharedAppSettings>();
        let settings = state.lock().await;
        settings
            .save_to_default_path()
            .ctx("Failed to save settings")?;

        Ok(())
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

        let state = app_handle.state::<SharedAppSettings>();
        let settings = state.lock().await;
        settings
            .save_to_default_path()
            .ctx("Failed to save settings")?;

        Ok(())
    }

    async fn logout<R: Runtime>(self, app_handle: AppHandle<R>) -> Result<(), String> {
        let user_state = user_controller(&app_handle)?;
        let controller = user_state.lock().await;

        controller.delete_user().ctx("Logout failed")?;
        emit_auth_state(&app_handle, None);

        let state = app_handle.state::<SharedAppSettings>();
        let settings = state.lock().await;
        settings
            .save_to_default_path()
            .ctx("Failed to save settings")?;

        Ok(())
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
