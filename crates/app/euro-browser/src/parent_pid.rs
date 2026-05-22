//! Resolve and cache the parent browser's PID for the lifetime of the
//! native-messaging host process.
//!
//! Resolution order, highest priority first:
//!
//! 1. `EURORA_BROWSER_PID` environment variable — set by Firefox manifest
//!    entries that pass `{pid}` so we don't have to walk a process tree
//!    we wouldn't have access to anyway.
//! 2. A browser ancestor reachable via [`euro_process::browser_ancestor_pid`].
//! 3. The direct parent PID as a last-resort fallback so we always have
//!    *some* identifier even on unknown browser configurations.
//!
//! The resolved PID is cached in a [`OnceLock`] so callers can read it
//! synchronously without re-walking the tree.

use std::sync::OnceLock;

static PARENT_PID: OnceLock<u32> = OnceLock::new();

/// Capture the parent browser PID into the per-process cache. Idempotent
/// across the lifetime of the process; subsequent calls log a warning
/// rather than overwriting.
pub fn capture_parent_pid() {
    let ppid = resolve_parent_pid();
    if PARENT_PID.set(ppid).is_err() {
        tracing::warn!("Parent PID was already captured");
    }
    tracing::info!("Captured browser PID: {ppid}");
}

/// Cached parent browser PID, or `0` if [`capture_parent_pid`] hasn't
/// run yet (which would indicate a programming error — the binary
/// captures the PID on startup before anything else needs it).
pub fn get_parent_pid() -> u32 {
    PARENT_PID.get().copied().unwrap_or(0)
}

fn resolve_parent_pid() -> u32 {
    if let Ok(env_pid) = std::env::var("EURORA_BROWSER_PID") {
        match env_pid.parse::<u32>() {
            Ok(pid) => {
                tracing::info!("Using browser PID from EURORA_BROWSER_PID: {pid}");
                return pid;
            }
            Err(_) => {
                tracing::warn!(
                    "Invalid EURORA_BROWSER_PID value {env_pid:?}, falling back to ancestor walk",
                );
            }
        }
    }

    if let Some(pid) = euro_process::browser_ancestor_pid() {
        tracing::info!("Found browser ancestor PID: {pid}");
        return pid;
    }

    let direct = euro_process::parent_pid();
    tracing::debug!("No browser ancestor found; using direct parent PID {direct}");
    direct
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_returns_a_pid() {
        // Either the ancestor walk finds a browser or we fall back to
        // the direct parent — both yield a positive PID under any sane
        // test runner.
        let pid = resolve_parent_pid();
        assert!(pid > 0, "expected positive PID, got {pid}");
    }
}
