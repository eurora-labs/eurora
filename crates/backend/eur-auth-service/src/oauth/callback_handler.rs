use anyhow::{Result, anyhow};
use axum::{
    Router,
    extract::{Query, State},
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::get,
};
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, RedirectUrl,
    TokenResponse as OAuth2TokenResponse, TokenUrl, basic::BasicClient,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::net::TcpListener;
use tonic::Request;
use tracing::{error, info, warn};

use crate::AuthService;
use crate::oauth::google::{GoogleOAuthConfig, GoogleUserInfo};
use eur_proto::proto_auth_service::{
    LoginRequest, Provider, ThirdPartyCredentials, login_request::Credential,
    proto_auth_service_server::ProtoAuthService,
};

/// OAuth callback query parameters
#[derive(Debug, Deserialize)]
pub struct CallbackQuery {
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
}

/// OAuth callback handler state
#[derive(Clone)]
pub struct CallbackState {
    pub auth_service: Arc<AuthService>,
    pub oauth_config: GoogleOAuthConfig,
}

/// Google OAuth callback handler
pub struct GoogleCallbackHandler {
    state: CallbackState,
}

impl GoogleCallbackHandler {
    /// Create a new callback handler
    pub fn new(auth_service: Arc<AuthService>, oauth_config: GoogleOAuthConfig) -> Self {
        let state = CallbackState {
            auth_service,
            oauth_config,
        };

        Self { state }
    }

    /// Start the callback server
    pub async fn start_server(&self, port: u16) -> Result<()> {
        let app = Router::new()
            .route("/auth/google/callback", get(handle_google_callback))
            .with_state(self.state.clone());

        let listener = TcpListener::bind(format!("127.0.0.1:{}", port))
            .await
            .map_err(|e| anyhow!("Failed to bind to port {}: {}", port, e))?;

        info!("OAuth callback server listening on port {}", port);

        axum::serve(listener, app)
            .await
            .map_err(|e| anyhow!("Server error: {}", e))?;

        Ok(())
    }
}

/// Handle Google OAuth callback
async fn handle_google_callback(
    Query(params): Query<CallbackQuery>,
    State(state): State<CallbackState>,
) -> impl IntoResponse {
    info!("Received Google OAuth callback");

    // Check for OAuth errors
    if let Some(error) = params.error {
        error!("OAuth error: {}", error);
        return (
            StatusCode::BAD_REQUEST,
            Html(format!("<h1>OAuth Error</h1><p>{}</p>", error)),
        );
    }

    // Extract authorization code
    let code = match params.code {
        Some(code) => code,
        None => {
            error!("Missing authorization code in callback");
            return (
                StatusCode::BAD_REQUEST,
                Html("<h1>Error</h1><p>Missing authorization code</p>".to_string()),
            );
        }
    };

    // TODO: Validate CSRF state token here
    // For now, we'll log it but not validate
    if let Some(state_token) = params.state {
        info!("Received state token: {}", state_token);
        // In production, you should validate this against stored state
    }

    // Exchange code for tokens and get user info
    match exchange_code_and_login(&state, &code).await {
        Ok(response_html) => (StatusCode::OK, Html(response_html)),
        Err(e) => {
            error!("Failed to process OAuth callback: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html(format!(
                    "<h1>Error</h1><p>Failed to process login: {}</p>",
                    e
                )),
            )
        }
    }
}

/// Exchange authorization code for tokens and perform login
async fn exchange_code_and_login(state: &CallbackState, code: &str) -> Result<String> {
    info!("Exchanging authorization code for tokens");

    // Create OAuth client for token exchange
    let google_client_id = ClientId::new(state.oauth_config.client_id.clone());
    let google_client_secret = ClientSecret::new(state.oauth_config.client_secret.clone());

    let auth_url = AuthUrl::new("https://accounts.google.com/o/oauth2/v2/auth".to_string())
        .map_err(|e| anyhow!("Invalid authorization endpoint URL: {}", e))?;

    let token_url = TokenUrl::new("https://www.googleapis.com/oauth2/v3/token".to_string())
        .map_err(|e| anyhow!("Invalid token endpoint URL: {}", e))?;

    let redirect_url = RedirectUrl::new(state.oauth_config.redirect_uri.clone())
        .map_err(|e| anyhow!("Invalid redirect URL: {}", e))?;

    let client = BasicClient::new(google_client_id)
        .set_client_secret(google_client_secret)
        .set_auth_uri(auth_url)
        .set_token_uri(token_url)
        .set_redirect_uri(redirect_url);

    // Exchange code for tokens
    let http_client = reqwest::ClientBuilder::new()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| anyhow!("Failed to build HTTP client: {}", e))?;

    let token_result = client
        .exchange_code(AuthorizationCode::new(code.to_string()))
        .request_async(&http_client)
        .await
        .map_err(|e| anyhow!("Failed to exchange authorization code: {}", e))?;

    let access_token = token_result.access_token().secret();
    info!("Successfully obtained access token");

    // Get user info from Google
    let user_info = get_google_user_info(access_token).await?;
    info!("Retrieved user info for: {}", user_info.email);

    // Create ID token for third-party login
    // In a real implementation, you might want to create a proper JWT ID token
    // For now, we'll use the access token as the ID token
    let id_token = access_token.clone();

    // Create third-party credentials
    let third_party_creds = ThirdPartyCredentials {
        provider: Provider::Google as i32,
        id_token,
    };

    // Create login request
    let login_request = LoginRequest {
        credential: Some(Credential::ThirdParty(third_party_creds)),
    };

    // Call auth service login
    match state.auth_service.login(Request::new(login_request)).await {
        Ok(response) => {
            let token_response = response.into_inner();
            info!("Successfully logged in user via Google OAuth");

            // Return success page with tokens (in production, you'd handle this differently)
            Ok(format!(
                r#"
                <h1>Login Successful!</h1>
                <p>Welcome, {}!</p>
                <p>You have been successfully authenticated via Google.</p>
                <script>
                    // In a real app, you'd send these tokens to your frontend
                    console.log('Access token received');
                    // Close this window or redirect to your app
                    setTimeout(() => window.close(), 3000);
                </script>
                "#,
                user_info.name
            ))
        }
        Err(e) => {
            error!("Auth service login failed: {}", e);
            Err(anyhow!("Login failed: {}", e))
        }
    }
}

/// Get user info from Google using access token
async fn get_google_user_info(access_token: &str) -> Result<GoogleUserInfo> {
    info!("Fetching user info from Google");

    let client = reqwest::Client::new();
    let response = client
        .get("https://www.googleapis.com/oauth2/v2/userinfo")
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|e| anyhow!("Failed to fetch user info: {}", e))?;

    if !response.status().is_success() {
        error!("Google API returned error: {}", response.status());
        return Err(anyhow!("Failed to fetch user info from Google"));
    }

    let user_info: GoogleUserInfo = response
        .json()
        .await
        .map_err(|e| anyhow!("Failed to parse user info response: {}", e))?;

    info!("Successfully fetched user info for: {}", user_info.email);

    Ok(user_info)
}

/// Create and start a Google OAuth callback handler
pub async fn start_google_callback_server(auth_service: Arc<AuthService>, port: u16) -> Result<()> {
    let oauth_config = GoogleOAuthConfig::from_env()?;
    let handler = GoogleCallbackHandler::new(auth_service, oauth_config);
    handler.start_server(port).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_callback_query_parsing() {
        // Test successful callback
        let query = "code=test_code&state=test_state";
        let parsed: CallbackQuery = serde_urlencoded::from_str(query).unwrap();
        assert_eq!(parsed.code, Some("test_code".to_string()));
        assert_eq!(parsed.state, Some("test_state".to_string()));
        assert_eq!(parsed.error, None);

        // Test error callback
        let query = "error=access_denied&state=test_state";
        let parsed: CallbackQuery = serde_urlencoded::from_str(query).unwrap();
        assert_eq!(parsed.code, None);
        assert_eq!(parsed.state, Some("test_state".to_string()));
        assert_eq!(parsed.error, Some("access_denied".to_string()));
    }
}
