//! Authentication provider for other procedures to get valid auth tokens.

use anyhow::{Result, anyhow};
use eur_auth::AuthManager;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Authentication provider that other procedures use to get valid tokens
pub struct AuthProvider {
    auth_manager: Arc<Mutex<AuthManager>>,
}

impl AuthProvider {
    /// Create a new AuthProvider
    pub fn new(auth_manager: Arc<Mutex<AuthManager>>) -> Self {
        Self { auth_manager }
    }

    /// Get a valid authorization header for API calls
    /// This is the key method other procedures use
    pub async fn get_auth_header(&self) -> Result<String> {
        let auth = self.auth_manager.lock().await;
        let token = auth.get_valid_token().await?;
        Ok(format!("Bearer {}", token))
    }

    /// Ensure user is authenticated, return error if not
    pub async fn ensure_authenticated(&self) -> Result<()> {
        let auth = self.auth_manager.lock().await;
        if !auth.is_authenticated().await {
            return Err(anyhow!("User not authenticated"));
        }
        Ok(())
    }

    /// Get current user information
    pub async fn get_current_user(&self) -> Result<Option<eur_auth::UserInfo>> {
        let auth = self.auth_manager.lock().await;
        auth.get_current_user().await
    }

    /// Check if user is authenticated
    pub async fn is_authenticated(&self) -> bool {
        let auth = self.auth_manager.lock().await;
        auth.is_authenticated().await
    }
}

impl Clone for AuthProvider {
    fn clone(&self) -> Self {
        Self {
            auth_manager: self.auth_manager.clone(),
        }
    }
}
