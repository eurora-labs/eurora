use anyhow::{Ok, Result, anyhow};
use proto_gen::auth::{
    EmailPasswordCredentials, LoginByLoginTokenRequest, LoginRequest, RefreshTokenRequest,
    RegisterRequest, TokenResponse, login_request::Credential,
    proto_auth_service_client::ProtoAuthServiceClient,
};
use tokio::sync::watch;
use tonic::transport::Channel;

#[derive(Debug, Clone)]
pub struct AuthClient {
    channel_rx: watch::Receiver<Channel>,
}

impl AuthClient {
    pub fn new(channel_rx: watch::Receiver<Channel>) -> Self {
        Self { channel_rx }
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
        self.login(req).await
    }

    async fn login(&self, data: LoginRequest) -> Result<TokenResponse> {
        let mut client = self.client();
        let response = client.login(data).await.map_err(|e| {
            tracing::error!("Login failed: {}", e);
            anyhow!("Login failed: {}", e)
        })?;

        Ok(response.into_inner())
    }

    pub async fn register(
        &self,
        email: impl Into<String>,
        password: impl Into<String>,
        display_name: Option<String>,
    ) -> Result<TokenResponse> {
        let mut client = self.client();
        let response = client
            .register(RegisterRequest {
                email: email.into(),
                password: password.into(),
                display_name,
            })
            .await
            .map_err(|e| {
                tracing::error!("Registration failed: {}", e);
                anyhow!("Registration failed: {}", e)
            })?;

        Ok(response.into_inner())
    }

    pub async fn refresh_token(&self, refresh_token: impl Into<String>) -> Result<TokenResponse> {
        let refresh_token: String = refresh_token.into();
        let mut client = self.client();
        let mut request = tonic::Request::new(RefreshTokenRequest {});
        request.metadata_mut().insert(
            "authorization",
            format!("Bearer {}", refresh_token).parse().unwrap(),
        );
        let response = client.refresh_token(request).await.map_err(|e| {
            tracing::error!("Token refresh failed: {}", e);
            anyhow!("Token refresh failed: {}", e)
        })?;

        Ok(response.into_inner())
    }

    pub async fn login_by_login_token(
        &self,
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
                tracing::error!("Login by login token failed: {}", e);
                anyhow!("Login by login token failed: {}", e)
            })?;

        Ok(response.into_inner())
    }

    pub async fn resend_verification_email(&self, access_token: impl Into<String>) -> Result<()> {
        let mut client = self.client();
        let mut request = tonic::Request::new(());
        request.metadata_mut().insert(
            "authorization",
            format!("Bearer {}", access_token.into()).parse().unwrap(),
        );
        client
            .resend_verification_email(request)
            .await
            .map_err(|e| {
                tracing::error!("Resend verification email failed: {}", e);
                anyhow!("Resend verification email failed: {}", e)
            })?;
        Ok(())
    }

    fn client(&self) -> ProtoAuthServiceClient<Channel> {
        let channel = self.channel_rx.borrow().clone();
        ProtoAuthServiceClient::new(channel)
    }
}
