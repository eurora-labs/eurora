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
