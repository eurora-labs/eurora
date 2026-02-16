use std::env;
use std::time::Duration;

use openidconnect::{
    AuthenticationFlow, AuthorizationCode, ClientId, ClientSecret, CsrfToken, EndpointMaybeSet,
    EndpointNotSet, EndpointSet, IssuerUrl, Nonce, OAuth2TokenResponse, PkceCodeChallenge,
    PkceCodeVerifier, RedirectUrl, Scope, TokenResponse,
    core::{CoreClient, CoreIdTokenClaims, CoreProviderMetadata, CoreResponseType},
};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::info;

#[derive(Debug, Error)]
pub enum OAuthError {
    #[error("Missing environment variable: {0}")]
    MissingEnvVar(&'static str),
    #[error("OAuth discovery failed: {0}")]
    Discovery(String),
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),
    #[error("Code exchange failed: {0}")]
    CodeExchange(String),
    #[error("Missing ID token")]
    MissingIdToken,
    #[error("Token verification failed: {0}")]
    TokenVerification(String),
    #[error("Missing email in claims")]
    MissingEmail,
    #[error("HTTP client error: {0}")]
    HttpClient(String),
}

/// The concrete client type returned by `from_provider_metadata` + `set_redirect_uri`
type DiscoveredClient = CoreClient<
    EndpointSet,      // HasAuthUrl
    EndpointNotSet,   // HasDeviceAuthUrl
    EndpointNotSet,   // HasIntrospectionUrl
    EndpointNotSet,   // HasRevocationUrl
    EndpointMaybeSet, // HasTokenUrl
    EndpointMaybeSet, // HasUserInfoUrl
>;

#[derive(Debug, Clone)]
pub struct GoogleOAuthConfig {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
}

impl GoogleOAuthConfig {
    pub fn from_env() -> Result<Self, OAuthError> {
        let client_id = env::var("GOOGLE_CLIENT_ID")
            .map_err(|_| OAuthError::MissingEnvVar("GOOGLE_CLIENT_ID"))?;
        let client_secret = env::var("GOOGLE_CLIENT_SECRET")
            .map_err(|_| OAuthError::MissingEnvVar("GOOGLE_CLIENT_SECRET"))?;
        let redirect_uri = env::var("GOOGLE_REDIRECT_URI")
            .unwrap_or_else(|_| "http://localhost:5173/auth/google/callback".to_string());

        Ok(Self {
            client_id,
            client_secret,
            redirect_uri,
        })
    }
}

pub struct GoogleOAuthClient {
    client: DiscoveredClient,
}

fn build_http_client() -> Result<reqwest::Client, OAuthError> {
    reqwest::ClientBuilder::new()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| OAuthError::HttpClient(e.to_string()))
}

impl GoogleOAuthClient {
    pub async fn discover(config: GoogleOAuthConfig) -> Result<Self, OAuthError> {
        let issuer_url = IssuerUrl::new("https://accounts.google.com".to_string())
            .map_err(|e| OAuthError::InvalidUrl(e.to_string()))?;

        let http_client = build_http_client()?;

        info!("Discovering OpenID Connect provider metadata");
        let provider_metadata = CoreProviderMetadata::discover_async(issuer_url, &http_client)
            .await
            .map_err(|e| OAuthError::Discovery(e.to_string()))?;

        let client_id = ClientId::new(config.client_id);
        let client_secret = Some(ClientSecret::new(config.client_secret));
        let redirect_url = RedirectUrl::new(config.redirect_uri)
            .map_err(|e| OAuthError::InvalidUrl(e.to_string()))?;

        let client =
            CoreClient::from_provider_metadata(provider_metadata, client_id, client_secret)
                .set_redirect_uri(redirect_url);

        Ok(Self { client })
    }

    pub fn get_authorization_url_with_state_and_pkce(
        &self,
        state: &str,
        pkce_verifier: &str,
        nonce: &Nonce,
    ) -> String {
        info!("Generating Google OAuth authorization URL with custom state and PKCE");

        let pkce_code_verifier = PkceCodeVerifier::new(pkce_verifier.to_string());
        let pkce_challenge = PkceCodeChallenge::from_code_verifier_sha256(&pkce_code_verifier);

        let state_str = state.to_string();
        let nonce_clone = nonce.clone();
        let (authorize_url, _, _) = self
            .client
            .authorize_url(
                AuthenticationFlow::<CoreResponseType>::AuthorizationCode,
                || CsrfToken::new(state_str),
                move || nonce_clone,
            )
            .add_scope(Scope::new("email".to_string()))
            .add_scope(Scope::new("profile".to_string()))
            .set_pkce_challenge(pkce_challenge)
            .url();

        authorize_url.to_string()
    }

    /// The `nonce` verifies the ID token's nonce claim (OIDC replay protection).
    pub async fn exchange_code(
        &self,
        code: &str,
        pkce_verifier: String,
        nonce: Option<&Nonce>,
    ) -> Result<GoogleUserInfo, OAuthError> {
        let http_client = build_http_client()?;

        let token_response = self
            .client
            .exchange_code(AuthorizationCode::new(code.to_string()))
            .map_err(|e| OAuthError::CodeExchange(e.to_string()))?
            .set_pkce_verifier(PkceCodeVerifier::new(pkce_verifier))
            .request_async(&http_client)
            .await
            .map_err(|e| OAuthError::CodeExchange(e.to_string()))?;

        let id_token = token_response
            .id_token()
            .ok_or(OAuthError::MissingIdToken)?;

        let verifier = self.client.id_token_verifier();
        let claims: &CoreIdTokenClaims = match nonce {
            Some(expected_nonce) => id_token
                .claims(&verifier, expected_nonce)
                .map_err(|e| OAuthError::TokenVerification(e.to_string()))?,
            None => id_token
                .claims(&verifier, |_: Option<&Nonce>| Ok(()))
                .map_err(|e| OAuthError::TokenVerification(e.to_string()))?,
        };

        let subject = claims.subject().to_string();
        let email = claims.email().ok_or(OAuthError::MissingEmail)?.to_string();
        let email_verified = claims.email_verified().unwrap_or(false);

        let name = match claims.name() {
            Some(localized) => localized
                .get(None)
                .map(|v| v.to_string())
                .unwrap_or_default(),
            None => String::new(),
        };
        let given_name = claims
            .given_name()
            .and_then(|localized| localized.get(None).map(|v| v.to_string()));
        let family_name = claims
            .family_name()
            .and_then(|localized| localized.get(None).map(|v| v.to_string()));
        let picture = claims
            .picture()
            .and_then(|localized| localized.get(None).map(|v| v.to_string()));

        let access_token = token_response.access_token().secret().to_string();
        let refresh_token = token_response
            .refresh_token()
            .map(|t| t.secret().to_string());
        let expires_in = token_response.expires_in();

        Ok(GoogleUserInfo {
            id: subject,
            email,
            verified_email: email_verified,
            name,
            given_name,
            family_name,
            picture,
            locale: None,
            access_token,
            refresh_token,
            expires_in,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GoogleUserInfo {
    pub id: String,
    pub email: String,
    pub verified_email: bool,
    pub name: String,
    pub given_name: Option<String>,
    pub family_name: Option<String>,
    pub picture: Option<String>,
    pub locale: Option<String>,
    #[serde(skip)]
    pub access_token: String,
    #[serde(skip)]
    pub refresh_token: Option<String>,
    #[serde(skip)]
    pub expires_in: Option<std::time::Duration>,
}

pub async fn create_google_oauth_client() -> Result<GoogleOAuthClient, OAuthError> {
    let config = GoogleOAuthConfig::from_env()?;
    GoogleOAuthClient::discover(config).await
}
