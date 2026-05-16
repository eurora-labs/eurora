use serde::{Deserialize, Serialize};

#[cfg(feature = "specta")]
use specta::Type;

use crate::Provider;

/// Request body for `POST /auth/login`.
///
/// Encoded as a discriminated union: the `kind` tag selects the
/// credential variant, mirroring the gRPC `oneof` it replaces while
/// staying friendly to JSON consumers and TypeScript codegen.
///
/// The desktop-pairing `login_token` is **not** part of this body — it
/// is captured at OAuth URL issue time on
/// [`ThirdPartyAuthUrlRequest`] and read off the `oauth_state` row
/// during code exchange. Apple Sign In's form-post flow doesn't surface
/// `code`/`state` to the SPA, so an at-callback mechanism is structurally
/// impossible for that provider — moving every provider to the
/// at-issue-time path keeps the contract uniform.
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
///
/// The optional `login_token` is the desktop client's PKCE challenge —
/// when present, the backend stamps it onto the `oauth_state` row so the
/// eventual callback can pair the device without the SPA round-tripping
/// the value through the login request body. Mirrors the convention the
/// mobile flow already uses via `MobileThirdPartyAuthUrlRequest`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct ThirdPartyAuthUrlRequest {
    pub provider: Provider,
    #[serde(default)]
    pub login_token: Option<String>,
}

/// Request body for `POST /auth/oauth/mobile/url`.
///
/// The mobile app generates a PKCE pair locally and submits the
/// challenge here; the backend stamps that challenge as the OAuth
/// `state` (so it round-trips through Google and identifies the device
/// in the callback) and as the eventual `login_token` row that the
/// device redeems via `/auth/login-token/exchange`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct MobileThirdPartyAuthUrlRequest {
    pub provider: Provider,
    pub code_challenge: String,
    pub code_challenge_method: String,
}

/// Request body for `POST /auth/oauth/google/id-token`.
///
/// Used by mobile after a native Google Sign-In flow: the client sends
/// the ID token issued directly by Google's iOS / Android SDK and the
/// backend verifies it against Google's JWKS without an authorization
/// code round-trip. The optional `nonce` is checked against the JWT's
/// nonce claim when supplied.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct GoogleIdTokenLoginRequest {
    pub id_token: String,
    #[serde(default)]
    pub nonce: Option<String>,
}

/// Request body for `POST /auth/oauth/apple/id-token`.
///
/// Used by mobile after a native iOS Sign in with Apple flow
/// (`ASAuthorizationController`): the client hands us the ID token
/// Apple issued directly to the device. The backend verifies the
/// signature against Apple's JWKS and the nonce against
/// `base64url(sha256(raw_nonce))` (Apple echoes whatever the client
/// placed in `request.nonce`, and the iOS plugin hashes before sending).
///
/// `raw_nonce` is **required**, not optional like
/// [`GoogleIdTokenLoginRequest::nonce`]: dropping replay protection on
/// this endpoint would let a captured `id_token` mint sessions
/// indefinitely. The native plugin always supplies one.
///
/// `user` carries first/last name from `ASAuthorizationAppleIDCredential.fullName`
/// — Apple only ships this on the very first sign-in for a given user,
/// and the server-side guard inside `complete_oauth_login` ensures the
/// override is only applied when no existing user matches `(Apple, sub)`.
/// A malicious client can therefore not replay a stolen `id_token` with
/// a fabricated `user` field to overwrite an established display name.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct AppleIdTokenLoginRequest {
    pub id_token: String,
    pub raw_nonce: String,
    #[serde(default)]
    pub user: Option<AppleNativeUser>,
}

/// First/last name from `ASAuthorizationAppleIDCredential.fullName`.
/// Both fields are optional because Apple's `PersonNameComponents`
/// permits either side to be nil and the user can edit the name shown
/// in the system sheet before consent.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct AppleNativeUser {
    #[serde(default)]
    pub first_name: Option<String>,
    #[serde(default)]
    pub last_name: Option<String>,
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
    fn login_third_party_round_trip() {
        let req = LoginRequest::ThirdParty {
            provider: Provider::Google,
            code: "abc".into(),
            state: "xyz".into(),
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["kind"], "third_party");
        assert_eq!(json["provider"], "google");
        // `login_token` is no longer part of this body — capture happens
        // at URL-issue time via `ThirdPartyAuthUrlRequest`.
        assert!(json.get("login_token").is_none());
        let back: LoginRequest = serde_json::from_value(json).unwrap();
        match back {
            LoginRequest::ThirdParty {
                provider,
                code,
                state,
            } => {
                assert_eq!(provider.as_str(), "google");
                assert_eq!(code, "abc");
                assert_eq!(state, "xyz");
            }
            _ => panic!("expected ThirdParty variant"),
        }
    }

    #[test]
    fn login_third_party_rejects_unknown_fields_gracefully() {
        // A legacy client that still emits `login_token` in this body
        // round-trips cleanly (the field is silently dropped by serde's
        // default behaviour) — the wire-compat we keep is permissive
        // decoding, not preservation of the obsolete field.
        let raw = r#"{"kind":"third_party","provider":"apple","code":"c","state":"s","login_token":"old"}"#;
        let back: LoginRequest = serde_json::from_str(raw).unwrap();
        matches!(back, LoginRequest::ThirdParty { .. });
    }

    #[test]
    fn third_party_auth_url_request_round_trip() {
        let req = ThirdPartyAuthUrlRequest {
            provider: Provider::Apple,
            login_token: Some("lt".into()),
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["provider"], "apple");
        assert_eq!(json["login_token"], "lt");
        let back: ThirdPartyAuthUrlRequest = serde_json::from_value(json).unwrap();
        assert_eq!(back.provider.as_str(), "apple");
        assert_eq!(back.login_token.as_deref(), Some("lt"));
    }

    #[test]
    fn third_party_auth_url_request_decodes_without_login_token() {
        // Forward-compat: older clients omit the field entirely.
        let back: ThirdPartyAuthUrlRequest =
            serde_json::from_str(r#"{"provider":"github"}"#).unwrap();
        assert!(back.login_token.is_none());
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

    #[test]
    fn mobile_third_party_auth_url_round_trip() {
        let req = MobileThirdPartyAuthUrlRequest {
            provider: Provider::Google,
            code_challenge: "Y7-_aAbCdEfGhIjKlMnOpQrStUvWxYz0123456789AB".into(),
            code_challenge_method: "S256".into(),
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["provider"], "google");
        assert_eq!(json["code_challenge_method"], "S256");
        let back: MobileThirdPartyAuthUrlRequest = serde_json::from_value(json).unwrap();
        assert_eq!(back.code_challenge.len(), 43);
    }

    #[test]
    fn google_id_token_login_request_decodes_without_nonce() {
        // Forward-compat: clients that don't thread a nonce still parse.
        let back: GoogleIdTokenLoginRequest =
            serde_json::from_str(r#"{"id_token":"eyJ..."}"#).unwrap();
        assert!(back.nonce.is_none());
        assert_eq!(back.id_token, "eyJ...");
    }

    #[test]
    fn apple_id_token_login_request_round_trip() {
        let req = AppleIdTokenLoginRequest {
            id_token: "eyJ...".into(),
            raw_nonce: "n0nce".into(),
            user: Some(AppleNativeUser {
                first_name: Some("Ada".into()),
                last_name: Some("Lovelace".into()),
            }),
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["id_token"], "eyJ...");
        assert_eq!(json["raw_nonce"], "n0nce");
        assert_eq!(json["user"]["first_name"], "Ada");
        let back: AppleIdTokenLoginRequest = serde_json::from_value(json).unwrap();
        assert_eq!(back.raw_nonce, "n0nce");
        let user = back.user.unwrap();
        assert_eq!(user.first_name.as_deref(), Some("Ada"));
        assert_eq!(user.last_name.as_deref(), Some("Lovelace"));
    }

    #[test]
    fn apple_id_token_login_request_decodes_without_user() {
        // Apple only sends `user` on first sign-in; subsequent calls
        // must round-trip without it.
        let raw = r#"{"id_token":"eyJ...","raw_nonce":"n"}"#;
        let back: AppleIdTokenLoginRequest = serde_json::from_str(raw).unwrap();
        assert!(back.user.is_none());
        assert_eq!(back.raw_nonce, "n");
    }

    #[test]
    fn apple_id_token_login_request_rejects_missing_raw_nonce() {
        // `raw_nonce` is required (replay defence); a body without it
        // must fail to deserialise rather than silently default to "".
        let raw = r#"{"id_token":"eyJ..."}"#;
        let err = serde_json::from_str::<AppleIdTokenLoginRequest>(raw).unwrap_err();
        assert!(
            err.to_string().contains("raw_nonce"),
            "error should name missing field: {err}",
        );
    }

    #[test]
    fn apple_native_user_decodes_partial_names() {
        // Apple's `PersonNameComponents` may have either side nil; the
        // user can also edit the name before consent, leaving one half
        // blank. Both shapes must decode without erroring.
        let only_first = r#"{"first_name":"Ada"}"#;
        let parsed: AppleNativeUser = serde_json::from_str(only_first).unwrap();
        assert_eq!(parsed.first_name.as_deref(), Some("Ada"));
        assert!(parsed.last_name.is_none());

        let neither = r#"{}"#;
        let parsed: AppleNativeUser = serde_json::from_str(neither).unwrap();
        assert!(parsed.first_name.is_none() && parsed.last_name.is_none());
    }
}
