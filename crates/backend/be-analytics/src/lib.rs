//! Shared posthog capture primitive for Eurora backend services.
//!
//! Each service builds typed events with the rest of `posthog_rs` and hands
//! them to [`capture_async`]. This indirection exists so that:
//!
//! * the spawned task is bounded by a timeout (posthog can hang on DNS or
//!   network blips and a fire-and-forget `tokio::spawn` would otherwise leak
//!   indefinitely);
//! * capture failures land at a uniform log level (`warn`) across services
//!   instead of one service silencing them at `debug` and another shouting
//!   at `error`.

use std::time::Duration;

pub use posthog_rs::Event;

/// How long [`capture_async`] waits for a posthog request before giving up.
///
/// Five seconds is short enough that abandoned requests don't pile up under
/// load and long enough to absorb routine TLS/DNS variance.
pub const CAPTURE_TIMEOUT: Duration = Duration::from_secs(5);

/// Spawn a fire-and-forget task that ships `event` to posthog with a sane
/// timeout. Failures are logged at `warn` and never propagated; analytics
/// must never affect the success of the operation that produced them.
pub fn capture_async(event: Event) {
    tokio::spawn(async move {
        match tokio::time::timeout(CAPTURE_TIMEOUT, posthog_rs::capture(event)).await {
            Ok(Ok(())) => {}
            Ok(Err(e)) => tracing::warn!(error = %e, "posthog capture failed"),
            Err(_) => tracing::warn!("posthog capture timed out"),
        }
    });
}
