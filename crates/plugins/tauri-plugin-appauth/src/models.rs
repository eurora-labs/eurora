use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// URL-encoded extra parameters that ride along on authorize / refresh /
/// end-session requests. Both the key and the value end up in a query string
/// or `application/x-www-form-urlencoded` body, so values are plain strings.
pub type QueryParams = HashMap<String, String>;

/// Free-form JSON fields that appear inside discovery / registration / token
/// JSON payloads. Values are arbitrary `serde_json::Value`s because the
/// authorization server may return numbers, arrays, objects, or booleans for
/// vendor-specific extensions.
pub type ExtensionFields = HashMap<String, serde_json::Value>;

/// Where to source the authorization server's endpoints.
///
/// `Discovery` hits `<issuer>/.well-known/openid-configuration` (RFC 8414 /
/// OIDC); `Explicit` skips discovery for providers that don't publish a
/// document or for tests that want full control.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ConfigSource {
    /// Resolve endpoints by fetching the issuer's discovery document.
    #[serde(rename_all = "camelCase")]
    Discovery {
        /// Issuer URL. The plugin appends `/.well-known/openid-configuration`
        /// (or the RFC 8414 equivalent) before fetching.
        issuer: String,
    },
    /// Skip discovery and use the supplied endpoints verbatim.
    #[serde(rename_all = "camelCase")]
    Explicit {
        /// Authorization endpoint URL (RFC 6749 §3.1).
        authorization_endpoint: String,
        /// Token endpoint URL (RFC 6749 §3.2).
        token_endpoint: String,
        /// RP-initiated logout endpoint (RFC 8665), if the issuer publishes one.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        end_session_endpoint: Option<String>,
        /// Dynamic Client Registration endpoint (RFC 7591), if available.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        registration_endpoint: Option<String>,
    },
}

/// Resolved set of issuer endpoints, as returned by [`crate::AppAuth::discover`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceConfiguration {
    /// Authorization endpoint (RFC 6749 §3.1).
    pub authorization_endpoint: String,
    /// Token endpoint (RFC 6749 §3.2).
    pub token_endpoint: String,
    /// RP-initiated logout endpoint (RFC 8665), if the issuer publishes one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_session_endpoint: Option<String>,
    /// Dynamic Client Registration endpoint (RFC 7591), if available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registration_endpoint: Option<String>,
    /// Issuer identifier as advertised in the discovery document.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub issuer: Option<String>,
    /// Vendor-specific or otherwise non-standard fields the discovery document
    /// carried alongside the well-known endpoints.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub additional_parameters: ExtensionFields,
}

/// Input to [`crate::AppAuth::discover`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoverRequest {
    /// Issuer URL whose discovery document should be fetched.
    pub issuer: String,
}

/// OIDC `prompt` parameter values (RFC 6749 / OIDC Core §3.1.2.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Prompt {
    /// Force the user to (re-)authenticate.
    Login,
    /// Force the consent screen even if the scopes were previously granted.
    Consent,
    /// Show an account chooser.
    SelectAccount,
    /// Suppress all interactive prompts; succeeds only if the request can be
    /// satisfied silently. Wire value is `"none"`. Named `NoInteraction`
    /// in Rust to avoid colliding visually with [`Option::None`].
    #[serde(rename = "none")]
    NoInteraction,
}

/// Input to [`crate::AppAuth::authorize`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizeRequest {
    /// How endpoints are resolved (discovery vs. explicit).
    pub config: ConfigSource,
    /// `client_id` registered with the issuer.
    pub client_id: String,
    /// Custom-scheme URI (e.g. `com.example.app:/oauth/callback`) or HTTPS
    /// app-link. `AppAuth` validates that the redirect handler is registered
    /// with the OS before opening the browser.
    pub redirect_uri: String,
    /// Requested OAuth scopes (e.g. `["openid", "email"]`).
    #[serde(default)]
    pub scopes: Vec<String>,
    /// Extra parameters appended to the authorization request URL.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub additional_parameters: QueryParams,
    /// Optional OIDC `prompt` hint.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<Prompt>,
    /// Optional OIDC `login_hint`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub login_hint: Option<String>,
    /// Optional OIDC `ui_locales` (BCP 47 language tags, ordered by preference).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ui_locales: Option<Vec<String>>,
    /// iOS-only hint forwarded to `ASWebAuthenticationSession`. Ignored on
    /// Android (Custom Tabs always shares cookies with the user's default
    /// browser).
    #[serde(default = "default_true")]
    pub prefers_ephemeral_session: bool,
    /// Whether `AppAuth` should generate and validate an OIDC `nonce`. Defaults
    /// to `true` — set to `false` to opt out for non-OIDC providers that
    /// reject the parameter. OIDC requires nonce for the `code` flow and
    /// AppAuth auto-generates one, so the default matches what the mobile
    /// platforms can correctly express.
    #[serde(default = "default_true")]
    pub use_nonce: bool,
}

/// Token-bearing state returned by [`crate::AppAuth::authorize`] and
/// [`crate::AppAuth::refresh`].
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AuthState {
    /// Bearer access token.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub access_token: Option<String>,
    /// Unix seconds at which `access_token` expires.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub access_token_expires_at: Option<i64>,
    /// OIDC ID token JWT (when `openid` scope was requested).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id_token: Option<String>,
    /// Refresh token, when the issuer returned one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    /// Space-delimited scopes the access token was actually granted (RFC 6749 §3.3).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    /// `token_type` reported by the token endpoint (almost always `Bearer`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token_type: Option<String>,
    /// Surfaced for backend-mediated flows that exchange the code themselves.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authorization_code: Option<String>,
    /// Vendor-specific extension fields returned by the token endpoint.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub additional_parameters: ExtensionFields,
}

/// Input to [`crate::AppAuth::authorize_browser_only`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserOnlyRequest {
    /// Fully-built authorization URL. The plugin opens the browser at this URL
    /// and waits for the OS to intercept `redirect_uri`.
    pub auth_url: String,
    /// Redirect URI the OS should intercept. Must match what the authorization
    /// server is configured to redirect to.
    pub redirect_uri: String,
    /// iOS-only hint forwarded to `ASWebAuthenticationSession`. Ignored on
    /// Android (Custom Tabs always shares cookies with the user's default
    /// browser).
    #[serde(default = "default_true")]
    pub prefers_ephemeral_session: bool,
}

/// Output of [`crate::AppAuth::authorize_browser_only`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserOnlyResponse {
    /// Full callback URL the system intercepted, with all query parameters
    /// from the authorization server intact.
    pub url: String,
}

/// Input to [`crate::AppAuth::refresh`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshRequest {
    /// How endpoints are resolved (discovery vs. explicit).
    pub config: ConfigSource,
    /// `client_id` registered with the issuer.
    pub client_id: String,
    /// Refresh token previously obtained from the token endpoint.
    pub refresh_token: String,
    /// Optionally narrow the requested scopes (RFC 6749 §6).
    #[serde(default)]
    pub scopes: Vec<String>,
    /// Extra parameters appended to the token request body.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub additional_parameters: QueryParams,
}

/// Input to [`crate::AppAuth::register`] (RFC 7591).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterRequest {
    /// How endpoints are resolved (discovery vs. explicit).
    pub config: ConfigSource,
    /// One or more redirect URIs to register for this client.
    pub redirect_uris: Vec<String>,
    /// Human-readable client name shown on consent screens.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_name: Option<String>,
    /// `response_types` the client intends to use (e.g. `["code"]`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub response_types: Vec<String>,
    /// `grant_types` the client intends to use (e.g. `["authorization_code", "refresh_token"]`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub grant_types: Vec<String>,
    /// OIDC `subject_types` the client supports (e.g. `["public"]`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub subject_types: Vec<String>,
    /// Token-endpoint authentication method (e.g. `none`, `client_secret_basic`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token_endpoint_auth_method: Option<String>,
    /// Extra metadata fields included in the registration request body.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub additional_parameters: ExtensionFields,
}

/// Output of [`crate::AppAuth::register`] (RFC 7591).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistrationResponse {
    /// `client_id` issued by the authorization server.
    pub client_id: String,
    /// Unix seconds at which the `client_id` was issued.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_id_issued_at: Option<i64>,
    /// `client_secret`, when the issuer issues confidential credentials.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_secret: Option<String>,
    /// Unix seconds at which `client_secret` expires (`0` means "never").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_secret_expires_at: Option<i64>,
    /// Token used to read or update the registration via the registration
    /// management endpoint.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registration_access_token: Option<String>,
    /// URL of the registration management endpoint for this client.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registration_client_uri: Option<String>,
    /// Token-endpoint authentication method the issuer assigned.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token_endpoint_auth_method: Option<String>,
    /// Vendor-specific extension fields returned by the registration endpoint.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub additional_parameters: ExtensionFields,
}

/// Input to [`crate::AppAuth::end_session`] (RFC 8665).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EndSessionRequest {
    /// How endpoints are resolved (discovery vs. explicit).
    pub config: ConfigSource,
    /// `id_token` from the session being terminated, used to bind the logout
    /// to a specific authenticated user. Optional per RFC 8665 / OIDC
    /// RP-Initiated Logout: the parameter is RECOMMENDED, not REQUIRED, and
    /// some IdPs accept end-session without it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id_token_hint: Option<String>,
    /// URI the issuer should redirect to after logout completes.
    pub post_logout_redirect_uri: String,
    /// Opaque value echoed back via the post-logout redirect, for CSRF
    /// protection.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    /// Extra parameters appended to the end-session request URL.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub additional_parameters: QueryParams,
    /// iOS-only hint forwarded to `ASWebAuthenticationSession`. Ignored on
    /// Android (Custom Tabs always shares cookies with the user's default
    /// browser).
    #[serde(default = "default_true")]
    pub prefers_ephemeral_session: bool,
}

/// Output of [`crate::AppAuth::end_session`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EndSessionResponse {
    /// Full post-logout redirect URL the system intercepted.
    pub url: String,
    /// `state` value the issuer echoed back, when one was supplied on the
    /// request.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
}

fn default_true() -> bool {
    true
}
