//! Process-wide Sentry lifecycle, shared between the desktop
//! (`euro-tauri`) and mobile (`euro-mobile`) apps.
//!
//! ## What this crate owns
//!
//! * The native Sentry SDK init/teardown lifecycle ([`Controller`]) —
//!   PostHog stays on the frontend (`posthog-js`) where it can observe
//!   UI navigation.
//! * The compile-time secrets baked from the host environment via this
//!   crate's `build.rs` ([`SENTRY_DSN`], [`POSTHOG_KEY`], etc.).
//! * The path-scrubbing `before_send` hook that strips the user's home
//!   directory from every string-bearing field of an outgoing event.
//!
//! ## What it doesn't own
//!
//! * Tauri IPC commands (`settings_*_telemetry`, `system_*_telemetry*`)
//!   are duplicated in each app crate — they touch app-specific state
//!   (`SharedAppSettings`, [`Controller`]) that's wired into the Tauri
//!   `Manager` separately on each side.
//!
//! ## Compile-time invariants
//!
//! `build.rs` is fail-closed: if `EURORA_SENTRY_DSN` is set, every
//! field needed to identify and bucket events ([`RELEASE_CHANNEL`],
//! [`RELEASE_VERSION`]) must be set too, so the runtime never has to
//! defend against a half-configured telemetry build. A missing DSN
//! means "telemetry disabled" and dev builds don't accidentally ship
//! events to a stale project.

mod controller;
mod scrub;

pub use controller::Controller;

/// Re-exported `sentry::integrations::tracing` so consumers can wire a
/// Sentry layer into their `tracing-subscriber` setup without taking a
/// direct dependency on `sentry`. Aliased to avoid colliding with the
/// `tracing` crate at use-sites.
pub use sentry::integrations::tracing as sentry_tracing;

/// Compile-time DSN baked from `EURORA_SENTRY_DSN`. Empty string when
/// the build was produced without a DSN (every dev build, plus any
/// release variant we deliberately keep dark).
pub const SENTRY_DSN: &str = env!("EURORA_SENTRY_DSN");

/// Compile-time PostHog project key. Forwarded to the frontend
/// bootstrap payload; never consumed natively.
pub const POSTHOG_KEY: &str = env!("EURORA_POSTHOG_KEY");

/// Compile-time PostHog host (e.g. `https://eu.i.posthog.com`).
/// Forwarded to the frontend alongside [`POSTHOG_KEY`].
pub const POSTHOG_HOST: &str = env!("EURORA_POSTHOG_HOST");

/// Compile-time release channel (`dev` / `nightly` / `release`). The
/// build script enforces non-empty whenever [`SENTRY_DSN`] is set.
pub const RELEASE_CHANNEL: &str = env!("EURORA_RELEASE_CHANNEL");

/// Compile-time release version (e.g. `0.5.42`). Used as the Sentry
/// `release` tag so events from a given build are bucketed correctly.
/// The build script enforces non-empty whenever [`SENTRY_DSN`] is set.
pub const RELEASE_VERSION: &str = env!("RELEASE_VERSION");

/// Convert a baked compile-time string into `Some(s)` when non-empty
/// and `None` otherwise. Convenience for IPC commands that forward
/// these constants into `Option<String>` payloads — a missing key (dev
/// build) collapses to `None` at the wire boundary.
#[must_use]
pub fn non_empty(s: &'static str) -> Option<&'static str> {
    if s.is_empty() { None } else { Some(s) }
}
