//! Tauri 2 mobile plugin that bridges Sign in with Apple to the native
//! `ASAuthorizationController` on iOS.
//!
//! The native iOS flow is in-process: there is no browser round-trip,
//! no PKCE, no `state`, no redirect URI. The plugin opens the system
//! Apple sheet, obtains an Apple-signed ID token bound to the iOS
//! Bundle ID, and hands it back to the caller. The Rust side passes
//! that token to the Eurora backend, which verifies signature + nonce
//! against Apple's JWKS and mints a session.
//!
//! Nonce: the caller supplies a raw, unhashed nonce. The plugin
//! SHA-256-hashes it (and base64url-encodes the digest) before
//! assigning to `ASAuthorizationAppleIDRequest.nonce` — Apple echoes
//! whatever the client puts there verbatim into the ID token's
//! `nonce` claim. The backend re-derives the same hash to match.
//! Keeping the hashing on **both** the iOS plugin and the backend
//! means the caller never has to know about Apple's quirk.
//!
//! Android and desktop targets reject every call with
//! [`Error::UnsupportedPlatform`]: Apple ships no SDK for either,
//! and proxying through Google Credential Manager would be misleading
//! (Apple wouldn't accept the resulting identity).

#![deny(missing_docs)]

use tauri::{
    Manager, Runtime,
    plugin::{Builder, TauriPlugin},
};

mod bridge;
mod commands;
mod error;
mod models;

pub use bridge::AppleAuth;
pub use error::{Error, Result};
pub use models::{AppleNativeUser, AppleSignInOutcome, SignInRequest, SignInResponse};

/// Extension trait that hangs an [`AppleAuth`] handle off any [`Manager`].
pub trait AppleAuthExt<R: Runtime> {
    /// Returns the plugin handle managed by this Tauri app.
    fn apple_auth(&self) -> &AppleAuth<R>;
}

impl<R: Runtime, T: Manager<R>> AppleAuthExt<R> for T {
    fn apple_auth(&self) -> &AppleAuth<R> {
        self.state::<AppleAuth<R>>().inner()
    }
}

/// Build the plugin and register its commands. Pass the result to
/// `tauri::Builder::plugin`.
#[must_use]
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("apple-auth")
        .invoke_handler(tauri::generate_handler![commands::sign_in_with_apple])
        .setup(|app, api| {
            let plugin = bridge::init(app, api)?;
            app.manage(plugin);
            Ok(())
        })
        .build()
}
