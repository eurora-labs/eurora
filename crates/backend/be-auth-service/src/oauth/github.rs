use std::env;
use std::time::Duration;

use serde::Deserialize;

use super::OAuthError;

#[derive(Debug, Clone)]
pub struct GitHubOAuthConfig {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
}

impl GitHubOAuthConfig {
    pub fn from_env() -> Result<Self, OAuthError> {
        let client_id = env::var("GITHUB_CLIENT_ID")
            .map_err(|_| OAuthError::MissingEnvVar("GITHUB_CLIENT_ID"))?;
        let client_secret = env::var("GITHUB_CLIENT_SECRET")
            .map_err(|_| OAuthError::MissingEnvVar("GITHUB_CLIENT_SECRET"))?;
        let redirect_uri = env::var("GITHUB_REDIRECT_URI")
            .unwrap_or_else(|_| "http://localhost:5173/auth/github/callback".to_string());

        Ok(Self {
            client_id,
            client_secret,
            redirect_uri,
        })
    }
}

fn build_http_client() -> Result<reqwest::Client, OAuthError> {
    reqwest::ClientBuilder::new()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| OAuthError::HttpClient(e.to_string()))
}

#[derive(Deserialize)]
struct GitHubTokenResponse {
    access_token: Option<String>,
    #[allow(dead_code)]
    token_type: Option<String>,
    scope: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GitHubApiUser {
    id: i64,
    login: String,
    name: Option<String>,
    avatar_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GitHubApiEmail {
    email: String,
    primary: bool,
    verified: bool,
}

pub struct GitHubOAuthClient {
    config: GitHubOAuthConfig,
}

impl GitHubOAuthClient {
    pub fn new(config: GitHubOAuthConfig) -> Self {
        Self { config }
    }

    pub fn redirect_uri(&self) -> &str {
        &self.config.redirect_uri
    }

    pub fn get_authorization_url(&self, state: &str, pkce_challenge: &str) -> String {
        format!(
            "https://github.com/login/oauth/authorize?client_id={}&redirect_uri={}&state={}&scope=user:email&code_challenge={}&code_challenge_method=S256",
            url_encode(&self.config.client_id),
            url_encode(&self.config.redirect_uri),
            url_encode(state),
            url_encode(pkce_challenge),
        )
    }

    pub async fn exchange_code(
        &self,
        code: &str,
        pkce_verifier: &str,
    ) -> Result<GitHubUserInfo, OAuthError> {
        let http_client = build_http_client()?;

        let token_resp: GitHubTokenResponse = http_client
            .post("https://github.com/login/oauth/access_token")
            .header("Accept", "application/json")
            .form(&[
                ("client_id", self.config.client_id.as_str()),
                ("client_secret", self.config.client_secret.as_str()),
                ("code", code),
                ("redirect_uri", self.config.redirect_uri.as_str()),
                ("code_verifier", pkce_verifier),
            ])
            .send()
            .await
            .map_err(|e| OAuthError::CodeExchange(e.to_string()))?
            .error_for_status()
            .map_err(|e| OAuthError::CodeExchange(e.to_string()))?
            .json()
            .await
            .map_err(|e| OAuthError::CodeExchange(e.to_string()))?;

        if let Some(error) = &token_resp.error {
            let desc = token_resp.error_description.as_deref().unwrap_or(error);
            return Err(OAuthError::CodeExchange(desc.to_string()));
        }

        let access_token = token_resp
            .access_token
            .ok_or_else(|| OAuthError::CodeExchange("Missing access_token in response".into()))?;
        let scope = token_resp.scope.unwrap_or_default();

        let user: GitHubApiUser = http_client
            .get("https://api.github.com/user")
            .header("Authorization", format!("Bearer {}", access_token))
            .header("User-Agent", "eurora-auth-service")
            .send()
            .await
            .map_err(|e| OAuthError::UserInfoFetch(e.to_string()))?
            .error_for_status()
            .map_err(|e| OAuthError::UserInfoFetch(e.to_string()))?
            .json()
            .await
            .map_err(|e| OAuthError::UserInfoFetch(e.to_string()))?;

        let emails: Vec<GitHubApiEmail> = http_client
            .get("https://api.github.com/user/emails")
            .header("Authorization", format!("Bearer {}", access_token))
            .header("User-Agent", "eurora-auth-service")
            .send()
            .await
            .map_err(|e| OAuthError::UserInfoFetch(e.to_string()))?
            .error_for_status()
            .map_err(|e| OAuthError::UserInfoFetch(e.to_string()))?
            .json()
            .await
            .map_err(|e| OAuthError::UserInfoFetch(e.to_string()))?;

        let primary_email = emails
            .iter()
            .find(|e| e.primary && e.verified)
            .or_else(|| emails.iter().find(|e| e.verified))
            .ok_or(OAuthError::MissingEmail)?;

        Ok(GitHubUserInfo {
            id: user.id.to_string(),
            email: primary_email.email.clone(),
            verified_email: primary_email.verified,
            name: user.name.unwrap_or(user.login.clone()),
            username: user.login,
            picture: user.avatar_url,
            access_token,
            scope,
        })
    }
}

#[derive(Debug)]
pub struct GitHubUserInfo {
    pub id: String,
    pub email: String,
    pub verified_email: bool,
    pub name: String,
    pub username: String,
    pub picture: Option<String>,
    pub access_token: String,
    pub scope: String,
}

fn url_encode(s: &str) -> String {
    percent_encoding::utf8_percent_encode(s, percent_encoding::NON_ALPHANUMERIC).to_string()
}
