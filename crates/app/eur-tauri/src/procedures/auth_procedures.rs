//! Authentication procedures for the Tauri application.

use crate::auth::AuthManager;
use std::sync::Arc;
use tauri::{AppHandle, Manager, Runtime};
use tokio::sync::Mutex;

#[taurpc::ipc_type]
pub struct LoginToken {
    pub token: String,
    pub expires_in: i64,
    pub url: String,
}

/// Authentication API trait for TauRPC procedures
#[taurpc::procedures(path = "auth")]
pub trait AuthApi {
    async fn poll_for_login<R: Runtime>(
        app_handle: AppHandle<R>,
        login_token: String,
    ) -> Result<bool, String>;
    async fn get_login_token<R: Runtime>(app_handle: AppHandle<R>) -> Result<LoginToken, String>;
}

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
            auth_manager
                .get_login_token()
                .await
                .map_err(|e| e.to_string())
                .map(|token| LoginToken {
                    token: token.token,
                    expires_in: token.expires_in,
                    url: token.url,
                })
        } else {
            Err("Auth manager not available".to_string())
        }
    }

    async fn poll_for_login<R: Runtime>(
        self,
        app_handle: AppHandle<R>,
        login_token: String,
    ) -> Result<bool, String> {
        if let Some(auth_manager) = app_handle.try_state::<AuthManager>() {
            match auth_manager.login_by_login_token(login_token).await {
                Ok(_) => Ok(true),
                Err(_) => Ok(false),
            }
        } else {
            Ok(false)
        }
    }
}
