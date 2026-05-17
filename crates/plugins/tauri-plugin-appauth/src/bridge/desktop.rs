use std::marker::PhantomData;

use serde::de::DeserializeOwned;
use tauri::{AppHandle, Runtime, ipc::Channel, plugin::PluginApi};

use crate::events::AuthEvent;
use crate::models::{
    AuthState, AuthorizeRequest, BrowserOnlyRequest, BrowserOnlyResponse, DiscoverRequest,
    EndSessionRequest, EndSessionResponse, RefreshRequest, RegisterRequest, RegistrationResponse,
    ServiceConfiguration,
};
use crate::{Error, Result};

/// Handle to the AppAuth-backed plugin on desktop targets.
///
/// Every method returns [`Error::UnsupportedPlatform`]. Desktop OAuth has its
/// own canonical plugin (`tauri-plugin-oauth`); use that instead.
///
/// `PhantomData<fn() -> R>` is unconditionally `Send + Sync`, which is what
/// Tauri's `Manager::manage` requires.
pub struct AppAuth<R: Runtime>(PhantomData<fn() -> R>);

impl<R: Runtime> AppAuth<R> {
    /// Always returns [`Error::UnsupportedPlatform`] on desktop targets.
    pub async fn discover(&self, _req: DiscoverRequest) -> Result<ServiceConfiguration> {
        Err(Error::UnsupportedPlatform)
    }

    /// Always returns [`Error::UnsupportedPlatform`] on desktop targets.
    pub async fn register(&self, _req: RegisterRequest) -> Result<RegistrationResponse> {
        Err(Error::UnsupportedPlatform)
    }

    /// Always returns [`Error::UnsupportedPlatform`] on desktop targets.
    pub async fn authorize(&self, _req: AuthorizeRequest) -> Result<AuthState> {
        Err(Error::UnsupportedPlatform)
    }

    /// Always returns [`Error::UnsupportedPlatform`] on desktop targets.
    pub async fn authorize_browser_only(
        &self,
        _req: BrowserOnlyRequest,
    ) -> Result<BrowserOnlyResponse> {
        Err(Error::UnsupportedPlatform)
    }

    /// Always returns [`Error::UnsupportedPlatform`] on desktop targets.
    pub async fn refresh(&self, _req: RefreshRequest) -> Result<AuthState> {
        Err(Error::UnsupportedPlatform)
    }

    /// Always returns [`Error::UnsupportedPlatform`] on desktop targets.
    pub async fn end_session(&self, _req: EndSessionRequest) -> Result<EndSessionResponse> {
        Err(Error::UnsupportedPlatform)
    }

    /// Always returns [`Error::UnsupportedPlatform`] on desktop targets.
    pub async fn subscribe_events(&self, _channel: Channel<AuthEvent>) -> Result<()> {
        Err(Error::UnsupportedPlatform)
    }
}

pub(crate) fn init<R: Runtime, C: DeserializeOwned>(
    _app: &AppHandle<R>,
    _api: PluginApi<R, C>,
) -> Result<AppAuth<R>> {
    Ok(AppAuth(PhantomData))
}
