use crate::client::AuthClient;
use anyhow::{Result, anyhow};
use auth_core::Claims;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use euro_secret::{ExposeSecret, SecretString, secret};
use jsonwebtoken::dangerous::insecure_decode;
use rand::Rng;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tokio::sync::{Mutex, watch};
use tonic::transport::Channel;

#[derive(Debug, Clone, Copy)]
struct JwtConfig {
    refresh_offset_seconds: i64,
}

impl JwtConfig {
    fn from_env() -> Self {
        let minutes: i64 = std::env::var("JWT_REFRESH_OFFSET")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(15)
            .max(0);
        Self {
            refresh_offset_seconds: minutes.saturating_mul(60),
        }
    }
}

pub const ACCESS_TOKEN_HANDLE: &str = "AUTH_ACCESS_TOKEN";
pub const REFRESH_TOKEN_HANDLE: &str = "AUTH_REFRESH_TOKEN";

/// Shared authentication state.
///
/// `AuthManager` is cheap to clone — all clones share the same inner state via
/// an `Arc`. In particular, they share a single refresh lock so that concurrent
/// callers coalesce into one server-side refresh: the winning task performs the
/// rotation, and queued callers observe the freshly stored access token after
/// the lock is released. This is critical because the backend invalidates a
/// refresh token on first use, so naive concurrent refreshes would cause all
/// but one caller to receive `InvalidToken` and log the user out.
#[derive(Debug, Clone)]
pub struct AuthManager {
    inner: Arc<Inner>,
}

#[derive(Debug)]
struct Inner {
    auth_client: AuthClient,
    jwt_config: JwtConfig,
    refresh_lock: Mutex<()>,
}

impl AuthManager {
    pub fn new(channel_rx: watch::Receiver<Channel>) -> Self {
        Self {
            inner: Arc::new(Inner {
                auth_client: AuthClient::new(channel_rx),
                jwt_config: JwtConfig::from_env(),
                refresh_lock: Mutex::new(()),
            }),
        }
    }

    pub async fn login(
        &self,
        login: impl Into<String>,
        password: impl Into<String>,
    ) -> Result<SecretString> {
        let response = self
            .inner
            .auth_client
            .login_by_password(login, password)
            .await?;

        store_access_token(response.access_token.clone())?;
        store_refresh_token(response.refresh_token.clone())?;

        Ok(SecretString::from(response.access_token))
    }

    pub async fn register(
        &self,
        email: impl Into<String>,
        password: impl Into<String>,
    ) -> Result<SecretString> {
        let response = self
            .inner
            .auth_client
            .register(email, password, None)
            .await?;

        store_access_token(response.access_token.clone())?;
        store_refresh_token(response.refresh_token.clone())?;

        Ok(SecretString::from(response.access_token))
    }

    fn get_access_token(&self) -> Result<SecretString> {
        secret::retrieve(ACCESS_TOKEN_HANDLE)?.ok_or_else(|| anyhow!("No access token found"))
    }

    fn get_refresh_token(&self) -> Result<SecretString> {
        secret::retrieve(REFRESH_TOKEN_HANDLE)?.ok_or_else(|| anyhow!("No refresh token found"))
    }

    pub fn get_access_token_payload(&self) -> Result<Claims> {
        let token = self.get_access_token()?;
        let token = insecure_decode::<Claims>(token.expose_secret())?;
        Ok(token.claims)
    }

    pub fn get_refresh_token_payload(&self) -> Result<Claims> {
        let token = self.get_refresh_token()?;
        let token = insecure_decode::<Claims>(token.expose_secret())?;
        Ok(token.claims)
    }

    /// Returns a valid access token, refreshing from the server only if the
    /// stored token is missing or within the refresh-offset window of expiry.
    pub async fn get_or_refresh_access_token(&self) -> Result<SecretString> {
        if self.has_fresh_access_token() {
            return self.get_access_token();
        }
        self.ensure_refresh().await
    }

    /// Force a refresh, coalescing with any concurrent refresh already in
    /// flight. If another task completes a refresh while this one is waiting
    /// for the lock, the freshly stored token is returned without a second
    /// round-trip to the server.
    pub async fn refresh_tokens(&self) -> Result<SecretString> {
        self.ensure_refresh().await
    }

    async fn ensure_refresh(&self) -> Result<SecretString> {
        let _guard = self.inner.refresh_lock.lock().await;

        // Double-checked: another task may have refreshed while we waited.
        if self.has_fresh_access_token() {
            return self.get_access_token();
        }

        self.perform_refresh().await?;
        self.get_access_token()
    }

    async fn perform_refresh(&self) -> Result<()> {
        let refresh_token = self.get_refresh_token()?;
        let response = self
            .inner
            .auth_client
            .refresh_token(refresh_token.expose_secret())
            .await
            .map_err(|e| {
                tracing::warn!("Token refresh failed: {e}");
                e
            })?;

        store_access_token(response.access_token)?;
        store_refresh_token(response.refresh_token)?;
        Ok(())
    }

    fn has_fresh_access_token(&self) -> bool {
        let Ok(claims) = self.get_access_token_payload() else {
            return false;
        };
        let now = chrono::Utc::now().timestamp();
        now < claims
            .exp
            .saturating_sub(self.inner.jwt_config.refresh_offset_seconds)
    }

    pub async fn get_login_tokens(&self) -> Result<(String, String)> {
        let mut verifier_bytes = vec![0u8; 32];
        rand::rng().fill_bytes(&mut verifier_bytes);

        let code_verifier = URL_SAFE_NO_PAD.encode(verifier_bytes);
        let mut hasher = Sha256::new();
        hasher.update(&code_verifier);
        let code_challenge_raw = hasher.finalize();
        let code_challenge = URL_SAFE_NO_PAD.encode(code_challenge_raw);

        Ok((code_verifier, code_challenge))
    }

    pub async fn resend_verification_email(&self) -> Result<()> {
        let access_token = self.get_access_token()?;
        self.inner
            .auth_client
            .resend_verification_email(access_token.expose_secret())
            .await
    }

    pub async fn login_by_login_token(&self, login_token: String) -> Result<SecretString> {
        let response = self
            .inner
            .auth_client
            .login_by_login_token(login_token)
            .await?;

        store_access_token(response.access_token.clone())?;
        store_refresh_token(response.refresh_token.clone())?;

        Ok(SecretString::from(response.access_token))
    }
}

fn store_access_token(token: String) -> Result<()> {
    secret::persist(ACCESS_TOKEN_HANDLE, &SecretString::from(token))
        .map_err(|e| anyhow!("Failed to store access token: {}", e))
}

fn store_refresh_token(token: String) -> Result<()> {
    secret::persist(REFRESH_TOKEN_HANDLE, &SecretString::from(token))
        .map_err(|e| anyhow!("Failed to store refresh token: {}", e))
}
