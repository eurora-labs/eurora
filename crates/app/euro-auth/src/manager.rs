use crate::client::AuthClient;
use anyhow::{Result, anyhow};
use auth_core::Claims;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use euro_secret::{Sensitive, secret};
use jsonwebtoken::dangerous::insecure_decode;
use log::error;
use rand::{TryRngCore, rngs::OsRng};
use sha2::{Digest, Sha256};
use tokio::sync::watch;
use tonic::transport::Channel;

#[derive(Debug, Clone)]
pub struct JwtConfig {
    refresh_offset: i64,
}

#[derive(Debug, Clone)]
pub struct AuthManager {
    auth_client: AuthClient,
    jwt_config: JwtConfig,
}

pub const ACCESS_TOKEN_HANDLE: &str = "AUTH_ACCESS_TOKEN";
pub const REFRESH_TOKEN_HANDLE: &str = "AUTH_REFRESH_TOKEN";

impl AuthManager {
    pub fn new(channel_rx: watch::Receiver<Channel>) -> Self {
        let refresh_offset: i64 = std::env::var("JWT_REFRESH_OFFSET")
            .unwrap_or("15".to_string())
            .parse()
            .unwrap_or(15);
        Self {
            auth_client: AuthClient::new(channel_rx),
            jwt_config: JwtConfig { refresh_offset },
        }
    }

    pub async fn login(
        &mut self,
        login: impl Into<String>,
        password: impl Into<String>,
    ) -> Result<Sensitive<String>> {
        let response = self.auth_client.login_by_password(login, password).await?;

        store_access_token(response.access_token.clone())?;
        store_refresh_token(response.refresh_token.clone())?;

        Ok(Sensitive(response.access_token))
    }

    pub async fn register(
        &mut self,
        username: impl Into<String>,
        email: impl Into<String>,
        password: impl Into<String>,
    ) -> Result<Sensitive<String>> {
        let response = self
            .auth_client
            .register(username, email, password, None)
            .await?;

        store_access_token(response.access_token.clone())?;
        store_refresh_token(response.refresh_token.clone())?;

        Ok(Sensitive(response.access_token))
    }

    fn get_access_token(&self) -> Result<Sensitive<String>> {
        secret::retrieve(ACCESS_TOKEN_HANDLE, secret::Namespace::Global)?
            .ok_or_else(|| anyhow!("No access token found"))
    }

    fn get_refresh_token(&self) -> Result<Sensitive<String>> {
        secret::retrieve(REFRESH_TOKEN_HANDLE, secret::Namespace::Global)?
            .ok_or_else(|| anyhow!("No refresh token found"))
    }

    pub fn get_access_token_payload(&self) -> Result<Claims> {
        let token = self.get_access_token()?;
        let token = insecure_decode::<Claims>(&token.0)?;
        Ok(token.claims)
    }

    pub fn get_refresh_token_payload(&self) -> Result<Claims> {
        let token = self.get_refresh_token()?;
        let token = insecure_decode::<Claims>(&token.0)?;
        Ok(token.claims)
    }

    pub async fn get_or_refresh_access_token(&mut self) -> Result<Sensitive<String>> {
        match self.get_access_token_payload() {
            Ok(claims) => {
                let now = chrono::Utc::now().timestamp();
                let expiry_with_offset = claims.exp - self.jwt_config.refresh_offset * 60;

                if now < expiry_with_offset {
                    self.get_access_token()
                } else {
                    self.refresh_tokens().await.map_err(|err| {
                        error!("Failed to refresh tokens: {}", err);
                        err
                    })?;
                    self.get_access_token()
                }
            }

            Err(_) => {
                self.refresh_tokens().await.map_err(|err| {
                    error!("Failed to refresh tokens: {}", err);
                    err
                })?;
                self.get_access_token()
            }
        }
    }

    pub async fn refresh_tokens(&mut self) -> Result<Sensitive<String>> {
        let refresh_token = self.get_refresh_token()?;

        let response = self.auth_client.refresh_token(&refresh_token.0).await?;

        store_access_token(response.access_token.clone())?;
        store_refresh_token(response.refresh_token.clone())?;

        Ok(Sensitive(response.access_token))
    }

    pub async fn get_login_tokens(&self) -> Result<(String, String)> {
        let mut verifier_bytes = vec![0u8; 32];
        OsRng.try_fill_bytes(&mut verifier_bytes).map_err(|e| {
            error!("Failed to generate random bytes: {}", e);
            anyhow!("Failed to generate random bytes")
        })?;

        let code_verifier = URL_SAFE_NO_PAD.encode(verifier_bytes);
        let mut hasher = Sha256::new();
        hasher.update(&code_verifier);
        let code_challenge_raw = hasher.finalize();
        let code_challenge = URL_SAFE_NO_PAD.encode(code_challenge_raw);

        Ok((code_verifier, code_challenge))
    }

    pub async fn login_by_login_token(&mut self, login_token: String) -> Result<Sensitive<String>> {
        let response = self.auth_client.login_by_login_token(login_token).await?;

        store_access_token(response.access_token.clone())?;
        store_refresh_token(response.refresh_token.clone())?;

        Ok(Sensitive(response.access_token))
    }
}

fn store_access_token(token: String) -> Result<()> {
    secret::persist(
        ACCESS_TOKEN_HANDLE,
        &Sensitive(token),
        secret::Namespace::Global,
    )
    .map_err(|e| anyhow!("Failed to store access token: {}", e))
}

fn store_refresh_token(token: String) -> Result<()> {
    secret::persist(
        REFRESH_TOKEN_HANDLE,
        &Sensitive(token),
        secret::Namespace::Global,
    )
    .map_err(|e| anyhow!("Failed to store refresh token: {}", e))
}
