use tauri::{AppHandle, Runtime, command, ipc::Channel};

use crate::events::AuthEvent;
use crate::models::{
    AuthState, AuthorizeRequest, BrowserOnlyRequest, BrowserOnlyResponse, DiscoverRequest,
    EndSessionRequest, EndSessionResponse, RefreshRequest, RegisterRequest, RegistrationResponse,
    ServiceConfiguration,
};
use crate::{AppAuthExt, Result};

#[command]
pub(crate) async fn discover<R: Runtime>(
    app: AppHandle<R>,
    payload: DiscoverRequest,
) -> Result<ServiceConfiguration> {
    app.appauth().discover(payload).await
}

#[command]
pub(crate) async fn register<R: Runtime>(
    app: AppHandle<R>,
    payload: RegisterRequest,
) -> Result<RegistrationResponse> {
    app.appauth().register(payload).await
}

#[command]
pub(crate) async fn authorize<R: Runtime>(
    app: AppHandle<R>,
    payload: AuthorizeRequest,
) -> Result<AuthState> {
    app.appauth().authorize(payload).await
}

#[command]
pub(crate) async fn authorize_browser_only<R: Runtime>(
    app: AppHandle<R>,
    payload: BrowserOnlyRequest,
) -> Result<BrowserOnlyResponse> {
    app.appauth().authorize_browser_only(payload).await
}

#[command]
pub(crate) async fn refresh<R: Runtime>(
    app: AppHandle<R>,
    payload: RefreshRequest,
) -> Result<AuthState> {
    app.appauth().refresh(payload).await
}

#[command]
pub(crate) async fn end_session<R: Runtime>(
    app: AppHandle<R>,
    payload: EndSessionRequest,
) -> Result<EndSessionResponse> {
    app.appauth().end_session(payload).await
}

/// Register a `Channel<AuthEvent>` with the native side. Native handlers will
/// emit diagnostic events through this channel as flows progress. Call once
/// per session; calling again replaces the previous channel.
#[command]
pub(crate) async fn subscribe_events<R: Runtime>(
    app: AppHandle<R>,
    channel: Channel<AuthEvent>,
) -> Result<()> {
    app.appauth().subscribe_events(channel).await
}
