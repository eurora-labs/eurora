use euro_secret::{Sensitive, secret};
use tauri::{AppHandle, Manager, Runtime};
use tracing::error;
use url::Url;

use crate::shared_types::{SharedAppSettings, SharedUserController};

#[taurpc::ipc_type]
pub struct LoginToken {
    pub code_challenge: String,
    pub expires_in: i64,
    pub url: String,
}

#[taurpc::procedures(path = "auth")]
pub trait AuthApi {
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
}

const LOGIN_CODE_VERIFIER: &str = "LOGIN_CODE_VERIFIER";

#[derive(Clone)]
pub struct AuthApiImpl;

#[taurpc::resolvers]
impl AuthApi for AuthApiImpl {
    async fn get_login_token<R: Runtime>(
        self,
        app_handle: AppHandle<R>,
    ) -> Result<LoginToken, String> {
        if let Some(user_state) = app_handle.try_state::<SharedUserController>() {
            let mut controller = user_state.lock().await;
            let (code_verifier, code_challenge) = controller
                .get_login_tokens()
                .await
                .map_err(|e| format!("Failed to get login tokens: {}", e))?;
            let expires_in: i64 = 60 * 20;

            let base_url = std::env::var("AUTH_SERVICE_URL")
                .unwrap_or("https://www.eurora-labs.com".to_string());
            let mut url = Url::parse(&format!("{}/login", base_url))
                .map_err(|e| format!("Invalid AUTH_SERVICE_URL: {}", e))?;
            url.query_pairs_mut()
                .append_pair("code_challenge", &code_challenge)
                .append_pair("code_challenge_method", "S256");
            secret::persist(
                LOGIN_CODE_VERIFIER,
                &Sensitive(code_verifier.clone()),
                secret::Namespace::Global,
            )
            .map_err(|e| format!("Failed to persist code verifier: {}", e))?;
            Ok(LoginToken {
                code_challenge: code_challenge.to_string(),
                expires_in,
                url: url.to_string(),
            })
        } else {
            Err("Auth manager not available".to_string())
        }
    }

    async fn poll_for_login<R: Runtime>(self, app_handle: AppHandle<R>) -> Result<bool, String> {
        if let Some(user_state) = app_handle.try_state::<SharedUserController>() {
            let mut controller = user_state.lock().await;
            let login_token = secret::retrieve(LOGIN_CODE_VERIFIER, secret::Namespace::Global)
                .map_err(|e| format!("Failed to retrieve login token: {}", e))?
                .ok_or_else(|| "Login token not found".to_string())?;

            match controller.login_by_login_token(login_token.0).await {
                Ok(_) => {
                    secret::delete(LOGIN_CODE_VERIFIER, secret::Namespace::Global)
                        .map_err(|e| format!("Failed to remove login token: {}", e))?;

                    let state = app_handle.state::<SharedAppSettings>();
                    let settings = state.lock().await;

                    settings
                        .save_to_default_path()
                        .map_err(|e| format!("Failed to save settings: {}", e))?;

                    Ok(true)
                }
                Err(e) => {
                    error!("Login by login token failed: {}", e);

                    Ok(false)
                }
            }
        } else {
            error!("Failed to initialize prompt kit service: Invalid configuration");

            Ok(false)
        }
    }

    async fn register<R: Runtime>(
        self,
        app_handle: AppHandle<R>,
        username: String,
        email: String,
        password: String,
    ) -> Result<(), String> {
        let user_state = app_handle
            .try_state::<SharedUserController>()
            .ok_or_else(|| "User controller not available".to_string())?;
        let mut controller = user_state.lock().await;

        controller
            .register(&username, &email, &password)
            .await
            .map_err(|e| format!("Registration failed: {}", e))?;

        let state = app_handle.state::<SharedAppSettings>();
        let settings = state.lock().await;
        settings
            .save_to_default_path()
            .map_err(|e| format!("Failed to save settings: {}", e))?;

        Ok(())
    }

    async fn login<R: Runtime>(
        self,
        app_handle: AppHandle<R>,
        login: String,
        password: String,
    ) -> Result<(), String> {
        let user_state = app_handle
            .try_state::<SharedUserController>()
            .ok_or_else(|| "User controller not available".to_string())?;
        let mut controller = user_state.lock().await;

        controller
            .login(&login, &password)
            .await
            .map_err(|e| format!("Login failed: {}", e))?;

        let state = app_handle.state::<SharedAppSettings>();
        let settings = state.lock().await;
        settings
            .save_to_default_path()
            .map_err(|e| format!("Failed to save settings: {}", e))?;

        Ok(())
    }

    async fn logout<R: Runtime>(self, app_handle: AppHandle<R>) -> Result<(), String> {
        let user_state = app_handle
            .try_state::<SharedUserController>()
            .ok_or_else(|| "User controller not available".to_string())?;
        let mut controller = user_state.lock().await;

        controller
            .delete_user()
            .map_err(|e| format!("Logout failed: {}", e))?;

        let state = app_handle.state::<SharedAppSettings>();
        let settings = state.lock().await;
        settings
            .save_to_default_path()
            .map_err(|e| format!("Failed to save settings: {}", e))?;

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
            Ok(token) => Ok(!token.is_empty()),
            Err(e) => Err(format!("Failed to get or refresh access token: {}", e)),
        }
    }

    async fn get_role<R: Runtime>(self, app_handle: AppHandle<R>) -> Result<String, String> {
        if let Some(user_state) = app_handle.try_state::<SharedUserController>() {
            let mut controller = user_state.lock().await;
            controller
                .get_or_refresh_access_token()
                .await
                .map_err(|e| format!("Failed to get access token: {}", e))?;
            let claims = controller
                .get_access_token_payload()
                .map_err(|e| format!("Failed to get access token payload: {}", e))?;
            Ok(claims.role.to_string())
        } else {
            Err("User controller not available".to_string())
        }
    }
}
