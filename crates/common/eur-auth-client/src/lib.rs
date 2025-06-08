//! gRPC client for communicating with the eur-auth-service.

use anyhow::{Ok, Result, anyhow};
use eur_proto::proto_auth_service::GetLoginTokenResponse;
pub use eur_proto::proto_auth_service::{
    EmailPasswordCredentials, LoginByLoginTokenRequest, LoginRequest, RefreshTokenRequest,
    RegisterRequest, TokenResponse, login_request::Credential,
    proto_auth_service_client::ProtoAuthServiceClient,
};
use tonic::transport::Channel;
use tracing::{error, info};

/// gRPC client for authentication service
pub struct AuthClient {
    client: ProtoAuthServiceClient<Channel>,
}

impl AuthClient {
    /// Create a new gRPC client connected to the auth service
    pub async fn new(base_url: Option<String>) -> Result<Self> {
        let base_url = base_url.unwrap_or(
            std::env::var("API_BASE_URL").unwrap_or("http://localhost:50051".to_string()),
        );
        let channel = Channel::from_shared(base_url.clone())?
            .connect()
            .await
            .map_err(|e| anyhow!("Failed to connect to auth service: {}", e))?;

        let client = ProtoAuthServiceClient::new(channel);

        info!("Connected to auth service at {}", base_url);
        Ok(Self { client })
    }

    pub async fn login_by_password(
        &self,
        login: impl Into<String>,
        password: impl Into<String>,
    ) -> Result<TokenResponse> {
        let req = LoginRequest {
            credential: Some(Credential::EmailPassword(EmailPasswordCredentials {
                login: login.into(),
                password: password.into(),
            })),
        };
        Ok(self.login(req).await?)
    }

    /// Login with email/username and password
    async fn login(&self, data: LoginRequest) -> Result<TokenResponse> {
        let response = self.client.clone().login(data).await.map_err(|e| {
            error!("Login failed: {}", e);
            anyhow!("Login failed: {}", e)
        })?;

        Ok(response.into_inner())
    }

    /// Register a new user
    pub async fn register(
        &self,
        username: impl Into<String>,
        email: impl Into<String>,
        password: impl Into<String>,
        display_name: Option<String>,
    ) -> Result<TokenResponse> {
        let response = self
            .client
            .clone()
            .register(RegisterRequest {
                username: username.into(),
                email: email.into(),
                password: password.into(),
                display_name,
            })
            .await
            .map_err(|e| {
                error!("Registration failed: {}", e);
                anyhow!("Registration failed: {}", e)
            })?;

        Ok(response.into_inner())
    }

    /// Refresh access token using refresh token
    pub async fn refresh_token(&self, refresh_token: impl Into<String>) -> Result<TokenResponse> {
        let response = self
            .client
            .clone()
            .refresh_token(RefreshTokenRequest {
                refresh_token: refresh_token.into(),
            })
            .await
            .map_err(|e| {
                error!("Token refresh failed: {}", e);
                anyhow!("Token refresh failed: {}", e)
            })?;

        Ok(response.into_inner())
    }

    pub async fn login_by_login_token(
        &self,
        login_token: impl Into<String>,
    ) -> Result<TokenResponse> {
        let response = self
            .client
            .clone()
            .login_by_login_token(LoginByLoginTokenRequest {
                token: login_token.into(),
            })
            .await
            .map_err(|e| {
                error!("Login by login token failed: {}", e);
                anyhow!("Login by login token failed: {}", e)
            })?;

        Ok(response.into_inner())
    }

    pub async fn get_login_token(&self) -> Result<GetLoginTokenResponse> {
        let response = self.client.clone().get_login_token(()).await.map_err(|e| {
            error!("Get login token failed: {}", e);
            anyhow!("Get login token failed: {}", e)
        })?;

        Ok(response.into_inner())
    }
}
