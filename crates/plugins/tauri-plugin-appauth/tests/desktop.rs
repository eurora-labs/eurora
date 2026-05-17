//! Confirms the desktop stub returns [`Error::UnsupportedPlatform`] from every
//! method, so the desktop branch can't silently break (e.g. by accidentally
//! calling into `mobile::AppAuth`).

#![cfg(not(mobile))]

use tauri::ipc::Channel;
use tauri::test::{MockRuntime, mock_builder, mock_context, noop_assets};
use tauri_plugin_appauth::{
    AppAuthExt, AuthorizeRequest, BrowserOnlyRequest, ConfigSource, DiscoverRequest,
    EndSessionRequest, Error, RefreshRequest, RegisterRequest,
};

fn app() -> tauri::App<MockRuntime> {
    mock_builder()
        .plugin(tauri_plugin_appauth::init())
        .build(mock_context(noop_assets()))
        .expect("mock app should build")
}

fn discovery() -> ConfigSource {
    ConfigSource::Discovery {
        issuer: "https://issuer.example.com".into(),
    }
}

fn assert_unsupported<T: std::fmt::Debug>(result: Result<T, Error>) {
    assert!(
        matches!(result, Err(Error::UnsupportedPlatform)),
        "expected Error::UnsupportedPlatform, got {result:?}",
    );
}

#[test]
fn discover_returns_unsupported() {
    let app = app();
    let res = tauri::async_runtime::block_on(app.appauth().discover(DiscoverRequest {
        issuer: "https://issuer.example.com".into(),
    }));
    assert_unsupported(res);
}

#[test]
fn register_returns_unsupported() {
    let app = app();
    let res = tauri::async_runtime::block_on(app.appauth().register(RegisterRequest {
        config: discovery(),
        redirect_uris: vec!["com.example.app:/oauth/callback".into()],
        client_name: None,
        response_types: vec![],
        grant_types: vec![],
        subject_types: vec![],
        token_endpoint_auth_method: None,
        additional_parameters: Default::default(),
    }));
    assert_unsupported(res);
}

#[test]
fn authorize_returns_unsupported() {
    let app = app();
    let res = tauri::async_runtime::block_on(app.appauth().authorize(AuthorizeRequest {
        config: discovery(),
        client_id: "client-123".into(),
        redirect_uri: "com.example.app:/oauth/callback".into(),
        scopes: vec![],
        additional_parameters: Default::default(),
        prompt: None,
        login_hint: None,
        ui_locales: None,
        prefers_ephemeral_session: true,
        use_nonce: true,
    }));
    assert_unsupported(res);
}

#[test]
fn authorize_browser_only_returns_unsupported() {
    let app = app();
    let res =
        tauri::async_runtime::block_on(app.appauth().authorize_browser_only(BrowserOnlyRequest {
            auth_url: "https://issuer.example.com/auth".into(),
            redirect_uri: "com.example.app:/oauth/callback".into(),
            prefers_ephemeral_session: true,
        }));
    assert_unsupported(res);
}

#[test]
fn refresh_returns_unsupported() {
    let app = app();
    let res = tauri::async_runtime::block_on(app.appauth().refresh(RefreshRequest {
        config: discovery(),
        client_id: "client-123".into(),
        refresh_token: "rt".into(),
        scopes: vec![],
        additional_parameters: Default::default(),
    }));
    assert_unsupported(res);
}

#[test]
fn end_session_returns_unsupported() {
    let app = app();
    let res = tauri::async_runtime::block_on(app.appauth().end_session(EndSessionRequest {
        config: discovery(),
        id_token_hint: Some("it".into()),
        post_logout_redirect_uri: "com.example.app:/post-logout".into(),
        state: None,
        additional_parameters: Default::default(),
        prefers_ephemeral_session: true,
    }));
    assert_unsupported(res);
}

#[test]
fn subscribe_events_returns_unsupported() {
    let app = app();
    let channel = Channel::new(|_| Ok(()));
    let res = tauri::async_runtime::block_on(app.appauth().subscribe_events(channel));
    assert_unsupported(res);
}
