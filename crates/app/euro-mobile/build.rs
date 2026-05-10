//! Forward selected env vars into compile-time `cargo:rustc-env` slots
//! and run the Tauri mobile build pipeline.
//!
//! Mobile apps run in a sandbox with no access to `.env` at runtime,
//! so anything the binary needs to know about its backend has to be
//! baked at build time. Values come from the process environment
//! only; the justfile (`set dotenv-load`) is the single point that
//! reads `.env` and exports it to cargo.

use std::path::PathBuf;

/// Required: build fails loudly if these aren't set. The binary
/// cannot function without them.
const REQUIRED: &[&str] = &["WEB_URL"];

/// Optional: empty if unset.
const OPTIONAL: &[&str] = &[];

fn main() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    assert_eq!(manifest_dir.file_name().unwrap(), "euro-mobile");

    let build_dir = manifest_dir
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("apps")
        .join("mobile")
        .join("build");
    if !build_dir.exists() {
        #[allow(clippy::expect_fun_call, clippy::create_dir)]
        std::fs::create_dir(&build_dir).expect(
            format!(
                "failed to create apps/mobile/build directory: {:?}",
                build_dir
            )
            .as_str(),
        );
    }

    forward_env();

    tauri_build::build();
}

fn forward_env() {
    for key in REQUIRED {
        println!("cargo:rerun-if-env-changed={key}");
        let value = std::env::var(key)
            .ok()
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| missing(key));
        println!("cargo:rustc-env={key}={value}");
    }

    for key in OPTIONAL {
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
