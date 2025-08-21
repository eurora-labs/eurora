use anyhow::{Result, anyhow};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use eur_proto_client::auth::AuthClient;
use eur_secret::{Sensitive, secret};
use rand::{TryRngCore, rngs::OsRng};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tracing::error;

// Re-export shared types for convenience
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    sub: String,
    name: String,
    exp: i64,
}

#[derive(Clone)]
pub struct JwtConfig {
    refresh_offset: i64,
}

#[derive(Clone)]
pub struct AuthManager {
    auth_client: AuthClient,
    jwt_config: JwtConfig,
}

pub const ACCESS_TOKEN_HANDLE: &'static str = "AUTH_ACCESS_TOKEN";
pub const REFRESH_TOKEN_HANDLE: &'static str = "AUTH_REFRESH_TOKEN";

impl AuthManager {
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
        secret::retrieve(ACCESS_TOKEN_HANDLE, secret::Namespace::Global)?
            .ok_or_else(|| anyhow!("No access token found"))
    }

    fn get_refresh_token(&self) -> Result<Sensitive<String>> {
        secret::retrieve(REFRESH_TOKEN_HANDLE, secret::Namespace::Global)?
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
        // Check if token has expired or is close to expiration
        match self.get_access_token_payload() {
            Ok(claims) => {
                let now = chrono::Utc::now().timestamp();
                let expiry_with_offset = claims.exp - self.jwt_config.refresh_offset;

                if now < expiry_with_offset {
                    // Token is still valid
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

    pub async fn refresh_tokens(&self) -> Result<Sensitive<String>> {
        let refresh_token = self.get_refresh_token()?;

        let response = self.auth_client.refresh_token(&refresh_token.0).await?;

        // Store tokens securely
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
    let _header_b64 = parts
        .next()
        .ok_or_else(|| anyhow!("Invalid JWT format: missing header"))?;
    let payload_b64 = parts
        .next()
        .ok_or_else(|| anyhow!("Invalid JWT format: missing payload"))?;
    let payload = URL_SAFE_NO_PAD
        .decode(payload_b64)
        .map_err(|e| anyhow!("Failed to decode JWT payload: {}", e))?;
    let payload_json: Claims = serde_json::from_slice(&payload)
        .map_err(|e| anyhow!("Failed to parse JWT claims: {}", e))?;
    Ok(payload_json)
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
