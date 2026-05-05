use serde::{Deserialize, Serialize};

#[cfg(feature = "specta")]
use specta::Type;

use crate::Provider;

/// Standard JSON response for endpoints that mint a session.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    /// Lifetime of `access_token` in seconds.
    pub expires_in: i64,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
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
    fn check_email_response_omits_none_provider() {
        let resp = CheckEmailResponse {
            status: CheckEmailStatus::Password,
            provider: None,
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(!json.contains("provider"));
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
        assert!(!json.contains("details"));
        let back: AuthErrorResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(back.error, "oauth_email_conflict");
    }
}
