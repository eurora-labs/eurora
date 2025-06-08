//! Authentication procedures for the Tauri application.

use eur_auth::{AuthManager, LoginCredentials, RegisterData, UserInfo};
use std::sync::Arc;
use tauri::{AppHandle, Manager, Runtime};
use tokio::sync::Mutex;

/// Shared type for the AuthManager
pub type SharedAuthManager = Arc<Mutex<AuthManager>>;

/// Authentication API trait for TauRPC procedures
#[taurpc::procedures(path = "auth")]
pub trait AuthApi {
    async fn login(credentials: LoginCredentials) -> Result<UserInfo, String>;
    async fn register(user_data: RegisterData) -> Result<UserInfo, String>;
    async fn logout<R: Runtime>(app_handle: AppHandle<R>) -> Result<(), String>;
    async fn get_current_user<R: Runtime>(
        app_handle: AppHandle<R>,
    ) -> Result<Option<UserInfo>, String>;
    async fn is_authenticated<R: Runtime>(app_handle: AppHandle<R>) -> Result<bool, String>;
    async fn login_by_login_token<R: Runtime>(
        app_handle: AppHandle<R>,
        login_token: String,
    ) -> Result<UserInfo, String>;
}

/// Implementation of the AuthApi trait
#[derive(Clone)]
pub struct AuthApiImpl;

#[taurpc::resolvers]
impl AuthApi for AuthApiImpl {
    async fn login(self, credentials: LoginCredentials) -> Result<UserInfo, String> {
        // For now, return a mock response since auth service integration is not complete
        // TODO: Integrate with actual auth manager once auth service is running
        Err(
            "Auth service not yet available - please implement auth service integration"
                .to_string(),
        )
    }

    async fn register(self, user_data: RegisterData) -> Result<UserInfo, String> {
        // For now, return a mock response since auth service integration is not complete
        // TODO: Integrate with actual auth manager once auth service is running
        Err(
            "Auth service not yet available - please implement auth service integration"
                .to_string(),
        )
    }

    async fn logout<R: Runtime>(self, app_handle: AppHandle<R>) -> Result<(), String> {
        // Try to get auth manager from app state
        if let Some(auth_manager) = app_handle.try_state::<SharedAuthManager>() {
            let auth = auth_manager.lock().await;
            auth.logout().await.map_err(|e| e.to_string())?;
            Ok(())
        } else {
            Err("Auth manager not available".to_string())
        }
    }

    async fn get_current_user<R: Runtime>(
        self,
        app_handle: AppHandle<R>,
    ) -> Result<Option<UserInfo>, String> {
        // Try to get auth manager from app state
        if let Some(auth_manager) = app_handle.try_state::<SharedAuthManager>() {
            let auth = auth_manager.lock().await;
            auth.get_current_user().await.map_err(|e| e.to_string())
        } else {
            Err("Auth manager not available".to_string())
        }
    }

    async fn is_authenticated<R: Runtime>(self, app_handle: AppHandle<R>) -> Result<bool, String> {
        // Try to get auth manager from app state
        if let Some(auth_manager) = app_handle.try_state::<SharedAuthManager>() {
            let auth = auth_manager.lock().await;
            Ok(auth.is_authenticated().await)
        } else {
            // If auth manager is not available, assume not authenticated
            Ok(false)
        }
    }

    async fn login_by_login_token<R: Runtime>(
        self,
        app_handle: AppHandle<R>,
        login_token: String,
    ) -> Result<UserInfo, String> {
        // Try to get auth manager from app state
        if let Some(auth_manager) = app_handle.try_state::<SharedAuthManager>() {
            let auth = auth_manager.lock().await;
            auth.login_by_login_token(login_token)
                .await
                .map_err(|e| e.to_string())
        } else {
            Err("Auth manager not available".to_string())
        }
    }
}
