use anyhow::{Ok, Result, anyhow};
use proto_gen::auth::{
    EmailPasswordCredentials, GetLoginTokenResponse, LoginByLoginTokenRequest, LoginRequest,
    RefreshTokenRequest, RegisterRequest, TokenResponse, login_request::Credential,
    proto_auth_service_client::ProtoAuthServiceClient,
};
use tokio::sync::watch;
use tonic::transport::Channel;
use tracing::error;

#[derive(Debug, Clone)]
pub struct AuthClient {
    channel_rx: watch::Receiver<Channel>,
}

impl AuthClient {
    pub fn new(channel_rx: watch::Receiver<Channel>) -> Self {
        Self { channel_rx }
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
        self.login(req).await
    }

    async fn login(&mut self, data: LoginRequest) -> Result<TokenResponse> {
        let mut client = self.client();
        let response = client.login(data).await.map_err(|e| {
            error!("Login failed: {}", e);
            anyhow!("Login failed: {}", e)
        })?;

        Ok(response.into_inner())
    }

    pub async fn register(
        &mut self,
        username: impl Into<String>,
        email: impl Into<String>,
        password: impl Into<String>,
        display_name: Option<String>,
    ) -> Result<TokenResponse> {
        let mut client = self.client();
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

    pub async fn refresh_token(
        &mut self,
        refresh_token: impl Into<String>,
    ) -> Result<TokenResponse> {
        let refresh_token: String = refresh_token.into();
        let mut client = self.client();
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
        let mut client = self.client();
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
        let mut client = self.client();
        let response = client.get_login_token(()).await.map_err(|e| {
            error!("Get login token failed: {}", e);
            anyhow!("Get login token failed: {}", e)
        })?;

        Ok(response.into_inner())
    }

    fn client(&self) -> ProtoAuthServiceClient<Channel> {
        let channel = self.channel_rx.borrow().clone();
        ProtoAuthServiceClient::new(channel)
    }
}
