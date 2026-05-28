//! Bake `BACKEND_URL` into `api_settings::DEFAULT_API_URL` at compile time.
//!
//! The value comes from the process environment. The justfile (`set
//! dotenv-load`) is the single point that reads `.env` and exports it
//! into cargo; CI and production deployments inject vars via their
//! own mechanisms. When the var is unset/empty the build falls back to
//! `http://localhost:3000` so plain `cargo check` works during local
//! development; CI and release builds always set the var explicitly.

const REQUIRED: &[(&str, &str)] = &[("BACKEND_URL", "http://localhost:3000")];

fn main() {
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
}
