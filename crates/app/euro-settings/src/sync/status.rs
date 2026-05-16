//! Observable status surface for the cloud-settings sync engine.
//!
//! The engine publishes its current state into a `tokio::sync::watch`
//! channel; subscribers see the latest value at any moment and never
//! miss "the most recent" (older intermediate transitions are
//! coalesced). The engine holds an internal receiver on the channel so
//! the value remains current even when no external subscriber is
//! attached.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use specta::Type;

/// High-level state of the sync engine, presented as a single enum so
/// callers can render a status row without juggling several flags.
///
/// Transitions (driven by [`crate::sync::engine::SyncEngine`]):
///
/// - [`SyncStatus::LocalOnly`] — boot default and the state the engine
///   sits in while no authenticated user is present. The engine
///   performs no network I/O in this state.
/// - [`SyncStatus::Syncing`] — a request is in flight (pull or push).
/// - [`SyncStatus::Synced { at }`] — the most recent network round-trip
///   completed cleanly; `at` is the wall-clock time it finished.
/// - [`SyncStatus::Offline { since }`] — a transient transport or
///   server error occurred; the engine keeps the local cache and will
///   retry on the next trigger.
/// - [`SyncStatus::Conflict { at }`] — a PUT raced another client and
///   the server's row replaced the local cache. The UI shows this
///   until the next successful round-trip clears it.
/// - [`SyncStatus::ServerAhead`] — the server is on a newer schema
///   version than this build understands. The engine refuses to write
///   so it cannot silently downgrade fields it could not parse; the UI
///   should surface an upgrade prompt. Cleared by restart on a build
///   that supports the server's version.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum SyncStatus {
    #[default]
    LocalOnly,
    Syncing,
    Synced {
        at: DateTime<Utc>,
    },
    Offline {
        since: DateTime<Utc>,
    },
    Conflict {
        at: DateTime<Utc>,
    },
    ServerAhead,
}

impl SyncStatus {
    /// Convenience predicate used by tests and by the UI to gate the
    /// "refresh" button while a request is already in flight.
    #[must_use]
    pub fn is_syncing(&self) -> bool {
        matches!(self, SyncStatus::Syncing)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_only_is_default() {
        assert_eq!(SyncStatus::default(), SyncStatus::LocalOnly);
    }

    #[test]
    fn serde_uses_kind_tag() {
        let synced = SyncStatus::Synced {
            at: DateTime::<Utc>::UNIX_EPOCH,
        };
        let v = serde_json::to_value(&synced).unwrap();
        assert_eq!(v["kind"], "synced");
        let back: SyncStatus = serde_json::from_value(v).unwrap();
        assert_eq!(back, synced);
    }

    #[test]
    fn conflict_carries_at_timestamp() {
        let conflict = SyncStatus::Conflict {
            at: DateTime::<Utc>::UNIX_EPOCH,
        };
        let v = serde_json::to_value(&conflict).unwrap();
        assert_eq!(v["kind"], "conflict");
        assert!(v.get("at").is_some());
        let back: SyncStatus = serde_json::from_value(v).unwrap();
        assert_eq!(back, conflict);
    }
}
