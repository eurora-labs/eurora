//! Bake URL constants into the desktop binary.
//!
//! `WEB_URL` (login landing) is forwarded from the process environment.
//! `load_env` in `src/lib.rs` injects the non-empty value into the
//! process environment at startup so `std::env::var(...)` call sites in
//! the procedures continue to work in packaged release builds where
//! `.env` isn't available on disk.
//!
//! Telemetry secrets (`EURORA_SENTRY_DSN`, `EURORA_RELEASE_CHANNEL`,
//! `RELEASE_VERSION`, etc.) are owned by `euro-telemetry/build.rs` and
//! consumed via `env!()` from inside that crate, so they don't appear
//! here.
//!
//! Values come from the process environment only; the justfile (`set
//! dotenv-load`) is the single point that reads `.env` and exports it
//! to cargo.

use std::path::PathBuf;

/// Required URL bake-ins: build fails if any is missing.
const REQUIRED_URLS: &[&str] = &["WEB_URL"];

/// Optional runtime overrides: empty if unset.
const OPTIONAL_URLS: &[&str] = &[];

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

    for key in OPTIONAL_URLS {
        println!("cargo:rerun-if-env-changed={key}");
        let value = std::env::var(key).unwrap_or_default();
        println!("cargo:rustc-env={key}={value}");
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
