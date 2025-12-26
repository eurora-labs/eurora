//! Utilities for getting information about the runtime environment.

use std::collections::HashMap;
use std::sync::LazyLock;

/// The version of the agent-chain-core library.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Cached runtime environment information.
static RUNTIME_ENVIRONMENT: LazyLock<HashMap<&'static str, String>> = LazyLock::new(|| {
    let mut env = HashMap::new();
    env.insert("library_version", VERSION.to_string());
    env.insert("library", "agent-chain-core".to_string());
    env.insert("platform", get_platform_string());
    env.insert("runtime", "rust".to_string());
    env.insert("runtime_version", get_rust_version());
    env
});

/// Get information about the LangChain runtime environment.
///
/// Returns a HashMap with information about the runtime environment:
/// - `library_version`: The version of the library
/// - `library`: The library name ("agent-chain-core")
/// - `platform`: Platform information (OS, architecture)
/// - `runtime`: The runtime ("rust")
/// - `runtime_version`: The Rust version used to compile
pub fn get_runtime_environment() -> &'static HashMap<&'static str, String> {
    &RUNTIME_ENVIRONMENT
}

/// Gets a platform string similar to Python's platform.platform().
fn get_platform_string() -> String {
    format!(
        "{}-{}-{}",
        std::env::consts::OS,
        std::env::consts::ARCH,
        std::env::consts::FAMILY
    )
}

/// Gets the Rust version used to compile this crate.
fn get_rust_version() -> String {
    env!("CARGO_PKG_RUST_VERSION")
        .parse::<String>()
        .unwrap_or_else(|_| "unknown".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_runtime_environment() {
        let env = get_runtime_environment();

        assert_eq!(env.get("library").unwrap(), "agent-chain-core");
        assert_eq!(env.get("runtime").unwrap(), "rust");
        assert!(env.contains_key("library_version"));
        assert!(env.contains_key("platform"));
        assert!(env.contains_key("runtime_version"));
    }

    #[test]
    fn test_runtime_environment_is_cached() {
        let env1 = get_runtime_environment();
        let env2 = get_runtime_environment();
        assert!(std::ptr::eq(env1, env2));
    }
}
