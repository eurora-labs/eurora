//! Process-wide telemetry lifecycle for the desktop app.
//!
//! Sentry is the only SDK initialized natively — PostHog runs in the
//! frontend (`posthog-js`) where it can observe UI navigation. This module
//! exposes:
//!
//! * [`init`] — called once from `main()` before the Tauri builder, gated on
//!   the user's last-saved telemetry consent.
//! * [`Controller`] — a small handle managed as Tauri state so the
//!   `system.reinit_telemetry` procedure can teardown / rebuild Sentry
//!   when the user toggles consent at runtime.
//!
//! Compile-time keys come from `build.rs` via `cargo:rustc-env`. The
//! bake is fail-closed: if `EURORA_DESKTOP_SENTRY_DSN` is non-empty,
//! `EURORA_RELEASE_CHANNEL` and `RELEASE_VERSION` must be non-empty
//! too, so the runtime never has to defend against half-configured
//! telemetry. A missing DSN is treated as "telemetry disabled" so dev
//! builds don't accidentally ship events to a stale project.

use std::sync::Mutex;

use euro_settings::TelemetrySettings;
use sentry::ClientInitGuard;

/// Compile-time DSN baked from `EURORA_DESKTOP_SENTRY_DSN`. Empty when
/// the build was produced without a DSN (every dev build, plus any
/// release variant we explicitly want to keep dark).
const SENTRY_DSN: &str = env!("EURORA_DESKTOP_SENTRY_DSN");

/// Compile-time release channel (`dev` / `nightly` / `release`). The
/// build script enforces non-empty whenever `SENTRY_DSN` is set.
const RELEASE_CHANNEL: &str = env!("EURORA_RELEASE_CHANNEL");

/// Compile-time release version (e.g. `0.5.42`). Used as the Sentry
/// `release` tag so events from a given build are bucketed correctly.
/// The build script enforces non-empty whenever `SENTRY_DSN` is set.
pub const RELEASE_VERSION: &str = env!("RELEASE_VERSION");

/// Owns the live Sentry guard. The guard's `Drop` impl flushes pending
/// events, so the controller is held in `tauri::Manager` state for the
/// lifetime of the process. Re-init replaces the guard atomically.
///
/// `reapply` must be called from a tokio context: the previous guard is
/// dropped on `spawn_blocking` so the multi-second flush doesn't stall
/// the executor thread that handled the IPC.
pub struct Controller {
    guard: Mutex<Option<ClientInitGuard>>,
}

impl Controller {
    pub fn new(initial: Option<ClientInitGuard>) -> Self {
        Self {
            guard: Mutex::new(initial),
        }
    }

    /// `true` when a Sentry client is currently active. The lock is
    /// held only long enough to peek; callers shouldn't make decisions
    /// off this without expecting a race against [`reapply`].
    pub fn is_active(&self) -> bool {
        self.guard
            .lock()
            .expect("telemetry guard mutex poisoned")
            .is_some()
    }

    /// Apply a freshly loaded `TelemetrySettings`: drop the old guard
    /// (flushing any buffered events) and start a new client when the
    /// user consents to error reporting.
    ///
    /// The previous guard is dropped on `spawn_blocking` because
    /// `ClientInitGuard::Drop` blocks until pending events flush
    /// (default ~2s). Doing that on the calling tokio executor thread
    /// would stall every other in-flight IPC.
    pub fn reapply(&self, settings: &TelemetrySettings) {
        let new_guard = build_guard(settings);
        let active = new_guard.is_some();
        let old = {
            let mut slot = self.guard.lock().expect("telemetry guard mutex poisoned");
            std::mem::replace(&mut *slot, new_guard)
        };
        if let Some(old) = old {
            tokio::task::spawn_blocking(move || drop(old));
        }
        if active {
            register_panic_hook();
        }
        apply_user_scope(settings);
    }
}

/// Build a Sentry client guard from the current settings. Returns
/// `None` when telemetry is disabled, the DSN is absent (dev builds),
/// or — defensively — the channel is empty in a DSN-bearing build
/// (the build script already prevents this, so the runtime check is a
/// belt-and-braces guard against a future regression).
pub fn build_guard(settings: &TelemetrySettings) -> Option<ClientInitGuard> {
    if !settings.wants_errors() {
        return None;
    }
    if SENTRY_DSN.is_empty() {
        tracing::debug!("EURORA_DESKTOP_SENTRY_DSN unset; Sentry disabled");
        return None;
    }
    if RELEASE_CHANNEL.is_empty() || RELEASE_VERSION.is_empty() {
        tracing::error!(
            release_channel_set = !RELEASE_CHANNEL.is_empty(),
            release_version_set = !RELEASE_VERSION.is_empty(),
            "EURORA_DESKTOP_SENTRY_DSN is set but channel/version is empty; \
             refusing to initialize Sentry against an unidentifiable build"
        );
        return None;
    }

    let options = sentry::ClientOptions {
        release: Some(RELEASE_VERSION.into()),
        environment: Some(RELEASE_CHANNEL.into()),
        send_default_pii: false,
        attach_stacktrace: true,
        traces_sample_rate: 0.0,
        before_send: Some(std::sync::Arc::new(scrub_event)),
        ..Default::default()
    };

    Some(sentry::init((SENTRY_DSN, options)))
}

/// Single entry point used at startup. Builds the guard, registers the
/// panic hook if telemetry is active, and applies the persisted
/// distinct id as the Sentry user.
pub fn init(settings: &TelemetrySettings) -> Controller {
    let controller = Controller::new(build_guard(settings));
    if controller.is_active() {
        register_panic_hook();
        apply_user_scope(settings);
    }
    controller
}

/// `Once`-guarded panic hook installer. Forwards to the previous hook
/// after Sentry observes the panic so the default abort/print still
/// runs. Safe to call repeatedly: the second-and-onwards calls are
/// no-ops, which is what lets [`Controller::reapply`] install the hook
/// the first time the user opts in mid-session.
fn register_panic_hook() {
    static HOOK_INSTALLED: std::sync::Once = std::sync::Once::new();
    HOOK_INSTALLED.call_once(|| {
        let next = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            sentry::integrations::panic::panic_handler(info);
            next(info);
        }));
    });
}

fn apply_user_scope(settings: &TelemetrySettings) {
    let id = settings.distinct_id.clone();
    sentry::configure_scope(|scope| {
        scope.set_user(id.map(|id| sentry::User {
            id: Some(id),
            ..Default::default()
        }));
    });
}

/// Strip filesystem paths that include the user's home directory from
/// every string-bearing field of an outgoing event before it leaves the
/// machine. Sentry already redacts most server-side data with
/// `send_default_pii=false`, but local file paths are emitted verbatim
/// from `tracing`, the panic hook, and `debug-images`.
///
/// This walks: top-level `message`, every breadcrumb's `message` and
/// `data` map, every exception's `value` and both stacktraces (frames'
/// `filename` / `abs_path` / `context_line` / `pre_context` /
/// `post_context` / `vars`), the top-level `stacktrace`, and `extra`.
/// Anything `sentry-tracing` puts into `breadcrumb.data` (e.g.
/// `tracing::error!(path = ?some_pathbuf)`) is therefore covered.
fn scrub_event(
    mut event: sentry::protocol::Event<'static>,
) -> Option<sentry::protocol::Event<'static>> {
    let Some(home) = dirs::home_dir() else {
        return Some(event);
    };
    let home = home.to_string_lossy().into_owned();
    if home.is_empty() {
        return Some(event);
    }

    if let Some(message) = event.message.as_mut() {
        scrub_string(message, &home);
    }
    for breadcrumb in &mut event.breadcrumbs {
        if let Some(message) = breadcrumb.message.as_mut() {
            scrub_string(message, &home);
        }
        scrub_value_map(&mut breadcrumb.data, &home);
    }
    for exception in &mut event.exception {
        if let Some(value) = exception.value.as_mut() {
            scrub_string(value, &home);
        }
        if let Some(stacktrace) = exception.stacktrace.as_mut() {
            scrub_stacktrace(stacktrace, &home);
        }
        if let Some(stacktrace) = exception.raw_stacktrace.as_mut() {
            scrub_stacktrace(stacktrace, &home);
        }
    }
    if let Some(stacktrace) = event.stacktrace.as_mut() {
        scrub_stacktrace(stacktrace, &home);
    }
    scrub_value_map(&mut event.extra, &home);
    Some(event)
}

fn scrub_string(s: &mut String, home: &str) {
    // `String::replace("", _)` inserts the replacement between every
    // character — guard against the degenerate case so a malformed
    // call site can't corrupt event data.
    if home.is_empty() || !s.contains(home) {
        return;
    }
    *s = s.replace(home, "~");
}

fn scrub_stacktrace(stacktrace: &mut sentry::protocol::Stacktrace, home: &str) {
    for frame in &mut stacktrace.frames {
        if let Some(s) = frame.filename.as_mut() {
            scrub_string(s, home);
        }
        if let Some(s) = frame.abs_path.as_mut() {
            scrub_string(s, home);
        }
        if let Some(s) = frame.context_line.as_mut() {
            scrub_string(s, home);
        }
        for line in &mut frame.pre_context {
            scrub_string(line, home);
        }
        for line in &mut frame.post_context {
            scrub_string(line, home);
        }
        scrub_value_map(&mut frame.vars, home);
    }
}

fn scrub_value_map(map: &mut sentry::protocol::Map<String, serde_json::Value>, home: &str) {
    for value in map.values_mut() {
        scrub_value(value, home);
    }
}

fn scrub_value(value: &mut serde_json::Value, home: &str) {
    match value {
        serde_json::Value::String(s) => scrub_string(s, home),
        serde_json::Value::Array(items) => items.iter_mut().for_each(|v| scrub_value(v, home)),
        serde_json::Value::Object(map) => map.values_mut().for_each(|v| scrub_value(v, home)),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sentry::protocol::{Breadcrumb, Event, Exception, Frame, Stacktrace};
    use serde_json::json;

    /// Build a fixture event whose every string-bearing field carries
    /// `/home/test` somewhere, so a single scrub call can prove
    /// coverage of all the paths we care about.
    fn fixture_event() -> Event<'static> {
        let frame = Frame {
            filename: Some("/home/test/src/lib.rs".to_owned()),
            abs_path: Some("/home/test/src/lib.rs".to_owned()),
            context_line: Some("    let path = \"/home/test/db\";".to_owned()),
            pre_context: vec!["// /home/test/comment".to_owned()],
            post_context: vec!["// trailing /home/test".to_owned()],
            vars: [("path".to_owned(), json!("/home/test/db"))]
                .into_iter()
                .collect(),
            ..Default::default()
        };
        let stacktrace = Stacktrace {
            frames: vec![frame],
            ..Default::default()
        };
        let exception = Exception {
            ty: "Panic".to_owned(),
            value: Some("crashed reading /home/test/secrets".to_owned()),
            stacktrace: Some(stacktrace.clone()),
            raw_stacktrace: Some(stacktrace),
            ..Default::default()
        };
        let breadcrumb = Breadcrumb {
            message: Some("opened /home/test/file".to_owned()),
            data: [
                ("path".to_owned(), json!("/home/test/db")),
                (
                    "nested".to_owned(),
                    json!({"deeper": ["/home/test/inside", 42]}),
                ),
            ]
            .into_iter()
            .collect(),
            ..Default::default()
        };

        Event {
            message: Some("top-level /home/test/here".to_owned()),
            breadcrumbs: vec![breadcrumb].into(),
            exception: vec![exception].into(),
            extra: [("path".to_owned(), json!("/home/test/extra"))]
                .into_iter()
                .collect(),
            ..Default::default()
        }
    }

    fn scrub_with_home(event: Event<'static>, home: &str) -> Event<'static> {
        let mut event = event;
        if let Some(message) = event.message.as_mut() {
            scrub_string(message, home);
        }
        for breadcrumb in &mut event.breadcrumbs {
            if let Some(message) = breadcrumb.message.as_mut() {
                scrub_string(message, home);
            }
            scrub_value_map(&mut breadcrumb.data, home);
        }
        for exception in &mut event.exception {
            if let Some(value) = exception.value.as_mut() {
                scrub_string(value, home);
            }
            if let Some(stacktrace) = exception.stacktrace.as_mut() {
                scrub_stacktrace(stacktrace, home);
            }
            if let Some(stacktrace) = exception.raw_stacktrace.as_mut() {
                scrub_stacktrace(stacktrace, home);
            }
        }
        if let Some(stacktrace) = event.stacktrace.as_mut() {
            scrub_stacktrace(stacktrace, home);
        }
        scrub_value_map(&mut event.extra, home);
        event
    }

    fn assert_no_home(event: &Event<'static>, home: &str) {
        let serialized = serde_json::to_string(event).unwrap();
        assert!(
            !serialized.contains(home),
            "home path leaked through serialized event: {serialized}"
        );
    }

    #[test]
    fn scrubs_every_string_field() {
        let scrubbed = scrub_with_home(fixture_event(), "/home/test");
        assert_no_home(&scrubbed, "/home/test");
        // Spot-check the replacement marker is present where we expect.
        assert!(
            scrubbed.message.as_deref().unwrap().contains("~/here"),
            "message should be rewritten with ~",
        );
    }

    #[test]
    fn leaves_unrelated_strings_alone() {
        let mut event = Event::<'static> {
            message: Some("nothing to redact here".to_owned()),
            ..Default::default()
        };
        let before = event.message.clone();
        if let Some(message) = event.message.as_mut() {
            scrub_string(message, "/home/test");
        }
        assert_eq!(event.message, before);
    }

    #[test]
    fn handles_deeply_nested_breadcrumb_data() {
        let scrubbed = scrub_with_home(fixture_event(), "/home/test");
        let breadcrumb = scrubbed.breadcrumbs.first().unwrap();
        let nested = &breadcrumb.data["nested"]["deeper"][0];
        assert_eq!(nested.as_str(), Some("~/inside"));
    }

    #[test]
    fn empty_home_is_a_no_op() {
        let mut s = "/home/test/foo".to_owned();
        scrub_string(&mut s, "");
        assert_eq!(s, "/home/test/foo");
    }
}
