//! Tauri 2 mobile plugin that bridges OAuth 2.0 / OIDC flows to the
//! reference RFC 8252 implementations: AppAuth-iOS and AppAuth-Android.
//!
//! `AppAuth` handles PKCE (S256), `state`/`nonce` validation, discovery
//! (RFC 8414 / OIDC), authorization, token exchange, refresh, end-session
//! (RFC 8665), and dynamic client registration (RFC 7591). This crate is a
//! thin Rust + native bridge — we do not write OAuth state machines.
//!
//! Desktop targets reject every command with [`Error::UnsupportedPlatform`];
//! desktop OAuth has its own canonical plugin (`tauri-plugin-oauth`).

#![deny(missing_docs)]

use tauri::{
    Manager, Runtime,
    plugin::{Builder, TauriPlugin},
};

mod bridge;
mod commands;
mod error;
mod events;
mod models;

pub use bridge::AppAuth;
pub use error::{Error, Result};
pub use events::AuthEvent;
pub use models::{
    AuthState, AuthorizeRequest, BrowserOnlyRequest, BrowserOnlyResponse, ConfigSource,
    DiscoverRequest, EndSessionRequest, EndSessionResponse, ExtensionFields, Prompt, QueryParams,
    RefreshRequest, RegisterRequest, RegistrationResponse, ServiceConfiguration,
};
pub use tauri::ipc::Channel;

/// Extension trait that hangs an [`AppAuth`] handle off any [`Manager`].
pub trait AppAuthExt<R: Runtime> {
    /// Returns the plugin handle managed by this Tauri app.
    fn appauth(&self) -> &AppAuth<R>;
}

impl<R: Runtime, T: Manager<R>> AppAuthExt<R> for T {
    fn appauth(&self) -> &AppAuth<R> {
        self.state::<AppAuth<R>>().inner()
    }
}

/// Build the plugin and register its commands. Pass the result to
/// `tauri::Builder::plugin`.
#[must_use]
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("appauth")
        .invoke_handler(tauri::generate_handler![
            commands::discover,
            commands::register,
            commands::authorize,
            commands::authorize_browser_only,
            commands::refresh,
            commands::end_session,
            commands::subscribe_events,
        ])
        .setup(|app, api| {
            let plugin = bridge::init(app, api)?;
            app.manage(plugin);
            Ok(())
        })
        .build()
}
