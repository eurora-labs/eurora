//! Client-side active-context tracking.
//!
//! The [`ContextRegistry`] is the single source of truth for "what is
//! currently surface-able to the LLM" on a desktop client. Active
//! contexts are activated and deactivated by external observers — the
//! browser bridge (URL changes in the extension), the OS focus tracker,
//! and future ACP session handlers. The registry sits between those
//! observers and the per-turn `TurnState` snapshot taken by
//! `ChatBridge::start_turn` (see `plan.md`).
//!
//! The registry is owned in `euro-tauri` and exposed through
//! Tauri-managed state, but the types live here so the chat-side bridge
//! (in `euro-thread`) can call [`ContextRegistry::snapshot`] without
//! pulling in a Tauri dependency.
//!
//! # Invariants
//!
//! - Activation is keyed by [`ActiveContext::key`] alone. Re-activating
//!   the same key overwrites the previous entry. This implements the
//!   "last activation wins" policy declared in v1 scope, so navigating
//!   away from one YouTube video to another doesn't leave both pinned.
//! - Deactivating an unknown key is a no-op — observers fire one signal
//!   per state transition, but the registry is permissive so missed
//!   activations (e.g. across a desktop restart) don't crash the next
//!   deactivation.
//! - Snapshots return cloned `Vec<ActiveContext>`. The active set is
//!   tiny in v1 (≤ one entry in practice) so the clone cost is
//!   irrelevant.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde_json::Value;

use crate::origin::Origin;

/// One currently-active context entry on the client.
///
/// Mirrors [`thread_core::WireActiveContext`] but additionally carries
/// the routing [`Origin`]. The wire projection is produced by stripping
/// `origin` (it never crosses the chat WebSocket).
#[derive(Debug, Clone)]
pub struct ActiveContext {
    /// Stable namespaced key, e.g. `"youtube::watch_page"`. The key
    /// determines which [`Origin`] variant the entry will carry.
    pub key: String,
    /// When the observer reported the activation. Frozen at registry
    /// insertion time; later snapshots see the same value.
    pub activated_at: DateTime<Utc>,
    /// Opaque per-key payload. Shape is determined by [`Self::key`];
    /// the server's per-key formatter renders it into the LLM's system
    /// message.
    pub data: Value,
    /// Routing target the dispatcher will use when this context is
    /// captured in a [`TurnState`](crate). Never serialized.
    ///
    /// Stored in an `Arc` so the per-turn snapshot can hand the same
    /// origin to every `IncomingCall` without deep-cloning the inner
    /// `BrowserOrigin::page_url` (and similar string-heavy fields) per
    /// tool call.
    pub origin: Arc<Origin>,
}

/// Thread-safe registry of currently-active contexts on the client.
///
/// Backed by a [`DashMap`] keyed on [`ActiveContext::key`]; all access
/// paths are lock-free under contention so the bridge-listener task
/// can write concurrently with `ChatBridge` snapshots without
/// coordination.
#[derive(Debug, Default)]
pub struct ContextRegistry {
    active: DashMap<String, ActiveContext>,
}

impl ContextRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Activate (or re-activate) a context. If an entry with the same
    /// `key` already exists, it is overwritten.
    pub fn activate(&self, ctx: ActiveContext) {
        self.active.insert(ctx.key.clone(), ctx);
    }

    /// Deactivate the context with the given key. No-op if not present.
    pub fn deactivate(&self, key: &str) {
        self.active.remove(key);
    }

    /// Whether a context with the given key is currently active.
    pub fn is_active(&self, key: &str) -> bool {
        self.active.contains_key(key)
    }

    /// Number of currently-active contexts.
    pub fn len(&self) -> usize {
        self.active.len()
    }

    /// Whether the registry has any active contexts.
    pub fn is_empty(&self) -> bool {
        self.active.is_empty()
    }

    /// Snapshot the active set into a freshly allocated `Vec`. Iteration
    /// order is unspecified — callers that need a deterministic order
    /// must sort the result.
    pub fn snapshot(&self) -> Vec<ActiveContext> {
        self.active
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::origin::BrowserOrigin;
    use serde_json::json;

    fn sample_origin() -> Arc<Origin> {
        Arc::new(Origin::Browser(BrowserOrigin {
            process_id: 4242,
            tab_id: 19,
            window_id: Some("win-0".into()),
            page_url: "https://www.youtube.com/watch?v=abc123".into(),
        }))
    }

    fn sample_context(at: DateTime<Utc>) -> ActiveContext {
        ActiveContext {
            key: "youtube::watch_page".into(),
            activated_at: at,
            data: json!({ "video_id": "abc123" }),
            origin: sample_origin(),
        }
    }

    fn ts(secs: i64) -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(secs, 0).expect("valid timestamp")
    }

    #[test]
    fn activate_then_snapshot_returns_entry() {
        let registry = ContextRegistry::new();
        registry.activate(sample_context(ts(100)));

        let snapshot = registry.snapshot();
        assert_eq!(snapshot.len(), 1);
        assert_eq!(snapshot[0].key, "youtube::watch_page");
        assert_eq!(snapshot[0].activated_at, ts(100));
        assert!(registry.is_active("youtube::watch_page"));
        assert_eq!(registry.len(), 1);
        assert!(!registry.is_empty());
    }

    #[test]
    fn re_activating_same_key_overwrites_previous_entry() {
        let registry = ContextRegistry::new();
        registry.activate(sample_context(ts(100)));

        let later = ActiveContext {
            key: "youtube::watch_page".into(),
            activated_at: ts(200),
            data: json!({ "video_id": "def456" }),
            origin: sample_origin(),
        };
        registry.activate(later);

        let snapshot = registry.snapshot();
        assert_eq!(snapshot.len(), 1);
        assert_eq!(snapshot[0].activated_at, ts(200));
        assert_eq!(snapshot[0].data, json!({ "video_id": "def456" }));
    }

    #[test]
    fn deactivate_removes_entry() {
        let registry = ContextRegistry::new();
        registry.activate(sample_context(ts(100)));
        registry.deactivate("youtube::watch_page");

        assert!(!registry.is_active("youtube::watch_page"));
        assert!(registry.is_empty());
        assert_eq!(registry.snapshot().len(), 0);
    }

    #[test]
    fn deactivating_unknown_key_is_no_op() {
        let registry = ContextRegistry::new();
        registry.deactivate("not::present");

        let other = ActiveContext {
            key: "focus::app::vscode".into(),
            ..sample_context(ts(100))
        };
        registry.activate(other);
        registry.deactivate("not::present");
        assert!(registry.is_active("focus::app::vscode"));
    }

    #[test]
    fn distinct_keys_coexist() {
        let registry = ContextRegistry::new();
        registry.activate(sample_context(ts(100)));
        registry.activate(ActiveContext {
            key: "focus::app::vscode".into(),
            ..sample_context(ts(200))
        });

        assert_eq!(registry.len(), 2);
        let mut keys: Vec<_> = registry.snapshot().into_iter().map(|c| c.key).collect();
        keys.sort();
        assert_eq!(keys, vec!["focus::app::vscode", "youtube::watch_page"]);
    }
}
