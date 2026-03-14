use auth_core::Claims;
use euro_endpoint::DEFAULT_API_URL;
use euro_secret::{ExposeSecret, SecretString, secret};
use tauri::{AppHandle, Manager, Runtime};
use url::Url;

use crate::error::ResultExt;
use crate::shared_types::{SharedAppSettings, SharedEndpointManager, SharedUserController};

#[taurpc::ipc_type]
pub struct LoginToken {
    pub code_challenge: String,
    pub expires_in: i64,
    pub url: String,
}

#[taurpc::procedures(path = "auth")]
pub trait AuthApi {
    #[taurpc(event)]
    async fn auth_state_changed(claims: Option<Claims>);

    async fn poll_for_login<R: Runtime>(app_handle: AppHandle<R>) -> Result<bool, String>;
    async fn get_login_token<R: Runtime>(app_handle: AppHandle<R>) -> Result<LoginToken, String>;

    async fn register<R: Runtime>(
        app_handle: AppHandle<R>,
        username: String,
        email: String,
        password: String,
    ) -> Result<(), String>;

    async fn login<R: Runtime>(
        app_handle: AppHandle<R>,
        login: String,
        password: String,
    ) -> Result<(), String>;

    async fn logout<R: Runtime>(app_handle: AppHandle<R>) -> Result<(), String>;
    async fn is_authenticated<R: Runtime>(app_handle: AppHandle<R>) -> Result<bool, String>;
    async fn get_role<R: Runtime>(app_handle: AppHandle<R>) -> Result<String, String>;
    async fn get_username<R: Runtime>(app_handle: AppHandle<R>) -> Result<String, String>;
    async fn get_email<R: Runtime>(app_handle: AppHandle<R>) -> Result<String, String>;
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
        let user_state = user_controller(&app_handle)?;

        if !cfg!(debug_assertions) {
            {
                let settings_state = app_handle.state::<SharedAppSettings>();
                let mut settings = settings_state.lock().await;
                settings.api.endpoint = DEFAULT_API_URL.to_string();
                settings
                    .save_to_default_path()
                    .ctx("Failed to save settings")?;
            }

            let endpoint_manager = app_handle.state::<SharedEndpointManager>();
            endpoint_manager
                .set_global_backend_url(DEFAULT_API_URL)
                .ctx("Failed to switch to cloud endpoint")?;
        }

        let mut controller = user_state.lock().await;
        let (code_verifier, code_challenge) = controller
            .get_login_tokens()
            .await
            .ctx("Failed to get login tokens")?;
        let expires_in: i64 = 60 * 20;

        let base_url = std::env::var("AUTH_SERVICE_URL")
            .unwrap_or_else(|_| "https://www.eurora-labs.com".to_string());
        let mut url = Url::parse(&format!("{base_url}/login")).ctx("Invalid AUTH_SERVICE_URL")?;
        url.query_pairs_mut()
            .append_pair("code_challenge", &code_challenge)
            .append_pair("code_challenge_method", "S256");
        secret::persist(LOGIN_CODE_VERIFIER, &SecretString::from(code_verifier))
            .ctx("Failed to persist code verifier")?;
        Ok(LoginToken {
            code_challenge: code_challenge.to_string(),
            expires_in,
            url: url.to_string(),
        })
    }

    async fn poll_for_login<R: Runtime>(self, app_handle: AppHandle<R>) -> Result<bool, String> {
        let user_state = match app_handle.try_state::<SharedUserController>() {
            Some(s) => s,
            None => return Ok(false),
        };

        let mut controller = user_state.lock().await;

        if controller.get_or_refresh_access_token().await.is_ok() {
            let _ = secret::delete(LOGIN_CODE_VERIFIER);
            if let Ok(claims) = controller.get_access_token_payload() {
                emit_auth_state(&app_handle, Some(claims));
            }
            return Ok(true);
        }

        let login_token = secret::retrieve(LOGIN_CODE_VERIFIER)
            .ctx("Failed to retrieve login token")?
            .ok_or_else(|| "Login token not found".to_string())?;

        match controller
            .login_by_login_token(login_token.expose_secret().to_owned())
            .await
        {
            Ok(_) => {
                secret::delete(LOGIN_CODE_VERIFIER).ctx("Failed to remove login token")?;

                if let Ok(claims) = controller.get_access_token_payload() {
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

    async fn register<R: Runtime>(
        self,
        app_handle: AppHandle<R>,
        username: String,
        email: String,
        password: String,
    ) -> Result<(), String> {
        let user_state = user_controller(&app_handle)?;
        let mut controller = user_state.lock().await;

        controller
            .register(&username, &email, &password)
            .await
            .ctx("Registration failed")?;

        if let Ok(claims) = controller.get_access_token_payload() {
            emit_auth_state(&app_handle, Some(claims));
        }

        let state = app_handle.state::<SharedAppSettings>();
        let settings = state.lock().await;
        settings
            .save_to_default_path()
            .ctx("Failed to save settings")?;

        Ok(())
    }

    async fn login<R: Runtime>(
        self,
        app_handle: AppHandle<R>,
        login: String,
        password: String,
    ) -> Result<(), String> {
        let user_state = user_controller(&app_handle)?;
        let mut controller = user_state.lock().await;

        controller
            .login(&login, &password)
            .await
            .ctx("Login failed")?;

        if let Ok(claims) = controller.get_access_token_payload() {
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
        let mut controller = user_state.lock().await;

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

        let Some(user_state) = result.ok() else {
            return Ok(false);
        };

        let mut controller = user_state.lock().await;
        match controller.get_or_refresh_access_token().await {
            Ok(token) => Ok(!token.expose_secret().is_empty()),
            Err(e) => Err(format!("Failed to get or refresh access token: {e}")),
        }
    }

    async fn get_role<R: Runtime>(self, app_handle: AppHandle<R>) -> Result<String, String> {
        let user_state = user_controller(&app_handle)?;
        let mut controller = user_state.lock().await;
        controller
            .get_or_refresh_access_token()
            .await
            .ctx("Failed to get access token")?;
        let claims = controller
            .get_access_token_payload()
            .ctx("Failed to get access token payload")?;
        Ok(claims.role.to_string())
    }

    async fn get_username<R: Runtime>(self, app_handle: AppHandle<R>) -> Result<String, String> {
        let user_state = user_controller(&app_handle)?;
        let mut controller = user_state.lock().await;
        controller
            .get_or_refresh_access_token()
            .await
            .ctx("Failed to get access token")?;
        let claims = controller
            .get_access_token_payload()
            .ctx("Failed to get access token payload")?;
        Ok(claims.username)
    }

    async fn refresh_session<R: Runtime>(self, app_handle: AppHandle<R>) -> Result<(), String> {
        let user_state = user_controller(&app_handle)?;
        let mut controller = user_state.lock().await;
        controller
            .refresh_tokens()
            .await
            .ctx("Failed to refresh session")?;

        if let Ok(claims) = controller.get_access_token_payload() {
            emit_auth_state(&app_handle, Some(claims));
        }

        Ok(())
    }

    async fn get_email<R: Runtime>(self, app_handle: AppHandle<R>) -> Result<String, String> {
        let user_state = user_controller(&app_handle)?;
        let mut controller = user_state.lock().await;
        controller
            .get_or_refresh_access_token()
            .await
            .ctx("Failed to get access token")?;
        let claims = controller
            .get_access_token_payload()
            .ctx("Failed to get access token payload")?;
        Ok(claims.email)
    }
}
