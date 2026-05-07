//! Bake `BACKEND_URL` into `DEFAULT_API_URL` at compile time. See
//! `euro-settings/build.rs` for the same pattern with more
//! commentary; the two scripts deliberately stay independent (no
//! shared build crate) — the forwarding logic is short enough to
//! inline.

use std::path::{Path, PathBuf};

const REQUIRED: &[&str] = &["BACKEND_URL"];

fn main() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    forward_required(&manifest_dir, REQUIRED);
}

fn forward_required(manifest_dir: &Path, keys: &[&str]) {
    for key in keys {
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

    for key in keys {
        let value = std::env::var(key)
            .ok()
            .or_else(|| {
                entries
                    .iter()
                    .find(|(k, _)| k == key)
                    .map(|(_, v)| v.clone())
            })
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| {
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
