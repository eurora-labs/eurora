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
//! Compile-time keys come from `build.rs` via `cargo:rustc-env`. A missing
//! DSN is treated as "telemetry disabled" so dev builds don't accidentally
//! ship events to a stale project.

use std::sync::Mutex;

use euro_settings::TelemetrySettings;
use sentry::ClientInitGuard;

const SENTRY_DSN: &str = env!("EURORA_DESKTOP_SENTRY_DSN");
const RELEASE_CHANNEL: &str = env!("EURORA_RELEASE_CHANNEL");

/// Owns the live Sentry guard. The guard's `Drop` impl flushes pending
/// events, so the controller is held in `tauri::Manager` state for the
/// lifetime of the process. Re-init replaces the guard atomically.
pub struct Controller {
    guard: Mutex<Option<ClientInitGuard>>,
}

impl Controller {
    pub fn new(initial: Option<ClientInitGuard>) -> Self {
        Self {
            guard: Mutex::new(initial),
        }
    }

    /// Apply a freshly loaded `TelemetrySettings`: drop the old guard
    /// (flushing any buffered events) and start a new client when the
    /// user consents to error reporting.
    pub fn reapply(&self, settings: &TelemetrySettings) {
        let new_guard = build_guard(settings);
        let mut slot = self.guard.lock().expect("telemetry guard mutex poisoned");
        *slot = new_guard;
        apply_user_scope(settings);
    }
}

/// Build a Sentry client guard from the current settings. Returns `None`
/// when telemetry is disabled, the DSN is absent (dev builds), or the DSN
/// fails to parse.
pub fn build_guard(settings: &TelemetrySettings) -> Option<ClientInitGuard> {
    if !settings.wants_errors() {
        return None;
    }
    if SENTRY_DSN.is_empty() {
        tracing::debug!("EURORA_DESKTOP_SENTRY_DSN unset; Sentry disabled");
        return None;
    }

    let environment = if RELEASE_CHANNEL.is_empty() {
        "dev".to_owned()
    } else {
        RELEASE_CHANNEL.to_owned()
    };

    let options = sentry::ClientOptions {
        release: sentry::release_name!(),
        environment: Some(environment.into()),
        send_default_pii: false,
        attach_stacktrace: true,
        traces_sample_rate: 0.0,
        before_send: Some(std::sync::Arc::new(scrub_event)),
        ..Default::default()
    };

    Some(sentry::init((SENTRY_DSN, options)))
}

/// Single entry point used at startup. Loads settings without any
/// filesystem side effects, builds the guard, registers the panic hook,
/// and applies the persisted distinct id as the Sentry user.
pub fn init(settings: &TelemetrySettings) -> Controller {
    let guard = build_guard(settings);
    if guard.is_some() {
        register_panic_hook();
        apply_user_scope(settings);
    }
    Controller::new(guard)
}

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
/// breadcrumbs and log messages before they leave the machine. Sentry
/// already redacts most server-side data with `send_default_pii=false`,
/// but local file paths are emitted verbatim from `tracing` and the
/// panic hook.
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

    for breadcrumb in &mut event.breadcrumbs {
        if let Some(message) = breadcrumb.message.as_mut() {
            *message = message.replace(&home, "~");
        }
    }
    if let Some(message) = event.message.as_mut() {
        *message = message.replace(&home, "~");
    }
    Some(event)
}
