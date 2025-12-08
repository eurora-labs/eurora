//! Core authentication manager that handles all token operations autonomously.

use std::sync::Arc;

use anyhow::{Result, anyhow};
use chrono::{DateTime, Duration, Utc};
use euro_proto_client::auth::AuthClient;
use tokio::sync::RwLock;
use tracing::{debug, warn};

use crate::{
    JwtConfig, token_storage::TokenStorage, validate_access_token, validate_refresh_token,
};

/// User information returned after successful authentication
#[derive(Debug)]
#[taurpc::ipc_type]
pub struct UserInfo {
    pub id: String,
    pub username: String,
    pub email: String,
    pub display_name: Option<String>,
}

/// Login credentials for authentication
#[derive(Debug)]
#[taurpc::ipc_type]
pub struct LoginCredentials {
    pub login: String, // username or email
    pub password: String,
}

/// Registration data for new users
#[derive(Debug)]
#[taurpc::ipc_type]
pub struct RegisterData {
    pub username: String,
    pub email: String,
    pub password: String,
    pub display_name: Option<String>,
}

/// Core authentication manager that handles all token operations
pub struct AuthManager {
    token_storage: Arc<dyn TokenStorage>,
    grpc_client: Option<Arc<RwLock<Option<AuthClient>>>>,
    jwt_config: JwtConfig,
    refresh_threshold: Duration, // Refresh when token expires in X minutes
    current_user: Arc<RwLock<Option<UserInfo>>>,
}

impl AuthManager {
    /// Create a new AuthManager instance
    pub async fn new(
        token_storage: Box<dyn TokenStorage>,
        service_url: Option<String>,
    ) -> Result<Self> {
        let jwt_config = JwtConfig::default();

        // Try to connect to gRPC service if URL provided
        let grpc_client = if let Some(url) = service_url {
            match AuthClient::new(Some(url)).await {
                Ok(client) => Some(client),
                Err(e) => {
                    warn!("Failed to connect to auth service: {}", e);
                    None
                }
            }
        } else {
            None
        };

        let manager = Self {
            token_storage: Arc::from(token_storage),
            grpc_client: grpc_client.map(|client| Arc::new(RwLock::new(Some(client)))),
            jwt_config,
            refresh_threshold: Duration::minutes(5), // Refresh 5 minutes before expiry
            current_user: Arc::new(RwLock::new(None)),
        };

        // Try to restore user session from stored tokens
        if let Err(e) = manager.restore_session().await {
            debug!("No valid session to restore: {}", e);
        }

        Ok(manager)
    }

    /// Login with credentials
    pub async fn login(&self, credentials: LoginCredentials) -> Result<UserInfo> {
        let mut client_guard = self.grpc_client.as_ref().unwrap().write().await;
        let client = client_guard
            .as_mut()
            .ok_or_else(|| anyhow!("Auth service not available"))?;

        let response = client
            .login_by_password(credentials.login, credentials.password)
            .await?;

        // Store tokens securely
        self.token_storage
            .store_access_token(&response.access_token)
            .await?;
        self.token_storage
            .store_refresh_token(&response.refresh_token)
            .await?;

        // Extract user info from token
        let user_info = self.extract_user_info(&response.access_token)?;

        // Update current user
        let mut user_guard = self.current_user.write().await;
        *user_guard = Some(user_info.clone());

        debug!("User logged in successfully: {}", user_info.username);
        Ok(user_info)
    }

    /// Register a new user
    pub async fn register(&self, user_data: RegisterData) -> Result<UserInfo> {
        let mut client_guard = self.grpc_client.as_ref().unwrap().write().await;
        let client = client_guard
            .as_mut()
            .ok_or_else(|| anyhow!("Auth service not available"))?;

        let response = client
            .register(
                &user_data.username,
                &user_data.email,
                &user_data.password,
                user_data.display_name,
            )
            .await?;

        // Store tokens securely
        self.token_storage
            .store_access_token(&response.access_token)
            .await?;
        self.token_storage
            .store_refresh_token(&response.refresh_token)
            .await?;

        // Extract user info from token
        let user_info = self.extract_user_info(&response.access_token)?;

        // Update current user
        let mut user_guard = self.current_user.write().await;
        *user_guard = Some(user_info.clone());

        debug!("User registered successfully: {}", user_info.username);
        Ok(user_info)
    }

    /// Logout and clear all tokens
    pub async fn logout(&self) -> Result<()> {
        self.token_storage.clear_tokens().await?;

        let mut user_guard = self.current_user.write().await;
        *user_guard = None;

        debug!("User logged out successfully");
        Ok(())
    }

    /// Get a valid access token, refreshing if necessary
    pub async fn get_valid_token(&self) -> Result<String> {
        // Try to get current access token
        if let Some(access_token) = self.token_storage.get_access_token().await? {
            // Check if token is still valid and not close to expiry
            if let Ok(claims) = validate_access_token(&access_token, &self.jwt_config) {
                let exp_time = DateTime::from_timestamp(claims.exp as i64, 0)
                    .ok_or_else(|| anyhow!("Invalid expiration time in token"))?;
                let now = Utc::now();

                // If token expires in more than our threshold, use it
                if exp_time - now > self.refresh_threshold {
                    return Ok(access_token);
                }
            }
        }

        // Token is invalid or close to expiry, try to refresh
        self.refresh_token_if_needed().await?;

        // Get the refreshed token
        self.token_storage
            .get_access_token()
            .await?
            .ok_or_else(|| anyhow!("No valid access token available after refresh"))
    }

    /// Get current user information
    pub async fn get_current_user(&self) -> Result<Option<UserInfo>> {
        let user_guard = self.current_user.read().await;
        Ok(user_guard.clone())
    }

    /// Check if user is authenticated
    pub async fn is_authenticated(&self) -> bool {
        // Check if we have a valid access token
        self.get_valid_token().await.is_ok()
    }

    /// Refresh access token using refresh token
    async fn refresh_token_if_needed(&self) -> Result<()> {
        let refresh_token = self
            .token_storage
            .get_refresh_token()
            .await?
            .ok_or_else(|| anyhow!("No refresh token available"))?;

        // Validate refresh token
        validate_refresh_token(&refresh_token, &self.jwt_config)?;

        let mut client_guard = self.grpc_client.as_ref().unwrap().write().await;
        let client = client_guard
            .as_mut()
            .ok_or_else(|| anyhow!("Auth service not available"))?;

        let response = client.refresh_token(&refresh_token).await?;

        // Store new tokens
        self.token_storage
            .store_access_token(&response.access_token)
            .await?;
        self.token_storage
            .store_refresh_token(&response.refresh_token)
            .await?;

        // Update user info from new token
        let user_info = self.extract_user_info(&response.access_token)?;
        let mut user_guard = self.current_user.write().await;
        *user_guard = Some(user_info);

        debug!("Access token refreshed successfully");
        Ok(())
    }

    /// Try to restore session from stored tokens
    async fn restore_session(&self) -> Result<()> {
        if let Some(access_token) = self.token_storage.get_access_token().await?
            && let Ok(claims) = validate_access_token(&access_token, &self.jwt_config)
        {
            let user_info = UserInfo {
                id: claims.sub,
                username: claims.username,
                email: claims.email,
                display_name: None, // Not stored in JWT
            };

            let mut user_guard = self.current_user.write().await;
            *user_guard = Some(user_info.clone());

            debug!("Session restored for user: {}", user_info.username);
            return Ok(());
        }

        Err(anyhow!("No valid session to restore"))
    }

    /// Extract user information from JWT token
    fn extract_user_info(&self, access_token: &str) -> Result<UserInfo> {
        let claims = validate_access_token(access_token, &self.jwt_config)?;

        Ok(UserInfo {
            id: claims.sub,
            username: claims.username,
            email: claims.email,
            display_name: None, // Not stored in JWT
        })
    }

    pub async fn login_by_login_token(&self, login_token: String) -> Result<UserInfo> {
        let mut client_guard = self.grpc_client.as_ref().unwrap().write().await;
        let client = client_guard
            .as_mut()
            .ok_or_else(|| anyhow!("Auth service not available"))?;

        let response = client.login_by_login_token(&login_token).await?;

        // Store tokens securely
        self.token_storage
            .store_access_token(&response.access_token)
            .await?;
        self.token_storage
            .store_refresh_token(&response.refresh_token)
            .await?;

        // Extract user info from token
        let user_info = self.extract_user_info(&response.access_token)?;

        // Update current user
        let mut user_guard = self.current_user.write().await;
        *user_guard = Some(user_info.clone());

        debug!("User logged in successfully: {}", user_info.username);
        Ok(user_info)
    }
}
