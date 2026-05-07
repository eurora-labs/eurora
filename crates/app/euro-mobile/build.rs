//! Forward selected keys from the workspace `.env` into compile-time
//! `cargo:rustc-env` slots. Mobile apps run in a sandbox with no
//! access to `.env` at runtime, so anything the binary needs to know
//! about its backend has to be baked at build time.

use std::path::{Path, PathBuf};

/// Required: build fails loudly if these aren't set. The binary
/// cannot function without them.
const REQUIRED: &[&str] = &["EURORA_AUTH_SERVICE_URL"];

/// Optional: runtime overrides (`EURORA_API_BASE_URL`,
/// `EURORA_REST_API_URL`) may legitimately be unset. Missing keys
/// are baked as empty strings; `option_env!` returns `Some("")` and
/// `load_env` in `src/lib.rs` skips empty values, leaving the binary
/// to fall back to its compiled-in defaults.
const OPTIONAL: &[&str] = &["EURORA_API_BASE_URL", "EURORA_REST_API_URL"];

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

    forward_workspace_env(&manifest_dir);

    tauri_build::build();
}

fn forward_workspace_env(manifest_dir: &Path) {
    for key in REQUIRED.iter().chain(OPTIONAL) {
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
        // Shell env wins over the .env file so CI / production builds
        // can override the dev defaults that ship in `.env.example`.
        std::env::var(key).ok().or_else(|| {
            entries
                .iter()
                .find(|(k, _)| k == key)
                .map(|(_, v)| v.clone())
        })
    };

    for key in REQUIRED {
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

    for key in OPTIONAL {
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
