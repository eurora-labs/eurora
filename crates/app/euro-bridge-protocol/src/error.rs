use thiserror::Error;

/// Errors surfaced by the bridge service to its callers. Replaces the
/// `tonic::Status` values that the gRPC-era API returned.
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

    /// Router delivered something other than a `Response` or `Error`
    /// frame in reply slot — protocol violation by the client.
    #[error("unexpected frame in response slot: expected Response or Error, got {0}")]
    UnexpectedFrame(&'static str),

    /// The frame could not be delivered to the client's outbound
    /// queue.
    #[error("failed to deliver frame to client: {0}")]
    Send(String),
}
