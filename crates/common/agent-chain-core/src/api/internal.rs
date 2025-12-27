//! Internal utilities for checking if calls originate from within the agent-chain crates.

/// Checks if the caller module is internal to agent-chain.
///
/// In Rust, we use compile-time module paths rather than runtime introspection.
/// This function provides a way to check if a given module path is internal.
///
/// # Arguments
///
/// * `module_path` - The module path to check (typically from `module_path!()` macro)
///
/// # Returns
///
/// `true` if the module path starts with "agent_chain", `false` otherwise.
///
/// # Example
///
/// ```
/// use agent_chain_core::api::is_caller_internal;
///
/// // Check at compile time
/// let is_internal = is_caller_internal(module_path!());
/// ```
#[inline]
pub fn is_caller_internal(module_path: &str) -> bool {
    module_path.starts_with("agent_chain")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_caller_internal() {
        assert!(is_caller_internal("agent_chain_core::api::internal"));
        assert!(is_caller_internal("agent_chain::providers"));
        assert!(is_caller_internal("agent_chain_macros"));
        assert!(!is_caller_internal("my_app::main"));
        assert!(!is_caller_internal("other_crate::module"));
    }

    #[test]
    fn test_current_module_is_internal() {
        assert!(is_caller_internal(module_path!()));
    }
}
