//! Bridge listener that feeds the client [`ContextRegistry`].
//!
//! The browser extension (and, in later phases, the OS focus tracker)
//! publishes `CONTEXT_ACTIVATED` / `CONTEXT_DEACTIVATED` notifications
//! as bridge [`EventFrame`]s. This module is the seam: it subscribes to
//! `BridgeService::subscribe_to_events()`, decodes each frame's JSON
//! payload, and translates the result into [`ContextRegistry`] calls.
//!
//! The registry type itself lives in `eurora-tools` so non-Tauri
//! consumers (e.g. `ChatBridge` in `euro-thread`) can snapshot it. The
//! glue here owns the bridge-protocol dependency in one place.

use std::sync::Arc;

use chrono::Utc;
use euro_bridge::{BridgeService, EventFrame};
use eurora_tools::{ActiveContext, ContextRegistry, Origin};
use serde::Deserialize;
use serde_json::Value;
use thiserror::Error;
use tokio::sync::broadcast::error::RecvError;

/// Bridge `EventFrame.action` value emitted when an observer wants the
/// desktop to start tracking a context.
pub const CONTEXT_ACTIVATED: &str = "CONTEXT_ACTIVATED";

/// Bridge `EventFrame.action` value emitted when an observer wants the
/// desktop to stop tracking a previously activated context.
pub const CONTEXT_DEACTIVATED: &str = "CONTEXT_DEACTIVATED";

/// Outcome of applying a single [`EventFrame`] to a [`ContextRegistry`].
///
/// Exposed primarily for tests; the production listener only logs the
/// variant for diagnostics.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum EventOutcome {
    /// The frame was a `CONTEXT_ACTIVATED` and the registry was mutated.
    Activated,
    /// The frame was a `CONTEXT_DEACTIVATED` and the registry was
    /// mutated (or the key was absent, which is a no-op).
    Deactivated,
    /// The frame's action was neither known constant. The registry was
    /// left unchanged. Forward-compatible with future observer
    /// additions.
    Ignored,
}

/// Failure modes when decoding a context [`EventFrame`].
///
/// All variants leave the registry untouched.
#[derive(Debug, Error)]
pub enum ApplyError {
    /// The event carried no payload at all.
    #[error("context event {action} arrived with no payload")]
    MissingPayload { action: String },

    /// The payload string failed to parse as JSON in the expected shape.
    #[error("context event {action} payload was not valid JSON: {source}")]
    InvalidJson {
        action: String,
        #[source]
        source: serde_json::Error,
    },

    /// The payload decoded but the embedded [`Origin`] failed a
    /// semantic check (e.g. a `BrowserOrigin` with `process_id == 0`,
    /// which the extension is supposed to always populate).
    #[error("context event {action} carried an invalid origin: {reason}")]
    InvalidOrigin {
        action: String,
        reason: &'static str,
    },
}

/// Deserialization mirror of the JSON payload that the extension
/// embeds inside an activation [`EventFrame`].
///
/// Kept private — callers go through [`apply_event`].
#[derive(Debug, Deserialize)]
struct ActivatePayload {
    key: String,
    #[serde(default)]
    data: Value,
    origin: Origin,
}

/// Deserialization mirror of the JSON payload that the extension
/// embeds inside a deactivation [`EventFrame`]. Only the key is needed
/// — the registry is keyed on it.
#[derive(Debug, Deserialize)]
struct DeactivatePayload {
    key: String,
}

/// Apply one bridge event to the registry.
///
/// Pure with respect to time *except* that `CONTEXT_ACTIVATED` stamps
/// the resulting [`ActiveContext::activated_at`] with [`Utc::now`]. The
/// extension does not send a timestamp — the desktop is the authority
/// on when a context entered the active set.
///
/// Returns:
///
/// - `Ok(EventOutcome::Activated | Deactivated)` on a successful mutate.
/// - `Ok(EventOutcome::Ignored)` for unrelated actions (forward compat).
/// - `Err(ApplyError)` on a malformed payload; the registry is untouched.
pub fn apply_event(
    registry: &ContextRegistry,
    frame: &EventFrame,
) -> Result<EventOutcome, ApplyError> {
    match frame.action.as_str() {
        CONTEXT_ACTIVATED => {
            let payload: ActivatePayload = decode_payload(&frame.action, frame.payload.as_deref())?;
            validate_origin(&frame.action, &payload.origin)?;
            registry.activate(ActiveContext {
                key: payload.key,
                activated_at: Utc::now(),
                data: payload.data,
                origin: payload.origin,
            });
            Ok(EventOutcome::Activated)
        }
        CONTEXT_DEACTIVATED => {
            let payload: DeactivatePayload =
                decode_payload(&frame.action, frame.payload.as_deref())?;
            registry.deactivate(&payload.key);
            Ok(EventOutcome::Deactivated)
        }
        _ => Ok(EventOutcome::Ignored),
    }
}

fn decode_payload<T: for<'de> Deserialize<'de>>(
    action: &str,
    payload: Option<&str>,
) -> Result<T, ApplyError> {
    let raw = payload.ok_or_else(|| ApplyError::MissingPayload {
        action: action.to_owned(),
    })?;
    serde_json::from_str(raw).map_err(|source| ApplyError::InvalidJson {
        action: action.to_owned(),
        source,
    })
}

fn validate_origin(action: &str, origin: &Origin) -> Result<(), ApplyError> {
    match origin {
        Origin::Browser(b) if b.process_id == 0 => Err(ApplyError::InvalidOrigin {
            action: action.to_owned(),
            reason: "BrowserOrigin.process_id must be non-zero",
        }),
        Origin::Focused(f) if f.process_id == 0 => Err(ApplyError::InvalidOrigin {
            action: action.to_owned(),
            reason: "FocusedOrigin.process_id must be non-zero",
        }),
        Origin::Acp(a) if a.process_id == 0 => Err(ApplyError::InvalidOrigin {
            action: action.to_owned(),
            reason: "AcpOrigin.process_id must be non-zero",
        }),
        _ => Ok(()),
    }
}

/// Spawn the bridge-events listener task.
///
/// The task subscribes to `BridgeService::get_or_init().subscribe_to_events()`
/// and drives [`apply_event`] on every received frame. Lifetime is the
/// process lifetime — the loop exits only when the bridge service
/// closes its channel (i.e. the process is shutting down).
///
/// `app_pid` from the bridge envelope is intentionally ignored: the
/// authoritative routing information is the `process_id` inside the
/// payload's [`Origin`]. Conflating the two would couple browser
/// activation events to the registering native-messaging session,
/// which is the wrong dimension.
pub fn spawn_context_listener(registry: Arc<ContextRegistry>) {
    tauri::async_runtime::spawn(async move {
        let service = BridgeService::get_or_init();
        let mut events_rx = service.subscribe_to_events();

        loop {
            match events_rx.recv().await {
                Ok((_pid, frame)) => match apply_event(&registry, &frame) {
                    Ok(EventOutcome::Activated) => {
                        tracing::debug!(
                            action = %frame.action,
                            "Context activated via bridge event"
                        );
                    }
                    Ok(EventOutcome::Deactivated) => {
                        tracing::debug!(
                            action = %frame.action,
                            "Context deactivated via bridge event"
                        );
                    }
                    Ok(EventOutcome::Ignored) => {
                        tracing::trace!(
                            action = %frame.action,
                            "Ignoring unrelated bridge event"
                        );
                    }
                    Err(err) => {
                        tracing::warn!(error = %err, "Discarding malformed context event");
                    }
                },
                Err(RecvError::Lagged(n)) => {
                    tracing::warn!("Context event subscription lagged by {n} events");
                }
                Err(RecvError::Closed) => break,
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn browser_payload(key: &str, process_id: u32) -> String {
        json!({
            "key": key,
            "data": {
                "video_id": "abc123",
                "title": "Tokio async patterns",
                "channel": "ThePrimeagen",
                "duration_seconds": 1122,
                "approximate_timestamp_seconds": 153,
            },
            "origin": {
                "Browser": {
                    "process_id": process_id,
                    "tab_id": 19,
                    "window_id": "win-0",
                    "page_url": "https://www.youtube.com/watch?v=abc123",
                }
            }
        })
        .to_string()
    }

    fn deactivate_payload(key: &str) -> String {
        json!({ "key": key }).to_string()
    }

    #[test]
    fn activated_frame_populates_registry() {
        let registry = ContextRegistry::new();
        let frame = EventFrame {
            action: CONTEXT_ACTIVATED.into(),
            payload: Some(browser_payload("youtube::watch_page", 4242)),
        };

        let outcome = apply_event(&registry, &frame).expect("apply");
        assert_eq!(outcome, EventOutcome::Activated);

        let snapshot = registry.snapshot();
        assert_eq!(snapshot.len(), 1);
        let entry = &snapshot[0];
        assert_eq!(entry.key, "youtube::watch_page");
        match &entry.origin {
            Origin::Browser(b) => {
                assert_eq!(b.process_id, 4242);
                assert_eq!(b.tab_id, 19);
                assert_eq!(b.window_id.as_deref(), Some("win-0"));
                assert_eq!(b.page_url, "https://www.youtube.com/watch?v=abc123");
            }
            other => panic!("expected Browser origin, got {other:?}"),
        }
        assert_eq!(entry.data["video_id"], json!("abc123"));
    }

    #[test]
    fn deactivated_frame_removes_entry() {
        let registry = ContextRegistry::new();
        apply_event(
            &registry,
            &EventFrame {
                action: CONTEXT_ACTIVATED.into(),
                payload: Some(browser_payload("youtube::watch_page", 4242)),
            },
        )
        .expect("activate");

        let outcome = apply_event(
            &registry,
            &EventFrame {
                action: CONTEXT_DEACTIVATED.into(),
                payload: Some(deactivate_payload("youtube::watch_page")),
            },
        )
        .expect("deactivate");
        assert_eq!(outcome, EventOutcome::Deactivated);
        assert!(registry.is_empty());
    }

    #[test]
    fn missing_payload_is_rejected() {
        let registry = ContextRegistry::new();
        let frame = EventFrame {
            action: CONTEXT_ACTIVATED.into(),
            payload: None,
        };
        let err = apply_event(&registry, &frame).expect_err("missing payload");
        assert!(matches!(err, ApplyError::MissingPayload { .. }));
        assert!(registry.is_empty());
    }

    #[test]
    fn malformed_json_is_rejected() {
        let registry = ContextRegistry::new();
        let frame = EventFrame {
            action: CONTEXT_ACTIVATED.into(),
            payload: Some("{not json".into()),
        };
        let err = apply_event(&registry, &frame).expect_err("malformed json");
        assert!(matches!(err, ApplyError::InvalidJson { .. }));
        assert!(registry.is_empty());
    }

    #[test]
    fn zero_browser_process_id_is_rejected() {
        let registry = ContextRegistry::new();
        let frame = EventFrame {
            action: CONTEXT_ACTIVATED.into(),
            payload: Some(browser_payload("youtube::watch_page", 0)),
        };
        let err = apply_event(&registry, &frame).expect_err("zero pid");
        assert!(matches!(err, ApplyError::InvalidOrigin { .. }));
        assert!(registry.is_empty());
    }

    #[test]
    fn unknown_action_is_ignored() {
        let registry = ContextRegistry::new();
        let outcome = apply_event(
            &registry,
            &EventFrame {
                action: "TAB_ACTIVATED".into(),
                payload: Some("{}".into()),
            },
        )
        .expect("ignored");
        assert_eq!(outcome, EventOutcome::Ignored);
        assert!(registry.is_empty());
    }

    #[test]
    fn deactivating_unknown_key_is_ok() {
        let registry = ContextRegistry::new();
        let outcome = apply_event(
            &registry,
            &EventFrame {
                action: CONTEXT_DEACTIVATED.into(),
                payload: Some(deactivate_payload("not::present")),
            },
        )
        .expect("deactivate");
        assert_eq!(outcome, EventOutcome::Deactivated);
        assert!(registry.is_empty());
    }

    #[test]
    fn re_activation_overwrites_previous_entry() {
        let registry = ContextRegistry::new();
        apply_event(
            &registry,
            &EventFrame {
                action: CONTEXT_ACTIVATED.into(),
                payload: Some(browser_payload("youtube::watch_page", 4242)),
            },
        )
        .expect("activate first");

        let updated = json!({
            "key": "youtube::watch_page",
            "data": { "video_id": "def456" },
            "origin": {
                "Browser": {
                    "process_id": 9999,
                    "tab_id": 22,
                    "window_id": null,
                    "page_url": "https://www.youtube.com/watch?v=def456",
                }
            }
        })
        .to_string();
        apply_event(
            &registry,
            &EventFrame {
                action: CONTEXT_ACTIVATED.into(),
                payload: Some(updated),
            },
        )
        .expect("activate again");

        let snapshot = registry.snapshot();
        assert_eq!(snapshot.len(), 1);
        let entry = &snapshot[0];
        assert_eq!(entry.data["video_id"], json!("def456"));
        match &entry.origin {
            Origin::Browser(b) => {
                assert_eq!(b.process_id, 9999);
                assert_eq!(b.tab_id, 22);
                assert!(b.window_id.is_none());
            }
            other => panic!("expected Browser origin, got {other:?}"),
        }
    }
}
