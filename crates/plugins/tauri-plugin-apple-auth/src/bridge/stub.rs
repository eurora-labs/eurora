//! Non-iOS handle: every call resolves to
//! [`crate::AppleSignInOutcome::NativeUnavailable`].
//!
//! Apple ships no SDK for Android or desktop, and proxying through
//! another provider (e.g. Google Credential Manager) is a footgun:
//! Apple wouldn't accept the resulting identity, and the user would
//! silently end up signed in to the wrong account-link. Surface
//! "not available" to the caller and let them open the browser flow
//! instead.

use std::marker::PhantomData;

use serde::de::DeserializeOwned;
use tauri::{AppHandle, Runtime, plugin::PluginApi};

use crate::Result;
use crate::models::{AppleSignInOutcome, SignInRequest};

/// Stub handle used on every non-iOS target.
///
/// `PhantomData<fn() -> R>` is unconditionally `Send + Sync`, which is
/// what Tauri's `Manager::manage` requires.
pub struct AppleAuth<R: Runtime>(PhantomData<fn() -> R>);

impl<R: Runtime> AppleAuth<R> {
    /// Always resolves to
    /// [`AppleSignInOutcome::NativeUnavailable`] on non-iOS targets.
    pub async fn sign_in_with_apple(
        &self,
        _req: SignInRequest,
    ) -> Result<AppleSignInOutcome> {
        Ok(AppleSignInOutcome::NativeUnavailable)
    }
}

pub(crate) fn init<R: Runtime, C: DeserializeOwned>(
    _app: &AppHandle<R>,
    _api: PluginApi<R, C>,
) -> Result<AppleAuth<R>> {
    Ok(AppleAuth(PhantomData))
}
