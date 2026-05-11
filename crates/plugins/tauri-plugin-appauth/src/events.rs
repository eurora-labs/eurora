use serde::{Deserialize, Serialize};

/// Diagnostic events emitted by the native `AppAuth` runtime as a flow
/// progresses. Subscribers receive these via a `Channel<AuthEvent>`
/// registered with [`crate::AppAuth::subscribe_events`].
///
/// The minimal set ships in v0.2.0; `#[non_exhaustive]` reserves room for
/// further events without a breaking change.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[non_exhaustive]
pub enum AuthEvent {
    /// The platform browser (Custom Tabs / `ASWebAuthenticationSession`) has
    /// been presented to the user.
    BrowserOpened,
    /// The OS handed the redirect URI back to the plugin.
    RedirectIntercepted,
    /// `AppAuth` has started the back-channel `code → token` exchange.
    TokenExchangeStarted,
    /// The token endpoint responded successfully.
    TokenExchangeCompleted,
}
