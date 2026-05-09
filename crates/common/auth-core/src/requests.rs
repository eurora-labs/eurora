use serde::{Deserialize, Serialize};

#[cfg(feature = "specta")]
use specta::Type;

use crate::Provider;

/// Request body for `POST /auth/login`.
///
/// Encoded as a discriminated union: the `kind` tag selects the
/// credential variant, mirroring the gRPC `oneof` it replaces while
/// staying friendly to JSON consumers and TypeScript codegen.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(Type))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LoginRequest {
    EmailPassword {
        login: String,
        password: String,
    },
    ThirdParty {
        provider: Provider,
        code: String,
        state: String,
        #[serde(default)]
        login_token: Option<String>,
    },
}

/// Request body for `POST /auth/register`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
    #[serde(default)]
    pub display_name: Option<String>,
}

/// Request body for `POST /auth/oauth/url`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct ThirdPartyAuthUrlRequest {
    pub provider: Provider,
}

/// Request body for `POST /auth/login-token/exchange`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct LoginByLoginTokenRequest {
    pub token: String,
}

/// Request body for `POST /auth/login-token/associate`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct AssociateLoginTokenRequest {
    pub code_challenge: String,
}

/// Request body for `POST /auth/email/check`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct CheckEmailRequest {
    pub email: String,
}

/// Request body for `POST /auth/email/verify`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct VerifyEmailRequest {
    pub token: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn login_email_password_round_trip() {
        let req = LoginRequest::EmailPassword {
            login: "user@example.com".into(),
            password: "secret".into(),
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["kind"], "email_password");
        let back: LoginRequest = serde_json::from_value(json).unwrap();
        match back {
            LoginRequest::EmailPassword { login, password } => {
                assert_eq!(login, "user@example.com");
                assert_eq!(password, "secret");
            }
            _ => panic!("expected EmailPassword variant"),
        }
    }

    #[test]
    fn login_third_party_round_trip_with_optional_token() {
        let req = LoginRequest::ThirdParty {
            provider: Provider::Google,
            code: "abc".into(),
            state: "xyz".into(),
            login_token: Some("lt".into()),
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["kind"], "third_party");
        assert_eq!(json["provider"], "google");
        assert_eq!(json["login_token"], "lt");
        let back: LoginRequest = serde_json::from_value(json).unwrap();
        matches!(back, LoginRequest::ThirdParty { .. });
    }

    #[test]
    fn login_third_party_emits_null_login_token() {
        let req = LoginRequest::ThirdParty {
            provider: Provider::Github,
            code: "abc".into(),
            state: "xyz".into(),
            login_token: None,
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["login_token"], serde_json::Value::Null);
        let back: LoginRequest = serde_json::from_value(json).unwrap();
        match back {
            LoginRequest::ThirdParty { login_token, .. } => assert!(login_token.is_none()),
            _ => panic!("expected ThirdParty variant"),
        }
    }

    #[test]
    fn register_request_emits_null_display_name() {
        let req = RegisterRequest {
            email: "u@e.com".into(),
            password: "p".into(),
            display_name: None,
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"display_name\":null"));
        let back: RegisterRequest = serde_json::from_str(&json).unwrap();
        assert!(back.display_name.is_none());
    }

    #[test]
    fn register_request_decodes_with_missing_display_name() {
        // Forward-compat: an older client that omits the field still parses.
        let back: RegisterRequest =
            serde_json::from_str(r#"{"email":"u@e.com","password":"p"}"#).unwrap();
        assert!(back.display_name.is_none());
    }
}
