use std::net::SocketAddr;

use thiserror::Error;

/// Errors surfaced by the bridge service to its callers.
#[derive(Debug, Error)]
pub enum BridgeError {
    /// No client is registered for the requested `app_pid`.
    #[error("no client registered for app_pid {app_pid}")]
    NotFound { app_pid: u32 },

    /// The remote did not reply within the configured timeout. The
    /// bridge sends a `CancelFrame` to the client when this happens.
    #[error("request timed out")]
    Timeout,

    /// The response channel was dropped before a reply arrived. Usually
    /// means the client disconnected mid-request.
    #[error("response channel closed before a reply was received")]
    ChannelClosed,

    /// The client returned an [`crate::ErrorFrame`] in response to the
    /// request.
    ///
    /// `code` carries the application-level status the client populated
    /// on the `ErrorFrame`. Conventional values follow HTTP semantics
    /// (`400` malformed request, `410` resource gone, `500` internal),
    /// with `0` reserved for "no code supplied".
    #[error("client returned error {code}: {message}")]
    Client {
        code: u32,
        message: String,
        details: Option<String>,
    },

    /// The frame could not be delivered to the client's outbound
    /// queue.
    #[error("failed to deliver frame to client: {0}")]
    Send(String),

    /// `BridgeService::bind` was called while the listener was already
    /// running. Surfaced explicitly (rather than silently no-op'd) so
    /// callers that want "ensure running" semantics check
    /// [`BridgeService::local_addr`] first and the lifecycle stays
    /// observable in logs.
    #[error("bridge listener already running on {local_addr}")]
    AlreadyRunning { local_addr: SocketAddr },

    /// The OS refused to bind the requested address (port already in
    /// use, IPv6 disabled, sandbox restriction, …).
    #[error("failed to bind bridge listener on {addr}: {source}")]
    Bind {
        addr: SocketAddr,
        #[source]
        source: std::io::Error,
    },

    /// The accept loop terminated with an unrecoverable error.
    #[error("bridge serve loop ended with error: {source}")]
    Serve {
        #[source]
        source: std::io::Error,
    },
}
