//! Forward selected env vars into compile-time `cargo:rustc-env` slots
//! and run the Tauri mobile build pipeline.
//!
//! Mobile apps run in a sandbox with no access to `.env` at runtime,
//! so anything the binary needs to know about its backend has to be
//! baked at build time. Values come from the process environment
//! only; the justfile (`set dotenv-load`) is the single point that
//! reads `.env` and exports it to cargo. Unset/empty values fall back
//! to the dev-server default (matching the justfile's
//! `env_var_or_default`) so plain `cargo check` works during local
//! development; CI and release builds always set the var explicitly.

use std::path::PathBuf;

/// Required URL bake-ins with their dev-server fallbacks.
const REQUIRED: &[(&str, &str)] = &[("WEB_URL", "http://localhost:5173")];

/// Optional: empty if unset. Native Google sign-in is gated on these
/// being present at runtime (see `procedures::auth_procedures::
/// native_google_client_id`); when absent, the mobile UI falls back to
/// the in-app browser flow without erroring out.
const OPTIONAL: &[&str] = &["GOOGLE_CLIENT_ID", "GOOGLE_CLIENT_ID_IOS"];

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
    for (key, fallback) in REQUIRED {
        println!("cargo:rerun-if-env-changed={key}");
        let value = std::env::var(key)
            .ok()
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| {
                println!("cargo:warning=build.rs: `{key}` unset; falling back to `{fallback}`");
                (*fallback).to_string()
            });
        println!("cargo:rustc-env={key}={value}");
    }

    for key in OPTIONAL {
        println!("cargo:rerun-if-env-changed={key}");
        let value = std::env::var(key).unwrap_or_default();
        println!("cargo:rustc-env={key}={value}");
    }
}
