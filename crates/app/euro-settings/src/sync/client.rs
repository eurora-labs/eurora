//! HTTP transport for the cloud-settings sync engine.
//!
//! Two layers live here:
//!
//! - [`SettingsTransport`] — async trait describing the GET/PUT/DELETE
//!   surface the engine needs. Object-safe via `async_trait` so the
//!   engine can hold an `Arc<dyn SettingsTransport>` and tests can
//!   substitute a fake without spinning up a real keyring-backed
//!   [`euro_auth::AuthManager`].
//! - [`ReqwestTransport`] — production implementation. Reads a fresh
//!   bearer token from the auth manager on every call, joins the
//!   request path against the live [`euro_endpoint::EndpointManager`]
//!   URL, and classifies the response into the typed
//!   [`super::error::SyncError`] surface.
//!
//! Keeping auth and HTTP behind a trait also means the engine's
//! reconciliation logic is testable in isolation: the wiremock-backed
//! tests in `tests/sync.rs` exercise the full HTTP path, while the
//! engine's pull/push/reconcile branches can be unit-tested against a
//! deterministic in-memory fake.

use std::sync::Arc;

use async_trait::async_trait;
use euro_auth::AuthManager;
use euro_endpoint::EndpointManager;
use reqwest::StatusCode;
use secrecy::ExposeSecret;
use serde::de::DeserializeOwned;
use settings_core::{
    GetSettingsResponse, PutSettingsAcceptedResponse, PutSettingsConflictResponse,
    PutSettingsRequest,
};

use super::error::{SyncError, SyncResult};

/// Outcome of a `GET /settings` call. 404 is modeled as a value rather
/// than an error because the engine's pull loop treats it as "no row
/// for this user yet; do a first-run upload" — see
/// [`super::migrate::upload_first_run`].
#[derive(Debug, Clone)]
pub enum PullOutcome {
    NotFound,
    Found(GetSettingsResponse),
}

/// Outcome of a `PUT /settings` call. The engine treats 200 and 409 as
/// the two normal terminal states; transient and server errors are
/// surfaced via [`SyncError`] and trigger retry / status updates.
#[derive(Debug, Clone)]
pub enum PushOutcome {
    Accepted(PutSettingsAcceptedResponse),
    Conflict(PutSettingsConflictResponse),
}

/// Abstraction over the `/settings` HTTP surface.
///
/// Engine tests substitute this with a deterministic in-memory fake
/// (see `tests/sync.rs`) so reconciliation can be exercised without
/// the keyring + secret-store machinery that the real auth client
/// requires.
#[async_trait]
pub trait SettingsTransport: Send + Sync + 'static {
    async fn get(&self) -> SyncResult<PullOutcome>;
    async fn put(&self, body: PutSettingsRequest) -> SyncResult<PushOutcome>;
    async fn delete(&self) -> SyncResult<()>;
}

/// Production transport. Holds clone-cheap handles to the shared
/// [`EndpointManager`] and [`AuthManager`]; both are designed to be
/// shared across the app and refresh themselves transparently.
#[derive(Clone)]
pub struct ReqwestTransport {
    endpoint: Arc<EndpointManager>,
    auth: AuthManager,
    http: reqwest::Client,
}

impl std::fmt::Debug for ReqwestTransport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReqwestTransport")
            .field("base_url", &self.endpoint.current_url().as_str())
            .finish()
    }
}

impl ReqwestTransport {
    /// Build a transport bound to the shared endpoint manager and auth
    /// manager. The shared HTTP client carries the workspace TLS
    /// configuration; constructing a fresh one would re-validate the
    /// trust store on every engine instantiation.
    #[must_use]
    pub fn new(endpoint: Arc<EndpointManager>, auth: AuthManager) -> Self {
        let http = endpoint.client();
        Self {
            endpoint,
            auth,
            http,
        }
    }

    async fn bearer(&self) -> SyncResult<String> {
        let token = self
            .auth
            .get_or_refresh_access_token()
            .await
            .map_err(SyncError::Auth)?;
        Ok(token.expose_secret().to_owned())
    }
}

#[async_trait]
impl SettingsTransport for ReqwestTransport {
    async fn get(&self) -> SyncResult<PullOutcome> {
        let bearer = self.bearer().await?;
        let response = self
            .http
            .get(self.endpoint.url("/settings"))
            .bearer_auth(bearer)
            .send()
            .await
            .map_err(SyncError::from_transport)?;

        match response.status() {
            StatusCode::OK => Ok(PullOutcome::Found(decode_body(response).await?)),
            StatusCode::NOT_FOUND => Ok(PullOutcome::NotFound),
            status => Err(classify_error_response(status, response).await),
        }
    }

    async fn put(&self, body: PutSettingsRequest) -> SyncResult<PushOutcome> {
        let bearer = self.bearer().await?;
        let response = self
            .http
            .put(self.endpoint.url("/settings"))
            .bearer_auth(bearer)
            .json(&body)
            .send()
            .await
            .map_err(SyncError::from_transport)?;

        match response.status() {
            StatusCode::OK => Ok(PushOutcome::Accepted(decode_body(response).await?)),
            StatusCode::CONFLICT => Ok(PushOutcome::Conflict(decode_body(response).await?)),
            status => Err(classify_error_response(status, response).await),
        }
    }

    async fn delete(&self) -> SyncResult<()> {
        let bearer = self.bearer().await?;
        let response = self
            .http
            .delete(self.endpoint.url("/settings"))
            .bearer_auth(bearer)
            .send()
            .await
            .map_err(SyncError::from_transport)?;

        match response.status() {
            // The service returns 204 for both "row existed and was
            // deleted" and "no row existed," so we don't distinguish.
            StatusCode::NO_CONTENT | StatusCode::OK => Ok(()),
            status => Err(classify_error_response(status, response).await),
        }
    }
}

/// Buffer a successful response body and decode it as JSON into `T`.
///
/// Wire-level read failures surface as [`SyncError::Transport`] (the
/// connection broke mid-body — retry); JSON shape failures surface as
/// [`SyncError::Decode`] (the server returned a body the client cannot
/// interpret, which is almost always a wire-incompatible deploy — do
/// not retry).
async fn decode_body<T: DeserializeOwned>(response: reqwest::Response) -> SyncResult<T> {
    let bytes = response.bytes().await.map_err(SyncError::from_transport)?;
    Ok(serde_json::from_slice(&bytes)?)
}

/// Drain the response body into a `SyncError::Server` with as much
/// detail as the wire carries. Used for any status the engine doesn't
/// model as a typed outcome (i.e. anything outside 200 / 404 / 409 /
/// 204).
async fn classify_error_response(status: StatusCode, response: reqwest::Response) -> SyncError {
    let message = response.text().await.unwrap_or_default();
    SyncError::Server { status, message }
}
