use serde::{Deserialize, Serialize};

#[cfg(feature = "specta")]
use specta::Type;
#[cfg(feature = "specta")]
use specta_typescript::BigInt;

use crate::{Provider, Role};

/// Bearer-mode session response, used by non-browser clients (desktop /
/// mobile) that send `Authorization: Bearer …` and need the tokens in
/// the JSON body.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    /// Lifetime of `access_token` in seconds.
    #[cfg_attr(feature = "specta", specta(type = BigInt))]
    pub expires_in: i64,
}

/// Public-facing user profile. Mirrors the subset of [`crate::Claims`]
/// the SPA renders; never carries the JWT itself.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct UserInfo {
    pub id: String,
    pub email: String,
    #[serde(default)]
    pub display_name: Option<String>,
    pub email_verified: bool,
    pub role: Role,
}

/// Cookie-mode session response. Tokens are delivered via `Set-Cookie`
/// (HttpOnly access + refresh); the JSON body carries only the user
/// profile so a compromised renderer cannot read the access token off
/// the response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct UserResponse {
    pub user: UserInfo,
}

/// Wire-level union returned by every endpoint that mints a session.
/// `untagged` so existing desktop clients keep deserialising into
/// [`TokenResponse`] while the SPA sees the cookie-only [`UserResponse`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(Type))]
#[serde(untagged)]
pub enum AuthSuccessResponse {
    Bearer(TokenResponse),
    Cookie(UserResponse),
}

/// Response body for `POST /auth/oauth/url`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct ThirdPartyAuthUrlResponse {
    pub url: String,
}

/// Outcome of an email lookup performed via `POST /auth/email/check`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "specta", derive(Type))]
#[serde(rename_all = "snake_case")]
pub enum CheckEmailStatus {
    /// No account is registered with that email.
    NotFound,
    /// Account exists and authenticates with a password.
    Password,
    /// Account exists and authenticates via a third-party provider.
    Oauth,
}

/// Response body for `POST /auth/email/check`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct CheckEmailResponse {
    pub status: CheckEmailStatus,
    /// Populated only when `status == Oauth`.
    #[serde(default)]
    pub provider: Option<Provider>,
}

/// JSON error body returned by the auth service on non-2xx responses.
///
/// Mirrors the shape used by `be-update-service` and `be-activity-service`
/// so the desktop client can decode failures uniformly across HTTP services.
/// The `error` field is the stable machine-readable kind (see
/// [`crate::error_kinds`]); `message` is human-readable; `details` is an
/// optional free-form string for additional context.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct AuthErrorResponse {
    pub error: String,
    pub message: String,
    #[serde(default)]
    pub details: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_email_status_serializes_snake_case() {
        let value = serde_json::to_value(CheckEmailStatus::NotFound).unwrap();
        assert_eq!(value, serde_json::Value::String("not_found".into()));
        let value = serde_json::to_value(CheckEmailStatus::Oauth).unwrap();
        assert_eq!(value, serde_json::Value::String("oauth".into()));
    }

    #[test]
    fn check_email_response_emits_null_provider() {
        let resp = CheckEmailResponse {
            status: CheckEmailStatus::Password,
            provider: None,
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"provider\":null"));
        let back: CheckEmailResponse = serde_json::from_str(&json).unwrap();
        assert!(back.provider.is_none());
    }

    #[test]
    fn check_email_response_decodes_with_missing_provider() {
        // Forward-compat: older servers that omit the field still parse.
        let back: CheckEmailResponse = serde_json::from_str(r#"{"status":"password"}"#).unwrap();
        assert!(back.provider.is_none());
    }

    #[test]
    fn auth_error_response_round_trip() {
        let body = AuthErrorResponse {
            error: "oauth_email_conflict".into(),
            message: "An account with this email already exists under a different sign-in method"
                .into(),
            details: None,
        };
        let json = serde_json::to_string(&body).unwrap();
        assert!(json.contains("\"details\":null"));
        let back: AuthErrorResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(back.error, "oauth_email_conflict");
        assert!(back.details.is_none());
    }

    #[test]
    fn auth_success_bearer_decodes_into_token_response() {
        // Wire compatibility: a desktop client deserialising the
        // bearer-mode JSON directly into `TokenResponse` must succeed,
        // because the untagged `AuthSuccessResponse::Bearer` variant
        // serialises with no tag wrapper.
        let payload = AuthSuccessResponse::Bearer(TokenResponse {
            access_token: "access".into(),
            refresh_token: "refresh".into(),
            expires_in: 3600,
        });
        let json = serde_json::to_string(&payload).unwrap();
        let decoded: TokenResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.access_token, "access");
        assert_eq!(decoded.expires_in, 3600);
    }

    #[test]
    fn auth_success_cookie_round_trips() {
        let payload = AuthSuccessResponse::Cookie(UserResponse {
            user: UserInfo {
                id: "00000000-0000-0000-0000-000000000000".into(),
                email: "u@example.com".into(),
                display_name: Some("U".into()),
                email_verified: true,
                role: crate::Role::Free,
            },
        });
        let json = serde_json::to_string(&payload).unwrap();
        // Cookie variant must not echo any token fields.
        assert!(!json.contains("access_token"));
        assert!(!json.contains("refresh_token"));
        assert!(json.contains("\"user\""));
        let decoded: AuthSuccessResponse = serde_json::from_str(&json).unwrap();
        match decoded {
            AuthSuccessResponse::Cookie(resp) => {
                assert_eq!(resp.user.email, "u@example.com");
            }
            AuthSuccessResponse::Bearer(_) => panic!("expected cookie variant"),
        }
    }
}
