//! Sentry client lifecycle: build a guard from current settings,
//! install the panic hook, manage runtime consent toggles.

use std::sync::Mutex;

use euro_settings::TelemetrySettings;
use sentry::ClientInitGuard;

use crate::{RELEASE_CHANNEL, RELEASE_VERSION, SENTRY_DSN, scrub};

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
    /// Single entry point used at startup. Builds the guard, registers
    /// the panic hook if telemetry is active, and applies the persisted
    /// distinct id as the Sentry user.
    #[must_use]
    pub fn init(settings: &TelemetrySettings) -> Self {
        let controller = Self {
            guard: Mutex::new(build_guard(settings)),
        };
        if controller.is_active() {
            register_panic_hook();
            apply_user_scope(settings);
        }
        controller
    }

    /// `true` when a Sentry client is currently active. The lock is
    /// held only long enough to peek; callers shouldn't make decisions
    /// off this without expecting a race against [`reapply`].
    #[must_use]
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
fn build_guard(settings: &TelemetrySettings) -> Option<ClientInitGuard> {
    if !settings.wants_errors() {
        return None;
    }
    if SENTRY_DSN.is_empty() {
        tracing::debug!("EURORA_SENTRY_DSN unset; Sentry disabled");
        return None;
    }
    if RELEASE_CHANNEL.is_empty() || RELEASE_VERSION.is_empty() {
        tracing::error!(
            release_channel_set = !RELEASE_CHANNEL.is_empty(),
            release_version_set = !RELEASE_VERSION.is_empty(),
            "EURORA_SENTRY_DSN is set but channel/version is empty; \
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
        before_send: Some(std::sync::Arc::new(scrub::scrub_event)),
        ..Default::default()
    };

    Some(sentry::init((SENTRY_DSN, options)))
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

#[cfg(test)]
mod tests {
    use super::*;

    /// `Controller::init` must produce an inactive controller whenever
    /// the user hasn't consented, regardless of whether the build has a
    /// DSN baked in. `wants_errors()` returning `false` short-circuits
    /// the `build_guard` call before it can touch `sentry::init`.
    #[test]
    fn inactive_without_consent() {
        let settings = TelemetrySettings::default();
        assert!(!settings.wants_errors());
        let controller = Controller::init(&settings);
        assert!(!controller.is_active());
    }

    /// Conversely, even with consent recorded, an empty `SENTRY_DSN`
    /// (the dev-build default) keeps the controller inactive — the
    /// secret-less build doesn't accidentally point at someone else's
    /// Sentry project.
    #[test]
    fn inactive_when_dsn_absent_in_dev_builds() {
        if !SENTRY_DSN.is_empty() {
            // Test runs with a baked DSN; skip rather than send fake
            // events into a real project. CI dev builds always have an
            // empty DSN so this is the common path.
            return;
        }
        let mut settings = TelemetrySettings::default();
        settings.record_consent();
        assert!(settings.wants_errors());
        let controller = Controller::init(&settings);
        assert!(!controller.is_active());
    }
}
