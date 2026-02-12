use crate::get_secure_channel;
use anyhow::{Ok, Result, anyhow};
use proto_gen::auth::{
    EmailPasswordCredentials, GetLoginTokenResponse, LoginByLoginTokenRequest, LoginRequest,
    RefreshTokenRequest, RegisterRequest, TokenResponse, login_request::Credential,
    proto_auth_service_client::ProtoAuthServiceClient,
};
use tonic::transport::Channel;
use tracing::error;

/// gRPC client for authentication service
#[derive(Debug, Clone)]
pub struct AuthClient {
    client: Option<ProtoAuthServiceClient<Channel>>,
}

impl AuthClient {
    /// Create a new gRPC client connected to the auth service
    pub async fn new() -> Self {
        let mut auth_client = Self { client: None };
        auth_client.get_or_init_client().await.ok();
        auth_client
    }

    pub async fn login_by_password(
        &mut self,
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
    async fn login(&mut self, data: LoginRequest) -> Result<TokenResponse> {
        let mut client = self.get_or_init_client().await?;
        let response = client.login(data).await.map_err(|e| {
            error!("Login failed: {}", e);
            anyhow!("Login failed: {}", e)
        })?;

        Ok(response.into_inner())
    }

    /// Register a new user
    pub async fn register(
        &mut self,
        username: impl Into<String>,
        email: impl Into<String>,
        password: impl Into<String>,
        display_name: Option<String>,
    ) -> Result<TokenResponse> {
        let mut client = self.get_or_init_client().await?;
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
    pub async fn refresh_token(
        &mut self,
        refresh_token: impl Into<String>,
    ) -> Result<TokenResponse> {
        let refresh_token: String = refresh_token.into();
        let mut client = self.get_or_init_client().await?;
        let mut request = tonic::Request::new(RefreshTokenRequest {});
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
        &mut self,
        login_token: impl Into<String>,
    ) -> Result<TokenResponse> {
        let mut client = self.get_or_init_client().await?;
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

    pub async fn get_login_token(&mut self) -> Result<GetLoginTokenResponse> {
        let mut client = self.get_or_init_client().await?;
        let response = client.get_login_token(()).await.map_err(|e| {
            error!("Get login token failed: {}", e);
            anyhow!("Get login token failed: {}", e)
        })?;

        Ok(response.into_inner())
    }

    async fn get_or_init_client(&mut self) -> Result<ProtoAuthServiceClient<Channel>> {
        match self.client.take() {
            Some(client) => Ok(client),
            None => {
                let channel = get_secure_channel().await?;
                let client = ProtoAuthServiceClient::new(channel);
                self.client = Some(client.clone());
                Ok(client)
            }
        }
    }
}
