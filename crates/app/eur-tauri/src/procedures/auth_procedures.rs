//! Authentication procedures for the Tauri application.

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
            let (code_verifier, code_challenge) = auth_manager.get_login_tokens().await.unwrap();
            let expires_in: i64 = 60 * 20;

            let mut url = Url::parse("http://localhost:5173/login").unwrap();
            // Add code challenge as parameter
            url.query_pairs_mut()
                .append_pair("code_challenge", &code_challenge)
                .append_pair("code_challenge_method", "S256");
            secret::persist(
                LOGIN_CODE_VERIFIER,
                &Sensitive(code_verifier.clone()),
                secret::Namespace::BuildKind,
            )
            .unwrap();
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
                .unwrap()
                .unwrap();
            match auth_manager
                .login_by_login_token(login_token.to_string())
                .await
            {
                Ok(_) => Ok(true),
                Err(_) => Ok(false),
            }
        } else {
            Ok(false)
        }
    }
}
