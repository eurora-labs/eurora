fn main() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
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

    embed_telemetry_keys();

    tauri_build::build();
}

/// Forward telemetry secrets from the build environment into compiled
/// `env!()` slots. Missing values become empty strings so dev builds
/// (and CI for forks) stay green; the runtime treats empty as "disabled".
fn embed_telemetry_keys() {
    for var in [
        "EURORA_DESKTOP_SENTRY_DSN",
        "EURORA_DESKTOP_POSTHOG_KEY",
        "EURORA_DESKTOP_POSTHOG_HOST",
        "EURORA_RELEASE_CHANNEL",
    ] {
        let value = std::env::var(var).unwrap_or_default();
        println!("cargo:rustc-env={var}={value}");
        println!("cargo:rerun-if-env-changed={var}");
    }
}
