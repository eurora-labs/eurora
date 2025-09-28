use std::env;

use anyhow::{Result, anyhow};
use oauth2::{
    AuthUrl, ClientId, ClientSecret, CsrfToken, RedirectUrl, RevocationUrl, Scope, TokenUrl,
    basic::BasicClient,
};
use serde::{Deserialize, Serialize};
use tracing::debug;

/// Google OAuth configuration
#[derive(Debug, Clone)]
pub struct GoogleOAuthConfig {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
}

impl GoogleOAuthConfig {
    /// Create a new GoogleOAuthConfig from environment variables
    pub fn from_env() -> Result<Self> {
        let client_id = env::var("GOOGLE_CLIENT_ID")
            .map_err(|_| anyhow!("GOOGLE_CLIENT_ID environment variable not set"))?;
        let client_secret = env::var("GOOGLE_CLIENT_SECRET")
            .map_err(|_| anyhow!("GOOGLE_CLIENT_SECRET environment variable not set"))?;
        let redirect_uri = env::var("GOOGLE_REDIRECT_URI")
            .unwrap_or_else(|_| "http://localhost:5173/auth/google/callback".to_string());

        Ok(Self {
            client_id,
            client_secret,
            redirect_uri,
        })
    }
}

/// Google OAuth client wrapper for URL generation
pub struct GoogleOAuthClient {
    config: GoogleOAuthConfig,
}

impl GoogleOAuthClient {
    /// Create a new Google OAuth client
    pub fn new(config: GoogleOAuthConfig) -> Result<Self> {
        Ok(Self { config })
    }

    /// Generate the authorization URL for Google OAuth
    /// Returns (authorization_url, csrf_state)
    pub fn get_authorization_url(&self) -> Result<(String, String)> {
        debug!("Generating Google OAuth authorization URL");

        let google_client_id = ClientId::new(self.config.client_id.clone());
        let google_client_secret = ClientSecret::new(self.config.client_secret.clone());

        let auth_url = AuthUrl::new("https://accounts.google.com/o/oauth2/v2/auth".to_string())
            .map_err(|e| anyhow!("Invalid authorization endpoint URL: {}", e))?;

        let token_url = TokenUrl::new("https://www.googleapis.com/oauth2/v3/token".to_string())
            .map_err(|e| anyhow!("Invalid token endpoint URL: {}", e))?;

        let redirect_url = RedirectUrl::new(self.config.redirect_uri.clone())
            .map_err(|e| anyhow!("Invalid redirect URL: {}", e))?;

        // Set up the config for the Google OAuth2 process
        let client = BasicClient::new(google_client_id)
            .set_client_secret(google_client_secret)
            .set_auth_uri(auth_url)
            .set_token_uri(token_url)
            .set_revocation_url(
                RevocationUrl::new("https://accounts.google.com/o/oauth2/revoke".to_string())
                    .map_err(|e| anyhow!("Invalid revocation endpoint URL: {}", e))?,
            )
            .set_redirect_uri(redirect_url);

        // Generate the authorization URL to which we'll redirect the user
        let (authorize_url, csrf_state) = client
            .authorize_url(CsrfToken::new_random)
            // Request access to OpenID Connect scopes for user authentication
            .add_scope(Scope::new("openid".to_string()))
            .add_scope(Scope::new("email".to_string()))
            .add_scope(Scope::new("profile".to_string()))
            .url();

        debug!("Generated authorization URL: {}", authorize_url);

        Ok((authorize_url.to_string(), csrf_state.secret().clone()))
    }

    /// Generate the authorization URL for Google OAuth with a custom state
    /// Returns the authorization_url
    pub fn get_authorization_url_with_state(&self, state: &str) -> Result<String> {
        debug!("Generating Google OAuth authorization URL with custom state");

        let google_client_id = ClientId::new(self.config.client_id.clone());
        let google_client_secret = ClientSecret::new(self.config.client_secret.clone());

        let auth_url = AuthUrl::new("https://accounts.google.com/o/oauth2/v2/auth".to_string())
            .map_err(|e| anyhow!("Invalid authorization endpoint URL: {}", e))?;

        let token_url = TokenUrl::new("https://www.googleapis.com/oauth2/v3/token".to_string())
            .map_err(|e| anyhow!("Invalid token endpoint URL: {}", e))?;

        let redirect_url = RedirectUrl::new(self.config.redirect_uri.clone())
            .map_err(|e| anyhow!("Invalid redirect URL: {}", e))?;

        // Set up the config for the Google OAuth2 process
        let client = BasicClient::new(google_client_id)
            .set_client_secret(google_client_secret)
            .set_auth_uri(auth_url)
            .set_token_uri(token_url)
            .set_revocation_url(
                RevocationUrl::new("https://accounts.google.com/o/oauth2/revoke".to_string())
                    .map_err(|e| anyhow!("Invalid revocation endpoint URL: {}", e))?,
            )
            .set_redirect_uri(redirect_url);

        // Generate the authorization URL with custom state
        let (authorize_url, _) = client
            .authorize_url(|| CsrfToken::new(state.to_string()))
            // Request access to OpenID Connect scopes for user authentication
            .add_scope(Scope::new("openid".to_string()))
            .add_scope(Scope::new("email".to_string()))
            .add_scope(Scope::new("profile".to_string()))
            .url();

        debug!(
            "Generated authorization URL with custom state: {}",
            authorize_url
        );

        Ok(authorize_url.to_string())
    }
}

/// Google user info response from userinfo endpoint
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
}

/// Create a Google OAuth client from environment variables
pub fn create_google_oauth_client() -> Result<GoogleOAuthClient> {
    let config = GoogleOAuthConfig::from_env()?;
    GoogleOAuthClient::new(config)
}
