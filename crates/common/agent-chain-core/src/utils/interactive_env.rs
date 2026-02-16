//! Utilities for working with interactive environments.
//!
//! Adapted from langchain_core/utils/interactive_env.py

use std::io::IsTerminal;

/// Determine if running within an interactive environment.
///
/// This function attempts to detect if the current process is running
/// in an interactive environment like a REPL or Jupyter notebook.
pub fn is_interactive_env() -> bool {
    std::env::var("RUST_INTERACTIVE").is_ok() || std::io::stdin().is_terminal()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_interactive_env() {
        let _ = is_interactive_env();
    }
}
