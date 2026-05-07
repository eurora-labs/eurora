use std::path::{Path, PathBuf};

/// Keys baked into the binary at compile time. Mobile apps run in a
/// sandbox so the runtime cannot find the project's `.env`; we read it
/// here and forward only the keys the mobile app actually consumes.
const FORWARDED_KEYS: &[&str] = &[
    "EURORA_AUTH_SERVICE_URL",
    "EURORA_API_BASE_URL",
    "EURORA_REST_API_URL",
];

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

/// Locate the workspace root and bake the project's `.env` values into
/// `cargo:rustc-env=…` slots so `option_env!` can recover them at
/// runtime in the mobile sandbox.
fn forward_workspace_env(manifest_dir: &Path) {
    for key in FORWARDED_KEYS {
        println!("cargo:rerun-if-env-changed={key}");
    }

    let Some(workspace_root) = find_workspace_root(manifest_dir) else {
        // No workspace root found (running outside a checkout, e.g.
        // `cargo install`). Emit empty values so `option_env!` returns
        // None and the runtime falls back to its production defaults.
        for key in FORWARDED_KEYS {
            println!("cargo:rustc-env={key}=");
        }
        return;
    };

    let env_path = workspace_root.join(".env");
    println!("cargo:rerun-if-changed={}", env_path.display());

    let entries = std::fs::read_to_string(&env_path)
        .map(|content| parse_env(&content))
        .unwrap_or_default();

    for key in FORWARDED_KEYS {
        // CI / shell-exported values win over the .env file.
        let value = std::env::var(key)
            .ok()
            .or_else(|| {
                entries
                    .iter()
                    .find(|(k, _)| k == key)
                    .map(|(_, v)| v.clone())
            })
            .unwrap_or_default();
        println!("cargo:rustc-env={key}={value}");
    }
}

/// Walk up from `start` looking for the `Cargo.toml` that declares
/// `[workspace]`. Returns the directory containing it.
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
