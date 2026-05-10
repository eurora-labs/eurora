use std::env;
use std::time::Duration;

use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use url::Url;

use super::OAuthError;

#[derive(Debug, Clone)]
pub struct GitHubOAuthConfig {
    pub client_id: SecretString,
    pub client_secret: SecretString,
    pub redirect_uri: String,
    /// See [`crate::oauth::google::GoogleOAuthConfig::mobile_redirect_uri`].
    pub mobile_redirect_uri: Option<String>,
}

impl GitHubOAuthConfig {
    pub fn from_env() -> Result<Self, OAuthError> {
        let client_id = SecretString::from(
            env::var("GITHUB_CLIENT_ID")
                .map_err(|_| OAuthError::MissingEnvVar("GITHUB_CLIENT_ID"))?,
        );
        let client_secret = SecretString::from(
            env::var("GITHUB_CLIENT_SECRET")
                .map_err(|_| OAuthError::MissingEnvVar("GITHUB_CLIENT_SECRET"))?,
        );
        let redirect_uri = env::var("GITHUB_REDIRECT_URI")
            .map_err(|_| OAuthError::MissingEnvVar("GITHUB_REDIRECT_URI"))?;
        let mobile_redirect_uri = env::var("GITHUB_MOBILE_REDIRECT_URI")
            .ok()
            .filter(|s| !s.is_empty());

        Ok(Self {
            client_id,
            client_secret,
            redirect_uri,
            mobile_redirect_uri,
        })
    }
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
    /// Shared HTTP client kept alive for connection pooling.
    http: reqwest::Client,
}

fn build_http_client() -> Result<reqwest::Client, OAuthError> {
    Ok(reqwest::ClientBuilder::new()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(30))
        .build()?)
}

impl GitHubOAuthClient {
    pub fn new(config: GitHubOAuthConfig) -> Result<Self, OAuthError> {
        let http = build_http_client()?;
        Ok(Self { config, http })
    }

    pub fn redirect_uri(&self) -> &str {
        &self.config.redirect_uri
    }

    pub fn mobile_redirect_uri(&self) -> Option<&str> {
        self.config.mobile_redirect_uri.as_deref()
    }

    pub fn authorization_url(&self, state: &str, pkce_challenge: &str) -> String {
        Self::build_authorization_url(
            self.config.client_id.expose_secret(),
            &self.config.redirect_uri,
            state,
            pkce_challenge,
        )
    }

    pub fn mobile_authorization_url(&self, state: &str, pkce_challenge: &str) -> Option<String> {
        self.config.mobile_redirect_uri.as_deref().map(|uri| {
            Self::build_authorization_url(
                self.config.client_id.expose_secret(),
                uri,
                state,
                pkce_challenge,
            )
        })
    }

    fn build_authorization_url(
        client_id: &str,
        redirect_uri: &str,
        state: &str,
        pkce_challenge: &str,
    ) -> String {
        let mut url =
            Url::parse("https://github.com/login/oauth/authorize").expect("static URL must parse");
        url.query_pairs_mut()
            .append_pair("client_id", client_id)
            .append_pair("redirect_uri", redirect_uri)
            .append_pair("state", state)
            .append_pair("scope", "user:email")
            .append_pair("code_challenge", pkce_challenge)
            .append_pair("code_challenge_method", "S256");
        url.into()
    }

    pub async fn exchange_code(
        &self,
        code: &str,
        pkce_verifier: &str,
    ) -> Result<GitHubUserInfo, OAuthError> {
        self.exchange_code_with_redirect(code, pkce_verifier, &self.config.redirect_uri)
            .await
    }

    pub async fn mobile_exchange_code(
        &self,
        code: &str,
        pkce_verifier: &str,
    ) -> Result<GitHubUserInfo, OAuthError> {
        let redirect_uri = self
            .config
            .mobile_redirect_uri
            .as_deref()
            .ok_or(OAuthError::MissingEnvVar("GITHUB_MOBILE_REDIRECT_URI"))?;
        self.exchange_code_with_redirect(code, pkce_verifier, redirect_uri)
            .await
    }

    async fn exchange_code_with_redirect(
        &self,
        code: &str,
        pkce_verifier: &str,
        redirect_uri: &str,
    ) -> Result<GitHubUserInfo, OAuthError> {
        let token_resp: GitHubTokenResponse = self
            .http
            .post("https://github.com/login/oauth/access_token")
            .header("Accept", "application/json")
            .form(&[
                ("client_id", self.config.client_id.expose_secret()),
                ("client_secret", self.config.client_secret.expose_secret()),
                ("code", code),
                ("redirect_uri", redirect_uri),
                ("code_verifier", pkce_verifier),
            ])
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        if let Some(error) = &token_resp.error {
            let desc = token_resp.error_description.as_deref().unwrap_or(error);
            return Err(OAuthError::CodeExchange(desc.to_string()));
        }

        let access_token = token_resp
            .access_token
            .ok_or(OAuthError::MissingField("access_token"))?;
        let scope = token_resp.scope.unwrap_or_default();

        let user: GitHubApiUser = self
            .http
            .get("https://api.github.com/user")
            .bearer_auth(&access_token)
            .header("User-Agent", "eurora-auth-service")
            .send()
            .await
            .map_err(|e| OAuthError::UserInfoFetch(e.to_string()))?
            .error_for_status()
            .map_err(|e| OAuthError::UserInfoFetch(e.to_string()))?
            .json()
            .await
            .map_err(|e| OAuthError::UserInfoFetch(e.to_string()))?;

        let emails: Vec<GitHubApiEmail> = self
            .http
            .get("https://api.github.com/user/emails")
            .bearer_auth(&access_token)
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
            .ok_or(OAuthError::MissingField("verified primary email"))?;

        Ok(GitHubUserInfo {
            id: user.id.to_string(),
            email: primary_email.email.clone(),
            verified_email: primary_email.verified,
            display_name: user.name.filter(|s| !s.is_empty()).or(Some(user.login)),
            picture: user.avatar_url,
            access_token: SecretString::from(access_token),
            scope,
        })
    }
}

#[derive(Debug)]
pub struct GitHubUserInfo {
    pub id: String,
    pub email: String,
    pub verified_email: bool,
    pub display_name: Option<String>,
    pub picture: Option<String>,
    pub access_token: SecretString,
    pub scope: String,
}
