//! Serde round-trip coverage for every public request/response type.
//!
//! Each case builds a fully-populated value, serializes it, deserializes the
//! result, and serializes again — the two JSON representations must match.
//! This catches accidental field renames, removed fields, and default
//! mismatches without having to derive `PartialEq` on the DTOs.

use serde::{Serialize, de::DeserializeOwned};
use serde_json::json;
use tauri_plugin_appauth::{
    AuthEvent, AuthState, AuthorizeRequest, BrowserOnlyRequest, BrowserOnlyResponse, ConfigSource,
    DiscoverRequest, EndSessionRequest, EndSessionResponse, Prompt, RefreshRequest,
    RegisterRequest, RegistrationResponse, ServiceConfiguration,
};

fn round_trip<T: Serialize + DeserializeOwned>(value: T) {
    let json = serde_json::to_value(&value).expect("serialize");
    let parsed: T = serde_json::from_value(json.clone()).expect("deserialize");
    let again = serde_json::to_value(&parsed).expect("re-serialize");
    assert_eq!(json, again);
}

#[test]
fn config_source_discovery() {
    round_trip(ConfigSource::Discovery {
        issuer: "https://issuer.example.com".into(),
    });
}

#[test]
fn config_source_explicit() {
    round_trip(ConfigSource::Explicit {
        authorization_endpoint: "https://issuer.example.com/auth".into(),
        token_endpoint: "https://issuer.example.com/token".into(),
        end_session_endpoint: Some("https://issuer.example.com/logout".into()),
        registration_endpoint: Some("https://issuer.example.com/register".into()),
    });
}

#[test]
fn prompt_each_variant() {
    for p in [
        Prompt::Login,
        Prompt::Consent,
        Prompt::SelectAccount,
        Prompt::NoInteraction,
    ] {
        round_trip(p);
    }
}

#[test]
fn prompt_no_interaction_wire_format() {
    assert_eq!(
        serde_json::to_value(Prompt::NoInteraction).unwrap(),
        json!("none"),
    );
}

#[test]
fn discover_request() {
    round_trip(DiscoverRequest {
        issuer: "https://issuer.example.com".into(),
    });
}

#[test]
fn service_configuration() {
    round_trip(ServiceConfiguration {
        authorization_endpoint: "https://issuer.example.com/auth".into(),
        token_endpoint: "https://issuer.example.com/token".into(),
        end_session_endpoint: Some("https://issuer.example.com/logout".into()),
        registration_endpoint: Some("https://issuer.example.com/register".into()),
        issuer: Some("https://issuer.example.com".into()),
        additional_parameters: [("vendor_flag".into(), json!(true))].into_iter().collect(),
    });
}

#[test]
fn authorize_request_full() {
    round_trip(AuthorizeRequest {
        config: ConfigSource::Discovery {
            issuer: "https://issuer.example.com".into(),
        },
        client_id: "client-123".into(),
        redirect_uri: "com.example.app:/oauth/callback".into(),
        scopes: vec!["openid".into(), "email".into()],
        additional_parameters: [("audience".into(), "api://default".into())]
            .into_iter()
            .collect(),
        prompt: Some(Prompt::Login),
        login_hint: Some("user@example.com".into()),
        ui_locales: Some(vec!["en-US".into(), "fr".into()]),
        prefers_ephemeral_session: false,
        use_nonce: true,
    });
}

#[test]
fn authorize_request_use_nonce_defaults_to_true_when_missing() {
    let payload = json!({
        "config": { "kind": "discovery", "issuer": "https://issuer.example.com" },
        "clientId": "client-123",
        "redirectUri": "com.example.app:/oauth/callback",
    });
    let req: AuthorizeRequest =
        serde_json::from_value(payload).expect("deserialize with omitted useNonce");
    assert!(
        req.use_nonce,
        "useNonce must default to true when the field is omitted",
    );
}

#[test]
fn authorize_request_use_nonce_round_trip_false() {
    round_trip(AuthorizeRequest {
        config: ConfigSource::Discovery {
            issuer: "https://issuer.example.com".into(),
        },
        client_id: "client-123".into(),
        redirect_uri: "com.example.app:/oauth/callback".into(),
        scopes: vec![],
        additional_parameters: Default::default(),
        prompt: None,
        login_hint: None,
        ui_locales: None,
        prefers_ephemeral_session: true,
        use_nonce: false,
    });
}

#[test]
fn auth_state_full() {
    round_trip(AuthState {
        access_token: Some("at".into()),
        access_token_expires_at: Some(1_700_000_000),
        id_token: Some("it".into()),
        refresh_token: Some("rt".into()),
        scope: Some("openid email".into()),
        token_type: Some("Bearer".into()),
        authorization_code: Some("code-123".into()),
        additional_parameters: [("session_state".into(), json!("xyz"))]
            .into_iter()
            .collect(),
    });
}

#[test]
fn auth_state_default() {
    round_trip(AuthState::default());
}

#[test]
fn browser_only_request() {
    round_trip(BrowserOnlyRequest {
        auth_url: "https://issuer.example.com/auth?...".into(),
        redirect_uri: "com.example.app:/oauth/callback".into(),
        prefers_ephemeral_session: false,
    });
}

#[test]
fn browser_only_response() {
    round_trip(BrowserOnlyResponse {
        url: "com.example.app:/oauth/callback?code=abc&state=xyz".into(),
    });
}

#[test]
fn refresh_request() {
    round_trip(RefreshRequest {
        config: ConfigSource::Explicit {
            authorization_endpoint: "https://issuer.example.com/auth".into(),
            token_endpoint: "https://issuer.example.com/token".into(),
            end_session_endpoint: None,
            registration_endpoint: None,
        },
        client_id: "client-123".into(),
        refresh_token: "rt".into(),
        scopes: vec!["openid".into()],
        additional_parameters: [("audience".into(), "api://default".into())]
            .into_iter()
            .collect(),
    });
}

#[test]
fn register_request() {
    round_trip(RegisterRequest {
        config: ConfigSource::Discovery {
            issuer: "https://issuer.example.com".into(),
        },
        redirect_uris: vec!["com.example.app:/oauth/callback".into()],
        client_name: Some("Example App".into()),
        response_types: vec!["code".into()],
        grant_types: vec!["authorization_code".into(), "refresh_token".into()],
        subject_types: vec!["public".into()],
        token_endpoint_auth_method: Some("none".into()),
        additional_parameters: [("software_id".into(), json!("example-mobile"))]
            .into_iter()
            .collect(),
    });
}

#[test]
fn registration_response() {
    round_trip(RegistrationResponse {
        client_id: "client-123".into(),
        client_id_issued_at: Some(1_700_000_000),
        client_secret: Some("secret".into()),
        client_secret_expires_at: Some(0),
        registration_access_token: Some("rat".into()),
        registration_client_uri: Some("https://issuer.example.com/register/client-123".into()),
        token_endpoint_auth_method: Some("none".into()),
        additional_parameters: [("software_id".into(), json!("example-mobile"))]
            .into_iter()
            .collect(),
    });
}

#[test]
fn end_session_request() {
    round_trip(EndSessionRequest {
        config: ConfigSource::Discovery {
            issuer: "https://issuer.example.com".into(),
        },
        id_token_hint: Some("it".into()),
        post_logout_redirect_uri: "com.example.app:/post-logout".into(),
        state: Some("opaque".into()),
        additional_parameters: [("ui_locales".into(), "en".into())].into_iter().collect(),
        prefers_ephemeral_session: true,
    });
}

#[test]
fn end_session_request_omits_id_token_hint() {
    let value = EndSessionRequest {
        config: ConfigSource::Discovery {
            issuer: "https://issuer.example.com".into(),
        },
        id_token_hint: None,
        post_logout_redirect_uri: "com.example.app:/post-logout".into(),
        state: None,
        additional_parameters: Default::default(),
        prefers_ephemeral_session: true,
    };
    let json = serde_json::to_value(&value).expect("serialize");
    assert!(
        json.get("idTokenHint").is_none(),
        "absent idTokenHint must be omitted from the wire payload, got: {json}",
    );
    round_trip(value);
}

#[test]
fn end_session_response() {
    round_trip(EndSessionResponse {
        url: "com.example.app:/post-logout?state=opaque".into(),
        state: Some("opaque".into()),
    });
}

#[test]
fn auth_event_each_variant() {
    for ev in [
        AuthEvent::BrowserOpened,
        AuthEvent::RedirectIntercepted,
        AuthEvent::TokenExchangeStarted,
        AuthEvent::TokenExchangeCompleted,
    ] {
        round_trip(ev);
    }
}
