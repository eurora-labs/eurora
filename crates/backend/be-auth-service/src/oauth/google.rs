use std::env;
use std::str::FromStr;
use std::time::Duration;

use openidconnect::{
    AuthenticationFlow, AuthorizationCode, ClientId, ClientSecret, CsrfToken, EndpointMaybeSet,
    EndpointNotSet, EndpointSet, IssuerUrl, Nonce, OAuth2TokenResponse, PkceCodeChallenge,
    PkceCodeVerifier, RedirectUrl, Scope, TokenResponse,
    core::{CoreClient, CoreIdToken, CoreIdTokenClaims, CoreProviderMetadata, CoreResponseType},
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
    /// Separate redirect URI for the mobile flow. When set, the mobile
    /// app's "Sign in with Google" button hits a backend endpoint
    /// (instead of bouncing through the web `/login` SPA) and Google
    /// calls back to *this* URL — also a backend endpoint — which
    /// completes the login server-side and 302s to the device's custom
    /// scheme. When unset, the mobile flow is unavailable and callers
    /// must use the web-mediated path.
    pub mobile_redirect_uri: Option<String>,
    /// Audience permitted on Google ID tokens minted for the *iOS*
    /// native client. Native iOS sign-in (via the GoogleSignIn SDK)
    /// issues tokens whose `aud` is the iOS OAuth client ID — distinct
    /// from the web/server `client_id` used by the redirect-based flow.
    /// When set, `verify_id_token` accepts both audiences. Android
    /// Credential Manager already issues tokens against the server
    /// client ID, so no Android equivalent is needed.
    pub ios_client_id: Option<SecretString>,
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
        let mobile_redirect_uri = env::var("GOOGLE_MOBILE_REDIRECT_URI")
            .ok()
            .filter(|s| !s.is_empty());
        let ios_client_id = env::var("GOOGLE_CLIENT_ID_IOS")
            .ok()
            .filter(|s| !s.is_empty())
            .map(SecretString::from);

        Ok(Self {
            client_id,
            client_secret,
            redirect_uri,
            mobile_redirect_uri,
            ios_client_id,
        })
    }
}

pub struct GoogleOAuthClient {
    client: DiscoveredClient,
    /// Same as `client` but bound to `mobile_redirect_uri`. Kept as a
    /// pre-built clone so per-request code paths don't have to mutate
    /// the shared client. `None` when no mobile redirect URI is
    /// configured.
    mobile_client: Option<DiscoveredClient>,
    redirect_uri: String,
    mobile_redirect_uri: Option<String>,
    /// Acceptable audience values for ID-token verification. Always
    /// contains the server `client_id`; also contains `ios_client_id`
    /// when a native-iOS client is configured.
    accepted_audiences: Vec<ClientId>,
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

        let client = CoreClient::from_provider_metadata(
            provider_metadata.clone(),
            client_id.clone(),
            client_secret.clone(),
        )
        .set_redirect_uri(redirect_url);

        let mobile_client = match &config.mobile_redirect_uri {
            Some(mobile_uri) => {
                let mobile_redirect_url = RedirectUrl::new(mobile_uri.clone())
                    .map_err(|e| OAuthError::InvalidUrl(e.to_string()))?;
                Some(
                    CoreClient::from_provider_metadata(
                        provider_metadata,
                        client_id.clone(),
                        client_secret,
                    )
                    .set_redirect_uri(mobile_redirect_url),
                )
            }
            None => None,
        };

        let mut accepted_audiences = vec![client_id];
        if let Some(ios) = &config.ios_client_id {
            accepted_audiences.push(ClientId::new(ios.expose_secret().to_owned()));
        }

        Ok(Self {
            client,
            mobile_client,
            redirect_uri,
            mobile_redirect_uri: config.mobile_redirect_uri,
            accepted_audiences,
            http,
        })
    }

    pub fn redirect_uri(&self) -> &str {
        &self.redirect_uri
    }

    pub fn mobile_redirect_uri(&self) -> Option<&str> {
        self.mobile_redirect_uri.as_deref()
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
        Self::build_authorization_url(&self.client, state, pkce_challenge, nonce)
    }

    /// Mobile variant: builds the URL against the mobile-redirect client
    /// so Google calls back to the backend's mobile-callback endpoint.
    /// Returns `None` when no `GOOGLE_MOBILE_REDIRECT_URI` is configured.
    pub fn mobile_authorization_url(
        &self,
        state: &str,
        pkce_challenge: PkceCodeChallenge,
        nonce: Nonce,
    ) -> Option<String> {
        self.mobile_client
            .as_ref()
            .map(|c| Self::build_authorization_url(c, state, pkce_challenge, nonce))
    }

    fn build_authorization_url(
        client: &DiscoveredClient,
        state: &str,
        pkce_challenge: PkceCodeChallenge,
        nonce: Nonce,
    ) -> String {
        let state_str = state.to_string();
        let (authorize_url, _, _) = client
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
        self.exchange_code_with(&self.client, code, pkce_verifier, nonce)
            .await
    }

    /// Mobile variant: exchanges against the mobile-bound client so
    /// Google's token endpoint sees the matching `redirect_uri`. Errors
    /// with `MissingEnvVar("GOOGLE_MOBILE_REDIRECT_URI")` if mobile
    /// support isn't configured — surfacing this as a config error
    /// (rather than a server-side panic) lets the caller fail the
    /// request cleanly.
    pub async fn mobile_exchange_code(
        &self,
        code: &str,
        pkce_verifier: String,
        nonce: &Nonce,
    ) -> Result<GoogleUserInfo, OAuthError> {
        let client = self
            .mobile_client
            .as_ref()
            .ok_or(OAuthError::MissingEnvVar("GOOGLE_MOBILE_REDIRECT_URI"))?;
        self.exchange_code_with(client, code, pkce_verifier, nonce)
            .await
    }

    async fn exchange_code_with(
        &self,
        client: &DiscoveredClient,
        code: &str,
        pkce_verifier: String,
        nonce: &Nonce,
    ) -> Result<GoogleUserInfo, OAuthError> {
        let token_response = client
            .exchange_code(AuthorizationCode::new(code.to_string()))
            .map_err(|e| OAuthError::CodeExchange(e.to_string()))?
            .set_pkce_verifier(PkceCodeVerifier::new(pkce_verifier))
            .request_async(&self.http)
            .await
            .map_err(|e| OAuthError::CodeExchange(e.to_string()))?;

        let id_token = token_response
            .id_token()
            .ok_or(OAuthError::MissingField("id_token"))?;

        let verifier = client.id_token_verifier();
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

        let access_token = Some(SecretString::from(
            token_response.access_token().secret().to_string(),
        ));
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

    /// Verify a Google ID token issued directly to a native client
    /// (Android Credential Manager, iOS GoogleSignIn SDK).
    ///
    /// Unlike [`Self::exchange_code`], no token endpoint round-trip
    /// happens — the JWT is verified locally against Google's JWKS
    /// (cached at startup via OIDC discovery). The verifier accepts
    /// either the server `client_id` (audience for Android Credential
    /// Manager and the web flow) or the iOS client ID (audience for
    /// native iOS sign-in), both expressed up-front via
    /// `GOOGLE_CLIENT_ID` and `GOOGLE_CLIENT_ID_IOS`.
    ///
    /// `expected_nonce` is checked when the caller supplied one
    /// (recommended for replay protection); when `None`, any value the
    /// token carries is accepted, which matches the native plugin's
    /// current behaviour of not threading a nonce through the SDK.
    pub fn verify_id_token(
        &self,
        id_token_str: &str,
        expected_nonce: Option<&Nonce>,
    ) -> Result<GoogleUserInfo, OAuthError> {
        let id_token = CoreIdToken::from_str(id_token_str)
            .map_err(|e| OAuthError::TokenVerification(format!("malformed id_token: {e}")))?;

        let mut verifier = self.client.id_token_verifier();
        if self.accepted_audiences.len() > 1 {
            // Permit any audience in the configured allow-list besides
            // the primary `client_id`. Returning `false` here causes the
            // verifier to reject the token, so this closure is the
            // single chokepoint for what audiences we accept.
            let extras: Vec<String> = self.accepted_audiences[1..]
                .iter()
                .map(|c| c.as_str().to_string())
                .collect();
            verifier = verifier.set_other_audience_verifier_fn(move |aud| {
                extras.iter().any(|a| a == aud.as_str())
            });
        }

        let claims: &CoreIdTokenClaims = if let Some(nonce) = expected_nonce {
            id_token.claims(&verifier, nonce)
        } else {
            id_token.claims(&verifier, |_: Option<&Nonce>| Ok(()))
        }
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

        // Native ID-token flows don't yield a Google access/refresh
        // token — the `oauth_credentials` row exists as a pure
        // provider-link record. Modelled as `None` rather than an
        // empty `SecretString` so callers can't accidentally encrypt
        // and persist a zero-length token.
        Ok(GoogleUserInfo {
            id,
            email,
            verified_email,
            display_name,
            access_token: None,
            refresh_token: None,
            expires_in: None,
            scope: "openid email profile".to_string(),
        })
    }
}

#[derive(Debug)]
pub struct GoogleUserInfo {
    pub id: String,
    pub email: String,
    pub verified_email: bool,
    pub display_name: Option<String>,
    /// `None` for the native-ID-token flow (no token endpoint
    /// round-trip happened); `Some` for the redirect/code-exchange
    /// flow.
    pub access_token: Option<SecretString>,
    pub refresh_token: Option<SecretString>,
    pub expires_in: Option<std::time::Duration>,
    pub scope: String,
}
