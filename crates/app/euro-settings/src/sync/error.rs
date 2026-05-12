//! Error surface for the sync engine.
//!
//! Errors are classified by their downstream effect on [`SyncStatus`]
//! rather than by their cause. The single [`SyncError::into_status`]
//! helper is the only place that mapping lives: callers see typed
//! variants, and the engine uses `into_status` to produce a
//! user-facing status update.

use chrono::Utc;
use settings_core::PutSettingsConflictResponse;
use thiserror::Error;

use super::status::SyncStatus;

/// Result alias used throughout the sync module.
pub type SyncResult<T> = Result<T, SyncError>;

/// Engine-internal error type. Variants are deliberately narrow so the
/// engine can branch on the *kind* of failure rather than parsing a
/// stringified message.
///
/// `Conflict` is its own variant because reconciliation is a normal
/// outcome of `PUT /settings`, not an exceptional one — the engine
/// matches on it and applies the server's row rather than retrying.
#[derive(Debug, Error)]
pub enum SyncError {
    /// No authenticated user. Engine sits in `LocalOnly` until auth
    /// state changes.
    #[error("no authenticated user")]
    NotAuthenticated,

    /// Auth refresh failed. Distinguished from `NotAuthenticated` so a
    /// transient refresh failure (server unreachable) doesn't masquerade
    /// as a logout.
    #[error("auth: {0}")]
    Auth(#[from] euro_auth::AuthError),

    /// Network-level failure (DNS, TLS, connection reset, timeout).
    /// Always classified as transient → `Offline`.
    #[error("transport: {0}")]
    Transport(#[source] reqwest::Error),

    /// Non-2xx, non-404, non-409 response from the settings service.
    #[error("server returned {status}: {message}")]
    Server {
        status: reqwest::StatusCode,
        message: String,
    },

    /// Server returned 409 with the current row. Not an error condition
    /// per se — the engine consumes this to replace the local cache.
    #[error("conflict; server row at {}", .0.updated_at)]
    Conflict(PutSettingsConflictResponse),

    /// JSON decode failure when interpreting a successful response. Rare
    /// in practice; almost always means a wire-incompatible deploy.
    #[error("decode: {0}")]
    Decode(#[from] serde_json::Error),

    /// Failure writing the cache file after a successful pull / conflict
    /// reconcile. Surfaces as `Offline` to nudge a retry without
    /// dropping the in-memory cache update.
    #[error("io: {0}")]
    Io(#[from] std::io::Error),

    /// Catch-all for anyhow-typed plumbing (e.g. config-dir resolution).
    #[error("internal: {0}")]
    Internal(#[from] anyhow::Error),
}

impl SyncError {
    /// Build a `SyncError::Transport` from a `reqwest::Error`. Avoid
    /// `#[from]` so call sites stay explicit about classifying a
    /// transport failure (versus a decode failure on the same reqwest
    /// surface, which goes through `Decode`).
    #[must_use]
    pub fn from_transport(err: reqwest::Error) -> Self {
        Self::Transport(err)
    }

    /// Project the error into the status the engine should publish.
    /// Single point so the watch-channel update never disagrees with
    /// the error type the caller saw.
    #[must_use]
    pub fn into_status(&self) -> SyncStatus {
        match self {
            SyncError::NotAuthenticated => SyncStatus::LocalOnly,
            SyncError::Auth(e) if e.is_logged_out() => SyncStatus::LocalOnly,
            SyncError::Auth(_)
            | SyncError::Transport(_)
            | SyncError::Server { .. }
            | SyncError::Io(_)
            | SyncError::Decode(_)
            | SyncError::Internal(_) => SyncStatus::Offline { since: Utc::now() },
            SyncError::Conflict(_) => SyncStatus::Conflict { at: Utc::now() },
        }
    }

    /// Whether a retry is sensible after this error. 4xx-class server
    /// errors and decode failures are not retried because the next
    /// attempt would carry the same payload and hit the same response.
    #[must_use]
    pub fn is_retryable(&self) -> bool {
        match self {
            SyncError::NotAuthenticated | SyncError::Conflict(_) | SyncError::Decode(_) => false,
            SyncError::Auth(e) => !e.is_logged_out(),
            SyncError::Transport(_) | SyncError::Io(_) | SyncError::Internal(_) => true,
            SyncError::Server { status, .. } => status.is_server_error(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::DateTime;

    #[test]
    fn conflict_classifies_as_conflict_status() {
        let err = SyncError::Conflict(PutSettingsConflictResponse {
            schema_version: 1,
            updated_at: DateTime::<Utc>::UNIX_EPOCH,
            current: serde_json::Value::Null,
        });
        assert!(matches!(err.into_status(), SyncStatus::Conflict { .. }));
        assert!(!err.is_retryable());
    }

    #[test]
    fn not_authenticated_classifies_as_local_only() {
        let err = SyncError::NotAuthenticated;
        assert_eq!(err.into_status(), SyncStatus::LocalOnly);
        assert!(!err.is_retryable());
    }

    #[test]
    fn auth_logged_out_classifies_as_local_only() {
        let err = SyncError::Auth(euro_auth::AuthError::InvalidRefreshToken);
        assert!(matches!(err.into_status(), SyncStatus::LocalOnly));
        assert!(!err.is_retryable());
    }

    #[test]
    fn server_5xx_is_retryable_4xx_is_not() {
        let server_500 = SyncError::Server {
            status: reqwest::StatusCode::INTERNAL_SERVER_ERROR,
            message: "boom".to_owned(),
        };
        assert!(server_500.is_retryable());
        let server_400 = SyncError::Server {
            status: reqwest::StatusCode::BAD_REQUEST,
            message: "bad".to_owned(),
        };
        assert!(!server_400.is_retryable());
    }
}
