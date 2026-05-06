use std::env;
use std::time::Duration;

use openidconnect::{
    AuthenticationFlow, AuthorizationCode, ClientId, ClientSecret, CsrfToken, EndpointMaybeSet,
    EndpointNotSet, EndpointSet, IssuerUrl, Nonce, OAuth2TokenResponse, PkceCodeChallenge,
    PkceCodeVerifier, RedirectUrl, Scope, TokenResponse,
    core::{CoreClient, CoreIdTokenClaims, CoreProviderMetadata, CoreResponseType},
};
use secrecy::{ExposeSecret, SecretString};

use super::OAuthError;

type DiscoveredClient = CoreClient<
    EndpointSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointMaybeSet,
    EndpointMaybeSet,
>;

#[derive(Debug, Clone)]
pub struct GoogleOAuthConfig {
    pub client_id: SecretString,
    pub client_secret: SecretString,
    pub redirect_uri: String,
}

impl GoogleOAuthConfig {
    pub fn from_env() -> Result<Self, OAuthError> {
        let client_id = SecretString::from(
            env::var("GOOGLE_CLIENT_ID")
                .map_err(|_| OAuthError::MissingEnvVar("GOOGLE_CLIENT_ID"))?,
        );
        let client_secret = SecretString::from(
            env::var("GOOGLE_CLIENT_SECRET")
                .map_err(|_| OAuthError::MissingEnvVar("GOOGLE_CLIENT_SECRET"))?,
        );
        let redirect_uri = env::var("GOOGLE_REDIRECT_URI")
            .map_err(|_| OAuthError::MissingEnvVar("GOOGLE_REDIRECT_URI"))?;

        Ok(Self {
            client_id,
            client_secret,
            redirect_uri,
        })
    }
}

pub struct GoogleOAuthClient {
    client: DiscoveredClient,
    redirect_uri: String,
    /// Shared HTTP client kept alive for connection pooling.
    http: reqwest::Client,
}

fn build_http_client() -> Result<reqwest::Client, OAuthError> {
    Ok(reqwest::ClientBuilder::new()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(30))
        .build()?)
}

impl GoogleOAuthClient {
    pub async fn discover(config: GoogleOAuthConfig) -> Result<Self, OAuthError> {
        let issuer_url = IssuerUrl::new("https://accounts.google.com".to_string())
            .map_err(|e| OAuthError::Discovery(e.to_string()))?;

        let http = build_http_client()?;

        tracing::info!("Discovering OpenID Connect provider metadata");
        let provider_metadata = CoreProviderMetadata::discover_async(issuer_url, &http)
            .await
            .map_err(|e| OAuthError::Discovery(e.to_string()))?;

        let client_id = ClientId::new(config.client_id.expose_secret().to_owned());
        let client_secret = Some(ClientSecret::new(
            config.client_secret.expose_secret().to_owned(),
        ));
        let redirect_uri = config.redirect_uri;
        let redirect_url = RedirectUrl::new(redirect_uri.clone())
            .map_err(|e| OAuthError::InvalidUrl(e.to_string()))?;

        let client =
            CoreClient::from_provider_metadata(provider_metadata, client_id, client_secret)
                .set_redirect_uri(redirect_url);

        Ok(Self {
            client,
            redirect_uri,
            http,
        })
    }

    pub fn redirect_uri(&self) -> &str {
        &self.redirect_uri
    }

    /// Build the authorisation URL. The caller supplies a pre-computed
    /// challenge (returned by `PkceCodeChallenge::new_random_sha256`) so
    /// the verifier never has to be hashed twice.
    pub fn authorization_url(
        &self,
        state: &str,
        pkce_challenge: PkceCodeChallenge,
        nonce: Nonce,
    ) -> String {
        let state_str = state.to_string();
        let (authorize_url, _, _) = self
            .client
            .authorize_url(
                AuthenticationFlow::<CoreResponseType>::AuthorizationCode,
                || CsrfToken::new(state_str),
                || nonce,
            )
            .add_scope(Scope::new("email".to_string()))
            .add_scope(Scope::new("profile".to_string()))
            .set_pkce_challenge(pkce_challenge)
            .url();

        authorize_url.to_string()
    }

    pub async fn exchange_code(
        &self,
        code: &str,
        pkce_verifier: String,
        nonce: &Nonce,
    ) -> Result<GoogleUserInfo, OAuthError> {
        let token_response = self
            .client
            .exchange_code(AuthorizationCode::new(code.to_string()))
            .map_err(|e| OAuthError::CodeExchange(e.to_string()))?
            .set_pkce_verifier(PkceCodeVerifier::new(pkce_verifier))
            .request_async(&self.http)
            .await
            .map_err(|e| OAuthError::CodeExchange(e.to_string()))?;

        let id_token = token_response
            .id_token()
            .ok_or(OAuthError::MissingField("id_token"))?;

        let verifier = self.client.id_token_verifier();
        let claims: &CoreIdTokenClaims = id_token
            .claims(&verifier, nonce)
            .map_err(|e| OAuthError::TokenVerification(e.to_string()))?;

        let id = claims.subject().to_string();
        let email = claims
            .email()
            .ok_or(OAuthError::MissingField("email"))?
            .to_string();
        let verified_email = claims.email_verified().unwrap_or(false);

        let display_name = claims
            .name()
            .and_then(|localized| localized.get(None).map(|v| v.to_string()))
            .filter(|s| !s.is_empty());

        let access_token = SecretString::from(token_response.access_token().secret().to_string());
        let refresh_token = token_response
            .refresh_token()
            .map(|t| SecretString::from(t.secret().to_string()));
        let expires_in = token_response.expires_in();

        // Prefer the scopes the provider says it actually granted; fall
        // back to the scopes we asked for. Either way, joined with spaces
        // for storage in `oauth_credentials.scope`.
        let scope = token_response
            .scopes()
            .map(|scopes| {
                scopes
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .unwrap_or_else(|| "openid email profile".to_string());

        Ok(GoogleUserInfo {
            id,
            email,
            verified_email,
            display_name,
            access_token,
            refresh_token,
            expires_in,
            scope,
        })
    }
}

#[derive(Debug)]
pub struct GoogleUserInfo {
    pub id: String,
    pub email: String,
    pub verified_email: bool,
    pub display_name: Option<String>,
    pub access_token: SecretString,
    pub refresh_token: Option<SecretString>,
    pub expires_in: Option<std::time::Duration>,
    pub scope: String,
}
