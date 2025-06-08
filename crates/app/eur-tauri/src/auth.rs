use anyhow::{Result, anyhow};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use eur_auth_client::AuthClient;
use eur_proto::proto_auth_service::{GetLoginTokenResponse, LoginRequest};
use eur_secret::{Sensitive, secret};
use serde::{Deserialize, Serialize};
use tracing::info;
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    sub: String,
    name: String,
    exp: i64,
}

pub struct JwtConfig {
    refresh_offset: i64,
}

pub struct AuthManager {
    auth_client: AuthClient,
    jwt_config: JwtConfig,
}

impl AuthManager {
    pub(super) const ACCESS_TOKEN_HANDLE: &'static str = "AUTH_ACCESS_TOKEN";
    pub(super) const REFRESH_TOKEN_HANDLE: &'static str = "AUTH_REFRESH_TOKEN";

    pub async fn new() -> Result<Self> {
        let refresh_offset = std::env::var("JWT_REFRESH_OFFSET").unwrap_or("15".to_string());
        Ok(Self {
            auth_client: AuthClient::new(None).await?,
            jwt_config: JwtConfig {
                refresh_offset: refresh_offset
                    .parse()
                    .map_err(|_| anyhow!("Invalid JWT_REFRESH_OFFSET format"))?,
            },
        })
    }

    pub async fn login(
        &self,
        login: impl Into<String>,
        password: impl Into<String>,
    ) -> Result<Sensitive<String>> {
        let response = self.auth_client.login_by_password(login, password).await?;

        // Store tokens securely
        store_access_token(response.access_token.clone())?;
        store_refresh_token(response.refresh_token.clone())?;

        Ok(Sensitive(response.access_token))
    }

    fn get_access_token(&self) -> Result<Sensitive<String>> {
        secret::retrieve(Self::ACCESS_TOKEN_HANDLE, secret::Namespace::BuildKind)?
            .ok_or_else(|| anyhow!("No access token found"))
    }

    fn get_refresh_token(&self) -> Result<Sensitive<String>> {
        secret::retrieve(Self::REFRESH_TOKEN_HANDLE, secret::Namespace::BuildKind)?
            .ok_or_else(|| anyhow!("No refresh token found"))
    }

    pub fn get_access_token_payload(&self) -> Result<Claims> {
        let token = self.get_access_token()?;
        extract_claims(&token.0)
    }

    pub fn get_refresh_token_payload(&self) -> Result<Claims> {
        let token = self.get_refresh_token()?;
        extract_claims(&token.0)
    }

    pub async fn get_or_refresh_access_token(&self) -> Result<Sensitive<String>> {
        // Check if refresh_threshold has passed
        if self.get_access_token_payload().unwrap().exp < chrono::Utc::now().timestamp() {
            return self.refresh_tokens().await;
        }

        self.refresh_tokens().await
    }

    pub async fn refresh_tokens(&self) -> Result<Sensitive<String>> {
        let refresh_token = self.get_refresh_token()?;

        let response = self.auth_client.refresh_token(&refresh_token.0).await?;

        // Store tokens securely
        store_access_token(response.access_token.clone())?;
        store_refresh_token(response.refresh_token.clone())?;

        Ok(Sensitive(response.access_token))
    }

    pub async fn get_login_token(&self) -> Result<GetLoginTokenResponse> {
        let token = self.auth_client.get_login_token().await?;
        info!("Login token: {}", token.token);
        Ok(token)
    }
    pub async fn login_by_login_token(&self, login_token: String) -> Result<Sensitive<String>> {
        let response = self.auth_client.login_by_login_token(login_token).await?;

        // Store tokens securely
        store_access_token(response.access_token.clone())?;
        store_refresh_token(response.refresh_token.clone())?;

        Ok(Sensitive(response.access_token))
    }
}

fn extract_claims(token: &str) -> Result<Claims> {
    let mut parts = token.splitn(3, '.');
    let _header_b64 = parts.next().unwrap();
    let payload_b64 = parts.next().unwrap();
    let payload = URL_SAFE_NO_PAD.decode(payload_b64).ok().unwrap();
    let payload_json: Claims = serde_json::from_slice(&payload).ok().unwrap();

    Ok(payload_json)
}

fn store_access_token(token: String) -> Result<()> {
    secret::persist(
        AuthManager::ACCESS_TOKEN_HANDLE,
        &Sensitive(token),
        secret::Namespace::BuildKind,
    )
    .map_err(|e| anyhow!("Failed to store access token: {}", e))
}

fn store_refresh_token(token: String) -> Result<()> {
    secret::persist(
        AuthManager::REFRESH_TOKEN_HANDLE,
        &Sensitive(token),
        secret::Namespace::BuildKind,
    )
    .map_err(|e| anyhow!("Failed to store refresh token: {}", e))
}
