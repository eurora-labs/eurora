//! HTTP client for the Eurora auth service.
//!
//! Talks JSON over HTTPS using the same shared base URL as the rest of
//! the desktop / mobile HTTP services. The base URL is read from
//! [`euro_endpoint::EndpointManager`] on every call so a backend switch
//! propagates without needing to rebuild the client.

use std::sync::Arc;

use auth_core::{
    AssociateLoginTokenRequest, AuthErrorResponse, CheckEmailRequest, CheckEmailResponse,
    GoogleIdTokenLoginRequest, LoginByLoginTokenRequest, LoginRequest,
    MobileThirdPartyAuthUrlRequest, RegisterRequest, ThirdPartyAuthUrlRequest,
    ThirdPartyAuthUrlResponse, TokenResponse, VerifyEmailRequest,
};
use euro_endpoint::EndpointManager;
use reqwest::{RequestBuilder, Response};
use serde::Serialize;
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
            .field("base_url", &self.endpoint_manager.current_url().as_str())
            .finish()
    }
}

impl AuthClient {
    pub fn new(endpoint_manager: Arc<EndpointManager>) -> Self {
        let http = endpoint_manager.client();
        Self {
            endpoint_manager,
            http,
        }
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
        send_typed(self.request("/auth/refresh", Some(refresh_token.as_ref()))).await
    }

    pub async fn logout(&self, refresh_token: impl AsRef<str>) -> AuthResult<()> {
        send_unit(self.request("/auth/logout", Some(refresh_token.as_ref()))).await
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
        send_unit(
            self.request("/auth/login-token/associate", Some(access_token.as_ref()))
                .json(&body),
        )
        .await
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
        send_unit(self.request(
            "/auth/email/resend-verification",
            Some(access_token.as_ref()),
        ))
        .await
    }

    pub async fn third_party_auth_url(
        &self,
        provider: auth_core::Provider,
    ) -> AuthResult<ThirdPartyAuthUrlResponse> {
        let body = ThirdPartyAuthUrlRequest { provider };
        self.post_json("/auth/oauth/url", &body, None).await
    }

    /// Mobile OAuth start: hand the backend a PKCE challenge and get
    /// back a provider-authorisation URL that, once the user finishes,
    /// 302s through `/auth/oauth/{provider}/mobile-callback` and lands
    /// on the device's `eurora://` scheme.
    pub async fn mobile_third_party_auth_url(
        &self,
        provider: auth_core::Provider,
        code_challenge: impl Into<String>,
    ) -> AuthResult<ThirdPartyAuthUrlResponse> {
        let body = MobileThirdPartyAuthUrlRequest {
            provider,
            code_challenge: code_challenge.into(),
            code_challenge_method: "S256".to_string(),
        };
        self.post_json("/auth/oauth/mobile/url", &body, None).await
    }

    /// Native Google sign-in: trade an iOS / Android-issued ID token
    /// for a session token pair.
    pub async fn login_by_google_id_token(
        &self,
        id_token: impl Into<String>,
        nonce: Option<String>,
    ) -> AuthResult<TokenResponse> {
        let body = GoogleIdTokenLoginRequest {
            id_token: id_token.into(),
            nonce,
        };
        self.post_json("/auth/oauth/google/id-token", &body, None)
            .await
    }

    async fn post_json<B, R>(&self, path: &str, body: &B, bearer: Option<&str>) -> AuthResult<R>
    where
        B: Serialize + ?Sized,
        R: DeserializeOwned,
    {
        send_typed(self.request(path, bearer).json(body)).await
    }

    fn request(&self, path: &str, bearer: Option<&str>) -> RequestBuilder {
        let mut builder = self.http.post(self.endpoint_manager.url(path));
        if let Some(token) = bearer {
            builder = builder.bearer_auth(token);
        }
        builder
    }
}

async fn send_typed<R: DeserializeOwned>(builder: RequestBuilder) -> AuthResult<R> {
    let response = builder.send().await.map_err(AuthError::from_transport)?;
    let status = response.status();
    let url = response.url().to_string();
    if !status.is_success() {
        let body = decode_error_body(response).await;
        tracing::warn!(%url, %status, ?body, "auth client: non-success response");
        return Err(AuthError::from_http_response(status, body));
    }
    let bytes = response.bytes().await.map_err(AuthError::from_transport)?;
    serde_json::from_slice(&bytes)
        .map_err(|e| AuthError::Transient(anyhow::anyhow!("failed to decode auth response: {e}")))
}

async fn send_unit(builder: RequestBuilder) -> AuthResult<()> {
    let response = builder.send().await.map_err(AuthError::from_transport)?;
    let status = response.status();
    let url = response.url().to_string();
    if !status.is_success() {
        let body = decode_error_body(response).await;
        tracing::warn!(%url, %status, ?body, "auth client: non-success response");
        return Err(AuthError::from_http_response(status, body));
    }
    Ok(())
}

async fn decode_error_body(response: Response) -> Option<AuthErrorResponse> {
    let bytes = response.bytes().await.ok()?;
    if bytes.is_empty() {
        return None;
    }
    serde_json::from_slice(&bytes).ok()
}
