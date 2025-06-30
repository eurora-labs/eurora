//! Authentication procedures for the Tauri application.

use crate::{
    procedures::prompt_procedures::TauRpcPromptApiEventTrigger,
    shared_types::SharedPromptKitService,
};
use eur_prompt_kit::EuroraConfig;
use eur_secret::{Sensitive, secret};
use eur_user::auth::AuthManager;
use tauri::{AppHandle, Manager, Runtime};
use url::Url;

#[taurpc::ipc_type]
pub struct LoginToken {
    pub code_challenge: String,
    pub expires_in: i64,
    pub url: String,
}

/// Authentication API trait for TauRPC procedures
#[taurpc::procedures(path = "auth")]
pub trait AuthApi {
    async fn poll_for_login<R: Runtime>(app_handle: AppHandle<R>) -> Result<bool, String>;
    async fn get_login_token<R: Runtime>(app_handle: AppHandle<R>) -> Result<LoginToken, String>;
}

const LOGIN_CODE_VERIFIER: &str = "LOGIN_CODE_VERIFIER";
/// Implementation of the AuthApi trait
#[derive(Clone)]
pub struct AuthApiImpl;

#[taurpc::resolvers]
impl AuthApi for AuthApiImpl {
    async fn get_login_token<R: Runtime>(
        self,
        app_handle: AppHandle<R>,
    ) -> Result<LoginToken, String> {
        // Try to get auth manager from app state
        if let Some(auth_manager) = app_handle.try_state::<AuthManager>() {
            let (code_verifier, code_challenge) = auth_manager
                .get_login_tokens()
                .await
                .map_err(|e| format!("Failed to get login tokens: {}", e))?;
            let expires_in: i64 = 60 * 20;

            let base_url = std::env::var("AUTH_SERVICE_URL")
                .unwrap_or_else(|_| "http://localhost:5173".to_string());
            let mut url = Url::parse(&format!("{}/login", base_url))
                .map_err(|e| format!("Invalid AUTH_SERVICE_URL: {}", e))?;
            // Add code challenge as parameter
            url.query_pairs_mut()
                .append_pair("code_challenge", &code_challenge)
                .append_pair("code_challenge_method", "S256");
            secret::persist(
                LOGIN_CODE_VERIFIER,
                &Sensitive(code_verifier.clone()),
                secret::Namespace::BuildKind,
            )
            .map_err(|e| format!("Failed to persist code verifier: {}", e))?;
            Ok(LoginToken {
                code_challenge,
                expires_in,
                url: url.to_string(),
            })
        } else {
            Err("Auth manager not available".to_string())
        }
    }

    async fn poll_for_login<R: Runtime>(self, app_handle: AppHandle<R>) -> Result<bool, String> {
        if let Some(auth_manager) = app_handle.try_state::<AuthManager>() {
            let login_token = secret::retrieve(LOGIN_CODE_VERIFIER, secret::Namespace::BuildKind)
                .map_err(|e| format!("Failed to retrieve login token: {}", e))?
                .ok_or_else(|| "Login token not found".to_string())?;
            match auth_manager.login_by_login_token(login_token.0).await {
                Ok(_) => {
                    secret::delete(LOGIN_CODE_VERIFIER, secret::Namespace::BuildKind)
                        .map_err(|e| format!("Failed to remove login token: {}", e))?;

                    let mut promptkit_client = eur_prompt_kit::PromptKitService::default();
                    promptkit_client
                        .switch_to_eurora(EuroraConfig {
                            model: "default".to_string(),
                        })
                        .await?;

                    TauRpcPromptApiEventTrigger::new(app_handle.clone())
                        .prompt_service_change(Some(
                            promptkit_client
                                .get_service_name()
                                .map_err(|e| e.to_string())?,
                        ))
                        .map_err(|e| e.to_string())?;

                    let state: tauri::State<SharedPromptKitService> = app_handle.state();
                    let mut guard = state.lock().await;
                    *guard = Some(promptkit_client);

                    Ok(true)
                }
                Err(_) => Ok(false),
            }
        } else {
            Ok(false)
        }
    }
}
