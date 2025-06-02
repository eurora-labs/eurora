//! gRPC client for communicating with the eur-auth-service.

use anyhow::{Result, anyhow};
use eur_proto::proto_auth_service::{
    EmailPasswordCredentials, LoginRequest, RefreshTokenRequest, RegisterRequest, TokenResponse,
    login_request::Credential, proto_auth_service_client::ProtoAuthServiceClient,
};
use tonic::transport::Channel;
use tracing::{error, info};

/// gRPC client for authentication service
pub struct AuthClient {
    channel: Channel,
}

impl AuthClient {
    /// Create a new gRPC client with a channel
    pub fn new(channel: Channel) -> Result<Self> {
        Ok(Self { channel })
    }

    /// Create a new gRPC client connected to the auth service
    pub async fn connect(service_url: &str) -> Result<Self> {
        let channel = Channel::from_shared(service_url.to_string())?
            .connect()
            .await
            .map_err(|e| anyhow!("Failed to connect to auth service: {}", e))?;

        info!("Connected to auth service at {}", service_url);
        Ok(Self { channel })
    }
    pub fn get_client(&self) -> ProtoAuthServiceClient<Channel> {
        ProtoAuthServiceClient::new(self.channel.clone())
    }

    /// Login with email/username and password
    pub async fn login(&self, data: LoginRequest) -> Result<TokenResponse> {
        let mut client = self.get_client();
        let response = client.login(data).await.map_err(|e| {
            error!("Login failed: {}", e);
            anyhow!("Login failed: {}", e)
        })?;

        info!("Login successful for user:");
        Ok(response.into_inner())
    }

    /// Register a new user
    pub async fn register(
        &self,
        username: &str,
        email: &str,
        password: &str,
        display_name: Option<String>,
    ) -> Result<TokenResponse> {
        let mut client = self.get_client();
        let request = RegisterRequest {
            username: username.to_string(),
            email: email.to_string(),
            password: password.to_string(),
            display_name,
        };
        let response = client.register(request).await.map_err(|e| {
            error!("Registration failed: {}", e);
            anyhow!("Registration failed: {}", e)
        })?;

        info!("Registration successful for user: {}", username);
        Ok(response.into_inner())
    }

    /// Refresh access token using refresh token
    pub async fn refresh_token(&self, refresh_token: &str) -> Result<TokenResponse> {
        let mut client = self.get_client();
        let request = RefreshTokenRequest {
            refresh_token: refresh_token.to_string(),
        };

        let response = client.refresh_token(request).await.map_err(|e| {
            error!("Token refresh failed: {}", e);
            anyhow!("Token refresh failed: {}", e)
        })?;

        info!("Token refresh successful");
        Ok(response.into_inner())
    }
}
