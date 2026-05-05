use std::net::SocketAddr;
use std::path::PathBuf;

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
    #[error("client returned error: {message}")]
    Client {
        message: String,
        details: Option<String>,
    },

    /// The frame could not be delivered to the client's outbound
    /// queue.
    #[error("failed to deliver frame to client: {0}")]
    Send(String),

    /// `BridgeService::bind` was called before TLS material was
    /// configured on the service. The bridge requires TLS — there is no
    /// plaintext fallback — so this is an unrecoverable configuration
    /// error rather than a runtime failure.
    #[error("bridge TLS material not configured; call BridgeService::configure_tls first")]
    TlsNotConfigured,

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

    /// The accept loop terminated with an unrecoverable error (TLS
    /// handshake panic, listener vanished, …).
    #[error("bridge serve loop ended with error: {source}")]
    Serve {
        #[source]
        source: std::io::Error,
    },

    /// The TLS material on disk could not be loaded into a rustls
    /// config. Typical causes: cert/key out of sync, bad PEM, key
    /// algorithm mismatch with what aws-lc-rs accepts.
    #[error(
        "failed to load bridge TLS material (cert={}, key={}): {source}",
        cert_path.display(),
        key_path.display()
    )]
    TlsLoad {
        cert_path: PathBuf,
        key_path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}
