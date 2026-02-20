use std::io::IsTerminal;

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
