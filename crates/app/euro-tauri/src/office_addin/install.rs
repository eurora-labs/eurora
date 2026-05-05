//! Per-OS deployment of the rendered add-in manifest into the Office
//! catalog.
//!
//! Idempotent on every launch: a clean reinstall and a no-op reinstall
//! converge on the same final state. Failures never crash the desktop;
//! the caller logs and continues.
//!
//! This file owns the cross-platform contract — public API, shared
//! types, the dev-sideload gate, and the standalone-uninstall sequence.
//! The actual catalog deployment lives in
//! `super::platform::install::{install_for_app, uninstall_for_app,
//! uninstall_standalone}`, where `platform` is one of [`super::macos`],
//! [`super::windows`], or [`super::linux`] (selected at compile time in
//! `office_addin.rs`).

use std::path::PathBuf;
#[cfg(any(target_os = "macos", target_os = "windows"))]
use std::{fs, path::Path};

use tauri::{AppHandle, Runtime};

#[cfg(any(target_os = "macos", target_os = "windows"))]
use super::Error;
use super::Result;

/// File name used for the deployed Word manifest inside the Office
/// catalog. Shared by the macOS and Windows install backends so a clean
/// install and a clean uninstall converge on the same path.
#[cfg(any(target_os = "macos", target_os = "windows"))]
pub(super) const MANIFEST_FILE: &str = "com.eurora.word.xml";

/// Set to `1` by `pnpm dev:word` (or any developer running the Vite-served
/// add-in via `office-addin-debugging`) so the desktop's bundled-install path
/// stays out of the Office catalog. Without this, the desktop would write a
/// rendered manifest pointing at `file:///…/resources/office-addin/runtime.html`
/// — pointing into a dev resource tree that may not exist — and Word would
/// register *two* "Eurora" add-ins (the live Vite one and the broken file://
/// one), which is confusing and breaks the live-reload story.
const DEV_SIDELOAD_ENV: &str = "EURORA_OFFICE_ADDIN_DEV_SIDELOAD";

/// Outcome of an install attempt. Distinguishes "did not apply on this OS" from
/// "Word's per-user state isn't ready yet" so the caller can log the right
/// severity.
#[derive(Debug, Clone)]
pub enum InstallOutcome {
    /// Manifest was written; Word will pick it up on its next launch.
    Installed { manifest_path: PathBuf },
    /// macOS only: Word's sandboxed container has not been created yet
    /// (Word has never been launched on this user account). The desktop
    /// will retry on its next launch.
    SkippedHostNotPresent,
    /// Linux/other: Word does not run natively here.
    SkippedUnsupportedOs,
    /// `EURORA_OFFICE_ADDIN_DEV_SIDELOAD=1` is set; a developer is iterating
    /// on the add-in via the Vite dev server + `office-addin-debugging` and
    /// does not want the desktop to also touch the Office catalog.
    SkippedDevSideload,
}

/// Result of a standalone uninstall, surfaced to the CLI for user-facing logs.
#[derive(Debug, Clone)]
pub enum UninstallOutcome {
    /// Removed the manifest file (and on Windows, the trusted-catalog subkey).
    /// The contained path is the manifest file location for reporting purposes
    /// — the file may or may not have existed before the call (the operation
    /// is idempotent).
    Cleaned { manifest_path: PathBuf },
    /// Linux/other: nothing to clean — Word does not run natively here.
    SkippedUnsupportedOs,
}

/// Returns true when `EURORA_OFFICE_ADDIN_DEV_SIDELOAD=1`. Anything else
/// (unset, empty, `0`, other values) is treated as "not in dev sideload mode"
/// to keep the contract narrow — accidentally exporting the variable as `true`
/// in a shell rc shouldn't silently disable the install.
fn dev_sideload_active() -> bool {
    matches!(std::env::var_os(DEV_SIDELOAD_ENV).as_deref(), Some(v) if v == "1")
}

/// Render the bundled manifest and hand off to the platform backend.
/// On Linux the backend is a no-op that returns
/// [`InstallOutcome::SkippedUnsupportedOs`].
pub fn install_for_app<R: Runtime>(app: &AppHandle<R>) -> Result<InstallOutcome> {
    if dev_sideload_active() {
        return Ok(InstallOutcome::SkippedDevSideload);
    }
    super::platform::install::install_for_app(app)
}

/// Tear down the catalog entry the platform backend wrote during
/// [`install_for_app`]. On Linux the backend is a no-op.
pub fn uninstall_for_app<R: Runtime>(app: &AppHandle<R>) -> Result<()> {
    super::platform::install::uninstall_for_app(app)
}

/// Remove every artifact `install_for_app` could have written, without
/// requiring a Tauri runtime. Idempotent: safe to run when nothing is
/// installed.
pub fn uninstall_standalone() -> Result<UninstallOutcome> {
    cfg_select! {
        any(target_os = "macos", target_os = "windows") => {
            let manifest_path = super::platform::install::uninstall_standalone()?;
            Ok(UninstallOutcome::Cleaned { manifest_path })
        }
        _ => Ok(UninstallOutcome::SkippedUnsupportedOs),
    }
}

/// Idempotent file removal: missing-file errors collapse to `Ok(())`.
/// Promoted to `pub(super)` so the macOS and Windows install backends
/// share one helper.
#[cfg(any(target_os = "macos", target_os = "windows"))]
pub(super) fn remove_if_exists(path: &Path) -> Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(Error::Io {
            action: "removing",
            path: path.to_path_buf(),
            source,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Serialises tests that mutate process-global env vars. Without this,
    /// concurrent tests in the same process can observe each other's writes.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    /// Scoped env-var override that always restores the prior value, even on
    /// panic. Required because `std::env::set_var` mutates process state and
    /// `std::env::set_var` / `remove_var` are `unsafe` in Rust 2024.
    struct EnvGuard {
        key: &'static str,
        prev: Option<std::ffi::OsString>,
    }

    impl EnvGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let prev = std::env::var_os(key);
            // SAFETY: serialised via ENV_LOCK; restored in Drop.
            unsafe { std::env::set_var(key, value) };
            Self { key, prev }
        }

        fn unset(key: &'static str) -> Self {
            let prev = std::env::var_os(key);
            // SAFETY: serialised via ENV_LOCK; restored in Drop.
            unsafe { std::env::remove_var(key) };
            Self { key, prev }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            // SAFETY: serialised via ENV_LOCK.
            unsafe {
                match self.prev.take() {
                    Some(value) => std::env::set_var(self.key, value),
                    None => std::env::remove_var(self.key),
                }
            }
        }
    }

    #[test]
    fn dev_sideload_active_only_true_for_exact_one() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

        let _g = EnvGuard::unset(DEV_SIDELOAD_ENV);
        assert!(!dev_sideload_active(), "unset should be inactive");

        let _g = EnvGuard::set(DEV_SIDELOAD_ENV, "1");
        assert!(dev_sideload_active(), "exact `1` should be active");

        for value in ["", "0", "true", "yes", "2", " 1"] {
            let _g = EnvGuard::set(DEV_SIDELOAD_ENV, value);
            assert!(
                !dev_sideload_active(),
                "value {value:?} should not activate dev sideload"
            );
        }
    }
}
