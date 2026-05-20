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
//!
//! ## `process_id` routing
//!
//! [`BrowserOrigin::process_id`](eurora_tools::BrowserOrigin) is the
//! routing key the dispatcher hands back to the bridge when calling
//! tools. It is **always** taken from the bridge envelope's `app_pid`
//! and never trusted from the extension's payload — a browser extension
//! has no way to read its own host's OS PID, and the native messenger
//! that delivers the event is the single source of truth for the
//! PID-to-session mapping by construction (it registered with that PID
//! and forwards every event on that registration). Stamping the value
//! from the envelope keeps the extension wire shape ergonomic and
//! eliminates an entire class of "extension reported the wrong PID"
//! bugs.

use std::sync::Arc;

use chrono::Utc;
use euro_bridge::{BridgeService, EventFrame, Payload};
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

    /// The bridge envelope carried `app_pid == 0`. The native messenger
    /// always registers with a real OS PID, so a zero here indicates a
    /// bridge bug rather than anything the extension can fix.
    #[error("context event {action} arrived without a valid envelope app_pid")]
    MissingAppPid { action: String },
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
/// `app_pid` is the routing PID the bridge associated with the
/// envelope that delivered this frame — i.e. the OS PID the native
/// messenger registered with. For `CONTEXT_ACTIVATED` frames whose
/// origin variant carries a `process_id` field, that field is
/// overwritten with `app_pid`. The extension never sets it; the
/// envelope is authoritative.
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
/// - `Err(ApplyError)` on a malformed payload or a zero `app_pid` on an
///   activation; the registry is untouched.
pub fn apply_event(
    registry: &ContextRegistry,
    app_pid: u32,
    frame: &EventFrame,
) -> Result<EventOutcome, ApplyError> {
    match frame.action.as_str() {
        CONTEXT_ACTIVATED => {
            if app_pid == 0 {
                return Err(ApplyError::MissingAppPid {
                    action: frame.action.clone(),
                });
            }
            let payload: ActivatePayload = decode_payload(&frame.action, frame.payload.as_ref())?;
            let origin = std::sync::Arc::new(stamp_routing_pid(payload.origin, app_pid));
            registry.activate(ActiveContext {
                key: payload.key,
                activated_at: Utc::now(),
                data: payload.data,
                origin,
            });
            Ok(EventOutcome::Activated)
        }
        CONTEXT_DEACTIVATED => {
            let payload: DeactivatePayload = decode_payload(&frame.action, frame.payload.as_ref())?;
            registry.deactivate(&payload.key);
            Ok(EventOutcome::Deactivated)
        }
        _ => Ok(EventOutcome::Ignored),
    }
}

fn decode_payload<T: for<'de> Deserialize<'de>>(
    action: &str,
    payload: Option<&Payload>,
) -> Result<T, ApplyError> {
    payload
        .ok_or_else(|| ApplyError::MissingPayload {
            action: action.to_owned(),
        })?
        .deserialize()
        .map_err(|source| ApplyError::InvalidJson {
            action: action.to_owned(),
            source,
        })
}

/// Replace the origin's routing PID with the envelope-derived
/// `app_pid`. Every variant present today carries its own `process_id`
/// field; the field is always overwritten regardless of what the
/// extension supplied. A future origin variant that lacks a routing
/// PID falls through unchanged — add an explicit arm if you introduce
/// one that needs stamping.
fn stamp_routing_pid(mut origin: Origin, app_pid: u32) -> Origin {
    match &mut origin {
        Origin::Browser(b) => b.process_id = app_pid,
        Origin::Focused(f) => f.process_id = app_pid,
        Origin::Acp(a) => a.process_id = app_pid,
        _ => {}
    }
    origin
}

/// Spawn the bridge-events listener task.
///
/// The task subscribes to `BridgeService::get_or_init().subscribe_to_events()`
/// and drives [`apply_event`] on every received frame, passing the
/// envelope's `app_pid` as the authoritative routing PID. Lifetime is
/// the process lifetime — the loop exits only when the bridge service
/// closes its channel (i.e. the process is shutting down).
pub fn spawn_context_listener(registry: Arc<ContextRegistry>) {
    tauri::async_runtime::spawn(async move {
        let service = BridgeService::get_or_init();
        let mut events_rx = service.subscribe_to_events();

        loop {
            match events_rx.recv().await {
                Ok((app_pid, frame)) => match apply_event(&registry, app_pid, &frame) {
                    Ok(EventOutcome::Activated) => {
                        tracing::debug!(
                            app_pid,
                            action = %frame.action,
                            "Context activated via bridge event"
                        );
                    }
                    Ok(EventOutcome::Deactivated) => {
                        tracing::debug!(
                            app_pid,
                            action = %frame.action,
                            "Context deactivated via bridge event"
                        );
                    }
                    Ok(EventOutcome::Ignored) => {
                        tracing::trace!(
                            app_pid,
                            action = %frame.action,
                            "Ignoring unrelated bridge event"
                        );
                    }
                    Err(err) => {
                        tracing::warn!(
                            app_pid,
                            error = %err,
                            "Discarding malformed context event",
                        );
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

    const TEST_APP_PID: u32 = 4242;

    /// The shape the extension actually emits: no `process_id` on the
    /// origin (it's stamped on the desktop from the envelope `app_pid`).
    fn browser_payload(key: &str) -> Payload {
        Payload::from_value(&json!({
            "key": key,
            "data": {
                "video_id": "abc123",
                "title": "Tokio async patterns",
                "page_url": "https://www.youtube.com/watch?v=abc123",
            },
            "origin": {
                "Browser": {
                    "tab_id": 19,
                    "window_id": "win-0",
                    "page_url": "https://www.youtube.com/watch?v=abc123",
                }
            }
        }))
        .expect("encode browser payload")
    }

    fn deactivate_payload(key: &str) -> Payload {
        Payload::from_value(&json!({ "key": key })).expect("encode deactivate payload")
    }

    #[test]
    fn activated_frame_populates_registry_with_envelope_pid() {
        let registry = ContextRegistry::new();
        let frame = EventFrame {
            action: CONTEXT_ACTIVATED.into(),
            payload: Some(browser_payload("youtube::watch_page")),
        };

        let outcome = apply_event(&registry, TEST_APP_PID, &frame).expect("apply");
        assert_eq!(outcome, EventOutcome::Activated);

        let snapshot = registry.snapshot();
        assert_eq!(snapshot.len(), 1);
        let entry = &snapshot[0];
        assert_eq!(entry.key, "youtube::watch_page");
        match entry.origin.as_ref() {
            Origin::Browser(b) => {
                assert_eq!(b.process_id, TEST_APP_PID);
                assert_eq!(b.tab_id, 19);
                assert_eq!(b.window_id.as_deref(), Some("win-0"));
                assert_eq!(b.page_url, "https://www.youtube.com/watch?v=abc123");
            }
            other => panic!("expected Browser origin, got {other:?}"),
        }
        assert_eq!(entry.data["video_id"], json!("abc123"));
    }

    #[test]
    fn envelope_pid_overrides_payload_process_id() {
        // Even if the extension misbehaves and supplies its own
        // `process_id`, the envelope is authoritative.
        let registry = ContextRegistry::new();
        let payload = Payload::from_value(&json!({
            "key": "youtube::watch_page",
            "data": {},
            "origin": {
                "Browser": {
                    "process_id": 1,
                    "tab_id": 7,
                    "window_id": null,
                    "page_url": "https://www.youtube.com/watch?v=xyz",
                }
            }
        }))
        .expect("encode payload");
        apply_event(
            &registry,
            TEST_APP_PID,
            &EventFrame {
                action: CONTEXT_ACTIVATED.into(),
                payload: Some(payload),
            },
        )
        .expect("apply");

        match registry.snapshot()[0].origin.as_ref() {
            Origin::Browser(b) => assert_eq!(b.process_id, TEST_APP_PID),
            other => panic!("expected Browser origin, got {other:?}"),
        }
    }

    #[test]
    fn deactivated_frame_removes_entry() {
        let registry = ContextRegistry::new();
        apply_event(
            &registry,
            TEST_APP_PID,
            &EventFrame {
                action: CONTEXT_ACTIVATED.into(),
                payload: Some(browser_payload("youtube::watch_page")),
            },
        )
        .expect("activate");

        let outcome = apply_event(
            &registry,
            TEST_APP_PID,
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
        let err = apply_event(&registry, TEST_APP_PID, &frame).expect_err("missing payload");
        assert!(matches!(err, ApplyError::MissingPayload { .. }));
        assert!(registry.is_empty());
    }

    #[test]
    fn malformed_json_is_rejected() {
        let registry = ContextRegistry::new();
        // Structurally valid JSON, but missing the `key` / `data` /
        // `origin` fields the activate path requires — exercises the
        // `InvalidJson` arm without bypassing the `Payload` newtype.
        let frame = EventFrame {
            action: CONTEXT_ACTIVATED.into(),
            payload: Some(Payload::from_value(&json!({"unexpected": true})).unwrap()),
        };
        let err = apply_event(&registry, TEST_APP_PID, &frame).expect_err("malformed json");
        assert!(matches!(err, ApplyError::InvalidJson { .. }));
        assert!(registry.is_empty());
    }

    #[test]
    fn zero_envelope_pid_is_rejected_on_activation() {
        let registry = ContextRegistry::new();
        let frame = EventFrame {
            action: CONTEXT_ACTIVATED.into(),
            payload: Some(browser_payload("youtube::watch_page")),
        };
        let err = apply_event(&registry, 0, &frame).expect_err("zero pid");
        assert!(matches!(err, ApplyError::MissingAppPid { .. }));
        assert!(registry.is_empty());
    }

    #[test]
    fn deactivation_does_not_require_envelope_pid() {
        // Deactivation only carries a key — there's no routing to stamp,
        // so the envelope PID is not consulted.
        let registry = ContextRegistry::new();
        let outcome = apply_event(
            &registry,
            0,
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
    fn unknown_action_is_ignored() {
        let registry = ContextRegistry::new();
        let outcome = apply_event(
            &registry,
            TEST_APP_PID,
            &EventFrame {
                action: "TAB_ACTIVATED".into(),
                payload: Some(Payload::from_value(&json!({})).unwrap()),
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
            TEST_APP_PID,
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
            TEST_APP_PID,
            &EventFrame {
                action: CONTEXT_ACTIVATED.into(),
                payload: Some(browser_payload("youtube::watch_page")),
            },
        )
        .expect("activate first");

        let updated = Payload::from_value(&json!({
            "key": "youtube::watch_page",
            "data": { "video_id": "def456" },
            "origin": {
                "Browser": {
                    "tab_id": 22,
                    "window_id": null,
                    "page_url": "https://www.youtube.com/watch?v=def456",
                }
            }
        }))
        .expect("encode payload");
        // Re-activation from a different host registration (new browser
        // session) carries a different envelope PID — verifies the
        // overwrite path stamps the latest value.
        let new_pid: u32 = 9999;
        apply_event(
            &registry,
            new_pid,
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
        match entry.origin.as_ref() {
            Origin::Browser(b) => {
                assert_eq!(b.process_id, new_pid);
                assert_eq!(b.tab_id, 22);
                assert!(b.window_id.is_none());
            }
            other => panic!("expected Browser origin, got {other:?}"),
        }
    }
}
