//! JSON shape contracts for [`tauri_plugin_appauth::Error`].
//!
//! `Error` is `Serialize`-only — the JS layer parses the JSON we emit. These
//! tests pin the wire shape: code, message, and optional `oauth_error` /
//! `oauth_error_description` fields. We compare against `serde_json::Value`
//! literals so a regression flips the diff, not a hand-rolled snapshot file.

use serde_json::json;
use tauri_plugin_appauth::Error;

#[test]
fn plain_variant_shape() {
    let err = Error::UserCanceled;
    assert_eq!(
        serde_json::to_value(&err).unwrap(),
        json!({
            "code": "USER_CANCELED",
            "message": "user canceled the authorization flow",
        }),
    );
}

#[test]
fn variant_with_string_payload_shape() {
    let err = Error::NetworkError("dns failure".into());
    assert_eq!(
        serde_json::to_value(&err).unwrap(),
        json!({
            "code": "NETWORK_ERROR",
            "message": "network error: dns failure",
        }),
    );
}

#[test]
fn authorization_failed_with_oauth_fields() {
    let err = Error::AuthorizationFailed {
        message: "denied".into(),
        oauth_error: Some("access_denied".into()),
        oauth_error_description: Some("user refused consent".into()),
    };
    assert_eq!(
        serde_json::to_value(&err).unwrap(),
        json!({
            "code": "AUTHORIZATION_FAILED",
            "message": "authorization failed: denied",
            "oauth_error": "access_denied",
            "oauth_error_description": "user refused consent",
        }),
    );
}

#[test]
fn authorization_failed_without_oauth_fields_omits_them() {
    let err = Error::AuthorizationFailed {
        message: "denied".into(),
        oauth_error: None,
        oauth_error_description: None,
    };
    assert_eq!(
        serde_json::to_value(&err).unwrap(),
        json!({
            "code": "AUTHORIZATION_FAILED",
            "message": "authorization failed: denied",
        }),
    );
}

#[test]
fn token_exchange_failed_with_partial_oauth_fields() {
    let err = Error::TokenExchangeFailed {
        message: "rejected".into(),
        oauth_error: Some("invalid_grant".into()),
        oauth_error_description: None,
    };
    assert_eq!(
        serde_json::to_value(&err).unwrap(),
        json!({
            "code": "TOKEN_EXCHANGE_FAILED",
            "message": "token exchange failed: rejected",
            "oauth_error": "invalid_grant",
        }),
    );
}

#[test]
fn unsupported_platform_shape() {
    let err = Error::UnsupportedPlatform;
    assert_eq!(
        serde_json::to_value(&err).unwrap(),
        json!({
            "code": "UNSUPPORTED_PLATFORM",
            "message": "AppAuth flows are only supported on iOS and Android",
        }),
    );
}

#[cfg(mobile)]
#[test]
fn plugin_invoke_shape() {
    let err = Error::PluginInvoke(tauri::plugin::mobile::PluginInvokeError::UnreachableWebview);
    let value = serde_json::to_value(&err).unwrap();
    assert_eq!(value["code"], json!("PLUGIN_INVOKE_FAILED"));
    assert_eq!(value["message"], json!("the webview is unreachable"));
    assert!(value.get("oauth_error").is_none());
    assert!(value.get("oauth_error_description").is_none());
}
