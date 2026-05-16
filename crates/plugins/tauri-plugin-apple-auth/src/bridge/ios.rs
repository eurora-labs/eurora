//! iOS bridge: forwards [`sign_in_with_apple`] to the Swift plugin,
//! which drives `ASAuthorizationController`.

use serde::{Serialize, de::DeserializeOwned};
use tauri::{
    AppHandle, Runtime,
    plugin::{PluginApi, PluginHandle},
};

use crate::models::{AppleSignInOutcome, SignInRequest};
use crate::{Error, Result};

/// Handle to the Apple-auth plugin on iOS. Acquired via
/// [`crate::AppleAuthExt::apple_auth`].
pub struct AppleAuth<R: Runtime>(PluginHandle<R>);

impl<R: Runtime> AppleAuth<R> {
    /// Drive the Apple sheet via `ASAuthorizationController`.
    ///
    /// Returns `Ok(AppleSignInOutcome::Success { … })` when the user
    /// completes the flow, `Ok(Cancelled)` when they dismiss, and
    /// `Ok(Rejected(reason))` when Apple refuses for a non-cancellation
    /// reason (`.failed`, `.invalidResponse`, `.notHandled`, …).
    /// Bridge / serde / native crash failures bubble up as `Err`.
    pub async fn sign_in_with_apple(&self, req: SignInRequest) -> Result<AppleSignInOutcome> {
        if req.raw_nonce.is_empty() {
            return Err(Error::InvalidRequest("rawNonce must not be empty".into()));
        }
        invoke(&self.0, "signInWithApple", &req).await
    }
}

async fn invoke<P, Resp, R>(
    handle: &PluginHandle<R>,
    command: &'static str,
    payload: &P,
) -> Result<Resp>
where
    P: Serialize,
    Resp: DeserializeOwned,
    R: Runtime,
{
    tracing::debug!(target: "tauri_plugin_apple_auth", command, "invoking native bridge");
    match handle.run_mobile_plugin_async(command, payload).await {
        Ok(response) => {
            tracing::debug!(
                target: "tauri_plugin_apple_auth",
                command,
                "native bridge returned",
            );
            Ok(response)
        }
        Err(err) => {
            tracing::warn!(
                target: "tauri_plugin_apple_auth",
                command,
                error = %err,
                "native bridge failed",
            );
            Err(Error::PluginInvoke(err))
        }
    }
}

tauri::ios_plugin_binding!(init_plugin_apple_auth);

pub(crate) fn init<R: Runtime, C: DeserializeOwned>(
    _app: &AppHandle<R>,
    api: PluginApi<R, C>,
) -> Result<AppleAuth<R>> {
    let handle = api.register_ios_plugin(init_plugin_apple_auth)?;
    Ok(AppleAuth(handle))
}
