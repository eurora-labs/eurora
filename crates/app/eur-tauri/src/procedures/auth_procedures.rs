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
}

/// Implementation of the AuthApi trait
#[derive(Clone)]
pub struct AuthApiImpl;

#[taurpc::resolvers]
impl AuthApi for AuthApiImpl {
    async fn login(self, credentials: LoginCredentials) -> Result<UserInfo, String> {
        // For now, return a mock response until we integrate with the auth manager
        // This will be updated when we integrate with main.rs
        Err("Auth manager not yet integrated".to_string())
    }

    async fn register(self, user_data: RegisterData) -> Result<UserInfo, String> {
        // For now, return a mock response until we integrate with the auth manager
        Err("Auth manager not yet integrated".to_string())
    }

    async fn logout<R: Runtime>(self, app_handle: AppHandle<R>) -> Result<(), String> {
        // For now, return a mock response until we integrate with the auth manager
        Err("Auth manager not yet integrated".to_string())
    }

    async fn get_current_user<R: Runtime>(
        self,
        app_handle: AppHandle<R>,
    ) -> Result<Option<UserInfo>, String> {
        // For now, return a mock response until we integrate with the auth manager
        Err("Auth manager not yet integrated".to_string())
    }

    async fn is_authenticated<R: Runtime>(self, app_handle: AppHandle<R>) -> Result<bool, String> {
        // For now, return a mock response until we integrate with the auth manager
        Ok(false)
    }
}
