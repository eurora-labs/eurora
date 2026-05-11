use serde::{Serialize, de::DeserializeOwned};
use tauri::{
    AppHandle, Runtime,
    ipc::Channel,
    plugin::{PluginApi, PluginHandle},
};

use crate::events::AuthEvent;
use crate::models::{
    AuthState, AuthorizeRequest, BrowserOnlyRequest, BrowserOnlyResponse, DiscoverRequest,
    EndSessionRequest, EndSessionResponse, RefreshRequest, RegisterRequest, RegistrationResponse,
    ServiceConfiguration,
};
use crate::{Error, Result};

/// Handle to the AppAuth-backed plugin. Acquired via [`crate::AppAuthExt::appauth`].
pub struct AppAuth<R: Runtime>(PluginHandle<R>);

impl<R: Runtime> AppAuth<R> {
    /// Resolve `<issuer>/.well-known/openid-configuration` (or RFC 8414
    /// equivalent) into a [`ServiceConfiguration`].
    pub async fn discover(&self, req: DiscoverRequest) -> Result<ServiceConfiguration> {
        invoke(&self.0, "discover", &req).await
    }

    /// Perform RFC 7591 dynamic client registration against an issuer that
    /// supports it. Most providers do not; check the discovery document's
    /// `registration_endpoint`.
    pub async fn register(&self, req: RegisterRequest) -> Result<RegistrationResponse> {
        invoke(&self.0, "register", &req).await
    }

    /// Open the platform browser, run PKCE, validate `state`/`nonce`, and
    /// exchange the authorization code for tokens. Returns the full
    /// post-exchange [`AuthState`].
    pub async fn authorize(&self, req: AuthorizeRequest) -> Result<AuthState> {
        invoke(&self.0, "authorize", &req).await
    }

    /// Open the browser at `auth_url`, capture the redirect to `redirect_uri`,
    /// and return the raw callback URL without performing a token exchange.
    /// Use this when a backend mediates the code-for-token swap.
    pub async fn authorize_browser_only(
        &self,
        req: BrowserOnlyRequest,
    ) -> Result<BrowserOnlyResponse> {
        invoke(&self.0, "authorizeBrowserOnly", &req).await
    }

    /// Trade a refresh token for a fresh access token via the issuer's token
    /// endpoint.
    pub async fn refresh(&self, req: RefreshRequest) -> Result<AuthState> {
        invoke(&self.0, "refresh", &req).await
    }

    /// RFC 8665 RP-initiated logout. Resolves once the post-logout redirect
    /// fires.
    pub async fn end_session(&self, req: EndSessionRequest) -> Result<EndSessionResponse> {
        invoke(&self.0, "endSession", &req).await
    }

    /// Register a [`Channel`] that the native side will use to emit
    /// [`AuthEvent`]s as flows progress. Call once per session; calling again
    /// replaces the previous subscription.
    pub async fn subscribe_events(&self, channel: Channel<AuthEvent>) -> Result<()> {
        #[derive(Serialize)]
        struct Payload<'a> {
            channel: &'a Channel<AuthEvent>,
        }
        invoke(&self.0, "subscribeEvents", &Payload { channel: &channel }).await
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
    tracing::debug!(target: "tauri_plugin_appauth", command, "invoking native bridge");
    match handle.run_mobile_plugin_async(command, payload).await {
        Ok(response) => {
            tracing::debug!(
                target: "tauri_plugin_appauth",
                command,
                "native bridge returned",
            );
            Ok(response)
        }
        Err(err) => {
            tracing::warn!(
                target: "tauri_plugin_appauth",
                command,
                error = %err,
                "native bridge failed",
            );
            Err(Error::PluginInvoke(err))
        }
    }
}

cfg_select! {
    target_os = "ios" => {
        tauri::ios_plugin_binding!(init_plugin_appauth);

        pub(crate) fn init<R: Runtime, C: DeserializeOwned>(
            _app: &AppHandle<R>,
            api: PluginApi<R, C>,
        ) -> Result<AppAuth<R>> {
            let handle = api.register_ios_plugin(init_plugin_appauth)?;
            Ok(AppAuth(handle))
        }
    }
    target_os = "android" => {
        const PLUGIN_IDENTIFIER: &str = "app.tauri.appauth";

        pub(crate) fn init<R: Runtime, C: DeserializeOwned>(
            _app: &AppHandle<R>,
            api: PluginApi<R, C>,
        ) -> Result<AppAuth<R>> {
            let handle = api.register_android_plugin(PLUGIN_IDENTIFIER, "AppAuthPlugin")?;
            Ok(AppAuth(handle))
        }
    }
    _ => {
        compile_error!("tauri-plugin-appauth bridge/mobile.rs requires target_os = \"ios\" or \"android\"");
    }
}
