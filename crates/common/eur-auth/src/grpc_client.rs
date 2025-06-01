//! gRPC client for communicating with the eur-auth-service.

use anyhow::{Result, anyhow};
use eur_proto::proto_auth_service::{
    EmailPasswordCredentials, LoginRequest, RefreshTokenRequest, RegisterRequest, TokenResponse,
    login_request::Credential, proto_auth_service_client::ProtoAuthServiceClient,
};
use tonic::transport::Channel;
use tracing::{error, info};

/// gRPC client for authentication service
pub struct AuthGrpcClient {
    client: ProtoAuthServiceClient<Channel>,
}

impl AuthGrpcClient {
    /// Create a new gRPC client connected to the auth service
    pub async fn new(service_url: &str) -> Result<Self> {
        let channel = Channel::from_shared(service_url.to_string())?
            .connect()
            .await
            .map_err(|e| anyhow!("Failed to connect to auth service: {}", e))?;

        let client = ProtoAuthServiceClient::new(channel);

        info!("Connected to auth service at {}", service_url);
        Ok(Self { client })
    }

    /// Login with email/username and password
    pub async fn login(&mut self, login: &str, password: &str) -> Result<TokenResponse> {
        let credentials = EmailPasswordCredentials {
            login: login.to_string(),
            password: password.to_string(),
        };

        let request = LoginRequest {
            credential: Some(Credential::EmailPassword(credentials)),
        };

        let response = self.client.login(request).await.map_err(|e| {
            error!("Login failed: {}", e);
            anyhow!("Login failed: {}", e)
        })?;

        info!("Login successful for user: {}", login);
        Ok(response.into_inner())
    }

    /// Register a new user
    pub async fn register(
        &mut self,
        username: &str,
        email: &str,
        password: &str,
        display_name: Option<String>,
    ) -> Result<TokenResponse> {
        let request = RegisterRequest {
            username: username.to_string(),
            email: email.to_string(),
            password: password.to_string(),
            display_name,
        };

        let response = self.client.register(request).await.map_err(|e| {
            error!("Registration failed: {}", e);
            anyhow!("Registration failed: {}", e)
        })?;

        info!("Registration successful for user: {}", username);
        Ok(response.into_inner())
    }

    /// Refresh access token using refresh token
    pub async fn refresh_token(&mut self, refresh_token: &str) -> Result<TokenResponse> {
        let request = RefreshTokenRequest {
            refresh_token: refresh_token.to_string(),
        };

        let response = self.client.refresh_token(request).await.map_err(|e| {
            error!("Token refresh failed: {}", e);
            anyhow!("Token refresh failed: {}", e)
        })?;

        info!("Token refresh successful");
        Ok(response.into_inner())
    }
}
