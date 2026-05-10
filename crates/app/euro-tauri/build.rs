//! Bake URL constants and telemetry secrets into the desktop binary.
//!
//! `WEB_URL` (login landing) is forwarded from the process environment.
//! `load_env` in `src/lib.rs` injects the non-empty value into the
//! process environment at startup so `std::env::var(...)` call sites in
//! the procedures continue to work in packaged release builds where
//! `.env` isn't available on disk.
//!
//! Telemetry secrets (`EURORA_DESKTOP_*`, `EURORA_RELEASE_CHANNEL`,
//! `RELEASE_VERSION`) are baked the same way but consumed via `env!()`
//! directly by the telemetry module. The bake is fail-closed: if a DSN
//! is present, every other field needed to identify and bucket events
//! must be present too. Missing pieces in a telemetry-bearing build
//! would silently produce useless Sentry data (every release tagged
//! "0.0.0", every event tagged environment "dev"), which is worse than
//! refusing to build.
//!
//! Values come from the process environment only; the justfile (`set
//! dotenv-load`) is the single point that reads `.env` and exports it
//! to cargo.

use std::path::PathBuf;

/// Required URL bake-ins: build fails if any is missing.
const REQUIRED_URLS: &[&str] = &["WEB_URL"];

/// Optional runtime overrides: empty if unset.
const OPTIONAL_URLS: &[&str] = &[];

/// Telemetry keys forwarded to the binary at compile time. All are
/// optional individually — a build with no DSN omits every other field
/// and disables telemetry at runtime — but the post-forward audit
/// below enforces an all-or-nothing rule whenever a DSN is present.
const TELEMETRY_KEYS: &[&str] = &[
    "EURORA_DESKTOP_SENTRY_DSN",
    "EURORA_DESKTOP_POSTHOG_KEY",
    "EURORA_DESKTOP_POSTHOG_HOST",
    "EURORA_RELEASE_CHANNEL",
    "RELEASE_VERSION",
];

/// Keys that must be non-empty whenever `EURORA_DESKTOP_SENTRY_DSN` is
/// non-empty. Each one is load-bearing for Sentry to be useful:
/// - `EURORA_RELEASE_CHANNEL` becomes the Sentry `environment` tag.
///   Without it every event lands in a "dev" bucket and prod/nightly
///   are indistinguishable.
/// - `RELEASE_VERSION` becomes the Sentry `release` tag. Without it
///   every shipped binary tags events with the same release and
///   Release Health, regression detection, and source-map matching all
///   silently break.
const TELEMETRY_REQUIRED_WHEN_DSN_SET: &[&str] = &["EURORA_RELEASE_CHANNEL", "RELEASE_VERSION"];

fn main() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    assert_eq!(manifest_dir.file_name().unwrap(), "euro-tauri");
    let build_dir = manifest_dir
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("apps")
        .join("desktop")
        .join("build");
    if !build_dir.exists() {
        #[allow(clippy::expect_fun_call, clippy::create_dir)]
        std::fs::create_dir(&build_dir).expect(
            format!(
                "failed to create apps/desktop/build directory: {:?}",
                build_dir
            )
            .as_str(),
        );
    }

    forward_env();
    audit_telemetry_consistency();

    tauri_build::build();
}

fn forward_env() {
    for key in REQUIRED_URLS {
        println!("cargo:rerun-if-env-changed={key}");
        let value = std::env::var(key)
            .ok()
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| missing(key));
        println!("cargo:rustc-env={key}={value}");
    }

    for key in OPTIONAL_URLS.iter().chain(TELEMETRY_KEYS) {
        println!("cargo:rerun-if-env-changed={key}");
        let value = std::env::var(key).unwrap_or_default();
        println!("cargo:rustc-env={key}={value}");
    }
}

fn audit_telemetry_consistency() {
    let dsn = std::env::var("EURORA_DESKTOP_SENTRY_DSN").unwrap_or_default();
    if dsn.is_empty() {
        return;
    }
    let missing: Vec<&str> = TELEMETRY_REQUIRED_WHEN_DSN_SET
        .iter()
        .copied()
        .filter(|k| std::env::var(k).map(|v| v.is_empty()).unwrap_or(true))
        .collect();
    if !missing.is_empty() {
        panic!(
            "build.rs: EURORA_DESKTOP_SENTRY_DSN is set but {missing:?} \
             {is_are} empty. A telemetry-bearing build must carry a release \
             identifier and a channel name; otherwise every shipped binary \
             tags Sentry events with the same release and environment, and \
             Release Health silently breaks.",
            is_are = if missing.len() == 1 { "is" } else { "are" },
        );
    }
}

fn missing(key: &str) -> ! {
    panic!(
        "build.rs: required env var `{key}` is unset.\n\
         Build via `just <recipe>` — the justfile loads `.env` and exports\n\
         every variable to cargo. To run `cargo build` directly, export\n\
         `{key}` first (`set -a; source .env; set +a; cargo build …`) or\n\
         use `direnv` (the repo ships an `.envrc`)."
    );
}
