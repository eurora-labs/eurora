//! Bake telemetry secrets and release identity into any binary that
//! depends on this crate.
//!
//! Both `euro-tauri` (desktop) and `euro-mobile` consume `env!()` reads
//! emitted from this `build.rs`, so the keys are owned in exactly one
//! place. Values come from the host environment only; the justfile
//! (`set dotenv-load`) and CI are the single points that read `.env` /
//! secrets and export them to cargo.
//!
//! The bake is fail-closed: if `EURORA_SENTRY_DSN` is non-empty,
//! `EURORA_RELEASE_CHANNEL` and `RELEASE_VERSION` must be non-empty
//! too. A telemetry-bearing build with an unidentifiable release would
//! tag every shipped binary with the same release/environment, silently
//! breaking Sentry Release Health and source-map matching — refusing to
//! build is the only correct response.
//!
//! Forwarding via `cargo:rustc-env` (rather than relying on the host
//! shell at `env!()` resolution time) ensures every key resolves
//! deterministically — an unset `EURORA_POSTHOG_KEY` becomes an empty
//! string the runtime can branch on, instead of failing compilation.

/// Telemetry keys forwarded to dependents at compile time. Each is
/// optional individually; the post-forward audit enforces an
/// all-or-nothing rule whenever a DSN is present.
const TELEMETRY_KEYS: &[&str] = &[
    "EURORA_SENTRY_DSN",
    "EURORA_POSTHOG_KEY",
    "EURORA_POSTHOG_HOST",
    "EURORA_RELEASE_CHANNEL",
    "RELEASE_VERSION",
];

/// Keys that must be non-empty whenever `EURORA_SENTRY_DSN` is
/// non-empty. Each one is load-bearing for Sentry to be useful:
/// - `EURORA_RELEASE_CHANNEL` becomes the Sentry `environment` tag.
///   Without it every event lands in a "dev" bucket and prod/nightly
///   are indistinguishable.
/// - `RELEASE_VERSION` becomes the Sentry `release` tag. Without it
///   every shipped binary tags events with the same release and
///   Release Health, regression detection, and source-map matching all
///   silently break.
const REQUIRED_WHEN_DSN_SET: &[&str] = &["EURORA_RELEASE_CHANNEL", "RELEASE_VERSION"];

fn main() {
    forward_env();
    audit_telemetry_consistency();
}

fn forward_env() {
    for key in TELEMETRY_KEYS {
        println!("cargo:rerun-if-env-changed={key}");
        let value = std::env::var(key).unwrap_or_default();
        println!("cargo:rustc-env={key}={value}");
    }
}

fn audit_telemetry_consistency() {
    let dsn = std::env::var("EURORA_SENTRY_DSN").unwrap_or_default();
    if dsn.is_empty() {
        return;
    }
    let missing: Vec<&str> = REQUIRED_WHEN_DSN_SET
        .iter()
        .copied()
        .filter(|k| std::env::var(k).map(|v| v.is_empty()).unwrap_or(true))
        .collect();
    if !missing.is_empty() {
        panic!(
            "build.rs: EURORA_SENTRY_DSN is set but {missing:?} {is_are} empty. \
             A telemetry-bearing build must carry a release identifier and a \
             channel name; otherwise every shipped binary tags Sentry events \
             with the same release and environment, and Release Health \
             silently breaks.",
            is_are = if missing.len() == 1 { "is" } else { "are" },
        );
    }
}
