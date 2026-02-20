use rustc_version_runtime::version;
use std::collections::HashMap;
use std::sync::LazyLock;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

static RUNTIME_ENVIRONMENT: LazyLock<HashMap<&'static str, String>> = LazyLock::new(|| {
    let mut env = HashMap::new();
    env.insert("library_version", VERSION.to_string());
    env.insert("library", "agent-chain-core".to_string());
    env.insert("platform", get_platform_string());
    env.insert("runtime", "rust".to_string());
    env.insert("runtime_version", get_rust_version());
    env
});

pub fn get_runtime_environment() -> &'static HashMap<&'static str, String> {
    &RUNTIME_ENVIRONMENT
}

fn get_platform_string() -> String {
    format!(
        "{}-{}-{}",
        std::env::consts::OS,
        std::env::consts::ARCH,
        std::env::consts::FAMILY
    )
}

fn get_rust_version() -> String {
    version().to_string()
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
