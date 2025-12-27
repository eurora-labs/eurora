//! Utilities for working with interactive environments.
//!
//! Adapted from langchain_core/utils/interactive_env.py

use std::io::IsTerminal;

/// Determine if running within an interactive environment.
///
/// This function attempts to detect if the current process is running
/// in an interactive environment like a REPL or Jupyter notebook.
///
/// # Returns
///
/// `true` if running in an interactive environment, `false` otherwise.
///
/// # Example
///
/// ```
/// use agent_chain_core::utils::interactive_env::is_interactive_env;
///
/// let is_interactive = is_interactive_env();
/// // Returns true if running in a REPL, false otherwise
/// ```
pub fn is_interactive_env() -> bool {
    std::env::var("RUST_INTERACTIVE").is_ok() || std::io::stdin().is_terminal()
}

/// Check if running in a CI environment.
///
/// # Returns
///
/// `true` if running in a CI environment, `false` otherwise.
pub fn is_ci_env() -> bool {
    std::env::var("CI").is_ok()
        || std::env::var("CONTINUOUS_INTEGRATION").is_ok()
        || std::env::var("GITHUB_ACTIONS").is_ok()
        || std::env::var("GITLAB_CI").is_ok()
        || std::env::var("TRAVIS").is_ok()
        || std::env::var("CIRCLECI").is_ok()
        || std::env::var("JENKINS_URL").is_ok()
}

/// Check if running in a testing environment.
///
/// # Returns
///
/// `true` if running in a testing environment, `false` otherwise.
pub fn is_test_env() -> bool {
    std::env::var("RUST_TEST").is_ok() || cfg!(test)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_interactive_env() {
        let _ = is_interactive_env();
    }

    #[test]
    fn test_is_ci_env() {
        let _ = is_ci_env();
    }

    #[test]
    fn test_is_test_env() {
        assert!(is_test_env());
    }
}
