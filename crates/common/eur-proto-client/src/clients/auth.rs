use anyhow::{Ok, Result, anyhow};
use eur_proto::proto_auth_service::GetLoginTokenResponse;
pub use eur_proto::proto_auth_service::{
    EmailPasswordCredentials, LoginByLoginTokenRequest, LoginRequest, RefreshTokenRequest,
    RegisterRequest, TokenResponse, login_request::Credential,
    proto_auth_service_client::ProtoAuthServiceClient,
};
use tonic::transport::Channel;
use tracing::{debug, error};

use crate::get_secure_channel;

/// gRPC client for authentication service
#[derive(Clone)]
pub struct AuthClient {
    base_url: String,
}

impl AuthClient {
    /// Create a new gRPC client connected to the auth service
    pub async fn new(base_url: Option<String>) -> Result<Self> {
        let base_url = base_url.unwrap_or(
            std::env::var("API_BASE_URL").unwrap_or("https://api.eurora-labs.com".to_string()),
        );
        Ok(Self { base_url })
    }

    async fn try_init_client(&self) -> Result<ProtoAuthServiceClient<Channel>> {
        let channel = get_secure_channel(self.base_url.clone())
            .await?
            .ok_or_else(|| anyhow!("Failed to initialize auth channel"))?;

        let client = ProtoAuthServiceClient::new(channel);

        debug!("Connected to auth service at {}", self.base_url);
        Ok(client)
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
        let mut client = self.try_init_client().await?;
        let response = client.login(data).await.map_err(|e| {
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
        let mut client = self.try_init_client().await?;
        let response = client
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
        let refresh_token: String = refresh_token.into();
        let mut client = self.try_init_client().await?;
        let mut request = tonic::Request::new(RefreshTokenRequest {
            refresh_token: refresh_token.clone(),
        });
        request.metadata_mut().insert(
            "authorization",
            format!("Bearer {}", refresh_token).parse().unwrap(),
        );
        let response = client.refresh_token(request).await.map_err(|e| {
            error!("Token refresh failed: {}", e);
            anyhow!("Token refresh failed: {}", e)
        })?;

        Ok(response.into_inner())
    }

    pub async fn login_by_login_token(
        &self,
        login_token: impl Into<String>,
    ) -> Result<TokenResponse> {
        let mut client = self.try_init_client().await?;
        let login_token = login_token.into();
        let response = client
            .login_by_login_token(LoginByLoginTokenRequest {
                token: login_token.clone(),
            })
            .await
            .map_err(|e| {
                error!("Login by login token failed: {}", e);
                anyhow!("Login by login token failed: {}", e)
            })?;

        Ok(response.into_inner())
    }

    pub async fn get_login_token(&self) -> Result<GetLoginTokenResponse> {
        let mut client = self.try_init_client().await?;
        let response = client.get_login_token(()).await.map_err(|e| {
            error!("Get login token failed: {}", e);
            anyhow!("Get login token failed: {}", e)
        })?;

        Ok(response.into_inner())
    }
}
