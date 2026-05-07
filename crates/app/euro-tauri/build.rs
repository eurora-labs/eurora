//! Bake URL constants and telemetry secrets into the desktop binary.
//!
//! `WEB_URL` (login landing) is forwarded from the workspace `.env`
//! (or shell env). `load_env` in `src/lib.rs` injects the non-empty
//! value into the process environment at startup so
//! `std::env::var(...)` call sites in the procedures continue to work
//! in packaged release builds where `.env` isn't available on disk.
//!
//! Telemetry secrets (`EURORA_DESKTOP_*`) are baked the same way but
//! consumed via `env!()` directly by the telemetry module — empty
//! values disable the corresponding service.

use std::path::{Path, PathBuf};

/// Required URL bake-ins: build fails if any is missing.
const REQUIRED_URLS: &[&str] = &["WEB_URL"];

/// Optional runtime overrides: empty if unset.
const OPTIONAL_URLS: &[&str] = &[];

/// Telemetry keys: optional, missing values disable telemetry at runtime.
const TELEMETRY_KEYS: &[&str] = &[
    "EURORA_DESKTOP_SENTRY_DSN",
    "EURORA_DESKTOP_POSTHOG_KEY",
    "EURORA_DESKTOP_POSTHOG_HOST",
    "EURORA_RELEASE_CHANNEL",
];

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

    forward_workspace_env(&manifest_dir);

    tauri_build::build();
}

fn forward_workspace_env(manifest_dir: &Path) {
    let all_keys = REQUIRED_URLS
        .iter()
        .chain(OPTIONAL_URLS)
        .chain(TELEMETRY_KEYS);
    for key in all_keys {
        println!("cargo:rerun-if-env-changed={key}");
    }

    let env_path = find_workspace_root(manifest_dir).map(|root| root.join(".env"));
    if let Some(path) = &env_path {
        println!("cargo:rerun-if-changed={}", path.display());
    }

    let entries = env_path
        .as_ref()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .map(|content| parse_env(&content))
        .unwrap_or_default();

    let lookup = |key: &str| -> Option<String> {
        // Shell env wins so CI / production builds can override the
        // dev defaults that ship in `.env.example`.
        std::env::var(key).ok().or_else(|| {
            entries
                .iter()
                .find(|(k, _)| k == key)
                .map(|(_, v)| v.clone())
        })
    };

    for key in REQUIRED_URLS {
        let value = lookup(key).filter(|v| !v.is_empty()).unwrap_or_else(|| {
            let where_to_look = match &env_path {
                Some(p) => format!("`.env` at {}", p.display()),
                None => "`.env` (workspace root not found)".to_string(),
            };
            panic!(
                "build.rs: required env var `{key}` is unset.\n\
                 Add `{key}=...` to {where_to_look} or export it in your shell.\n\
                 For local dev: run `just init` to create .env from .env.example."
            );
        });
        println!("cargo:rustc-env={key}={value}");
    }

    for key in OPTIONAL_URLS.iter().chain(TELEMETRY_KEYS) {
        let value = lookup(key).unwrap_or_default();
        println!("cargo:rustc-env={key}={value}");
    }
}

fn find_workspace_root(start: &Path) -> Option<PathBuf> {
    for ancestor in start.ancestors() {
        let manifest = ancestor.join("Cargo.toml");
        if let Ok(content) = std::fs::read_to_string(&manifest)
            && content.contains("[workspace]")
        {
            return Some(ancestor.to_path_buf());
        }
    }
    None
}

fn parse_env(content: &str) -> Vec<(String, String)> {
    content
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                return None;
            }
            let (key, value) = line.split_once('=')?;
            let value = value.trim().trim_matches('"').trim_matches('\'');
            Some((key.trim().to_string(), value.to_string()))
        })
        .collect()
}
