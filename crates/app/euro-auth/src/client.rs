//! HTTP client for the Eurora auth service.
//!
//! Talks JSON over HTTPS using the same shared base URL as the rest of
//! the desktop / mobile HTTP services. The base URL is read from
//! [`euro_endpoint::EndpointManager`] on every call so a backend switch
//! propagates without needing to rebuild the client.

use std::sync::Arc;

use auth_core::{
    AssociateLoginTokenRequest, AuthErrorResponse, CheckEmailRequest, CheckEmailResponse,
    LoginByLoginTokenRequest, LoginRequest, RegisterRequest, ThirdPartyAuthUrlRequest,
    ThirdPartyAuthUrlResponse, TokenResponse, VerifyEmailRequest,
};
use euro_endpoint::EndpointManager;
use reqwest::Response;
use serde::de::DeserializeOwned;

use crate::error::{AuthError, AuthResult};

#[derive(Clone)]
pub struct AuthClient {
    endpoint_manager: Arc<EndpointManager>,
    http: reqwest::Client,
}

impl std::fmt::Debug for AuthClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuthClient")
            .field("base_url", &self.endpoint_manager.current_url())
            .finish()
    }
}

impl AuthClient {
    pub fn new(endpoint_manager: Arc<EndpointManager>) -> Self {
        Self {
            endpoint_manager,
            http: reqwest::Client::new(),
        }
    }

    fn url(&self, path: &str) -> String {
        let base = self.endpoint_manager.current_url();
        let trimmed = base.trim_end_matches('/');
        format!("{trimmed}{path}")
    }

    pub async fn login_by_password(
        &self,
        login: impl Into<String>,
        password: impl Into<String>,
    ) -> AuthResult<TokenResponse> {
        let body = LoginRequest::EmailPassword {
            login: login.into(),
            password: password.into(),
        };
        self.post_json("/auth/login", &body, None).await
    }

    pub async fn register(
        &self,
        email: impl Into<String>,
        password: impl Into<String>,
        display_name: Option<String>,
    ) -> AuthResult<TokenResponse> {
        let body = RegisterRequest {
            email: email.into(),
            password: password.into(),
            display_name,
        };
        self.post_json("/auth/register", &body, None).await
    }

    pub async fn refresh_token(&self, refresh_token: impl AsRef<str>) -> AuthResult<TokenResponse> {
        self.post_empty("/auth/refresh", Some(refresh_token.as_ref()))
            .await
    }

    pub async fn logout(&self, refresh_token: impl AsRef<str>) -> AuthResult<()> {
        let response = self
            .request("/auth/logout", Some(refresh_token.as_ref().to_owned()))
            .send()
            .await
            .map_err(AuthError::from_transport)?;
        let _: serde_json::Value = decode_or_error(response).await?;
        Ok(())
    }

    pub async fn login_by_login_token(
        &self,
        login_token: impl Into<String>,
    ) -> AuthResult<TokenResponse> {
        let body = LoginByLoginTokenRequest {
            token: login_token.into(),
        };
        self.post_json("/auth/login-token/exchange", &body, None)
            .await
    }

    pub async fn associate_login_token(
        &self,
        access_token: impl AsRef<str>,
        code_challenge: impl Into<String>,
    ) -> AuthResult<()> {
        let body = AssociateLoginTokenRequest {
            code_challenge: code_challenge.into(),
        };
        let response = self
            .request(
                "/auth/login-token/associate",
                Some(access_token.as_ref().to_owned()),
            )
            .json(&body)
            .send()
            .await
            .map_err(AuthError::from_transport)?;
        let _: serde_json::Value = decode_or_error(response).await?;
        Ok(())
    }

    pub async fn check_email(&self, email: impl Into<String>) -> AuthResult<CheckEmailResponse> {
        let body = CheckEmailRequest {
            email: email.into(),
        };
        self.post_json("/auth/email/check", &body, None).await
    }

    pub async fn verify_email(&self, token: impl Into<String>) -> AuthResult<TokenResponse> {
        let body = VerifyEmailRequest {
            token: token.into(),
        };
        self.post_json("/auth/email/verify", &body, None).await
    }

    pub async fn resend_verification_email(&self, access_token: impl AsRef<str>) -> AuthResult<()> {
        let response = self
            .request(
                "/auth/email/resend-verification",
                Some(access_token.as_ref().to_owned()),
            )
            .send()
            .await
            .map_err(AuthError::from_transport)?;
        let _: serde_json::Value = decode_or_error(response).await?;
        Ok(())
    }

    pub async fn third_party_auth_url(
        &self,
        provider: auth_core::Provider,
    ) -> AuthResult<ThirdPartyAuthUrlResponse> {
        let body = ThirdPartyAuthUrlRequest { provider };
        self.post_json("/auth/oauth/url", &body, None).await
    }

    async fn post_json<B, R>(&self, path: &str, body: &B, bearer: Option<&str>) -> AuthResult<R>
    where
        B: serde::Serialize + ?Sized,
        R: DeserializeOwned,
    {
        let response = self
            .request(path, bearer.map(str::to_owned))
            .json(body)
            .send()
            .await
            .map_err(AuthError::from_transport)?;
        decode_or_error(response).await
    }

    async fn post_empty<R>(&self, path: &str, bearer: Option<&str>) -> AuthResult<R>
    where
        R: DeserializeOwned,
    {
        let response = self
            .request(path, bearer.map(str::to_owned))
            .send()
            .await
            .map_err(AuthError::from_transport)?;
        decode_or_error(response).await
    }

    fn request(&self, path: &str, bearer: Option<String>) -> reqwest::RequestBuilder {
        let mut builder = self.http.post(self.url(path));
        if let Some(token) = bearer {
            builder = builder.bearer_auth(token);
        }
        builder
    }
}

async fn decode_or_error<R: DeserializeOwned>(response: Response) -> AuthResult<R> {
    let status = response.status();
    if status.is_success() {
        // Some endpoints return an empty body on success; fall back to
        // deserializing `null` so callers can use `()` / `serde_json::Value`.
        let bytes = response.bytes().await.map_err(AuthError::from_transport)?;
        if bytes.is_empty() {
            return serde_json::from_slice(b"null").map_err(|e| {
                AuthError::Transient(anyhow::anyhow!("failed to decode empty response: {e}"))
            });
        }
        return serde_json::from_slice(&bytes).map_err(|e| {
            AuthError::Transient(anyhow::anyhow!("failed to decode auth response: {e}"))
        });
    }

    let body = decode_error_body(response).await;
    Err(AuthError::from_http_response(status, body))
}

async fn decode_error_body(response: Response) -> Option<AuthErrorResponse> {
    let bytes = response.bytes().await.ok()?;
    if bytes.is_empty() {
        return None;
    }
    serde_json::from_slice(&bytes).ok()
}
