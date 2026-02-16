//! Handle chained inputs and terminal output formatting.
//!
//! Adapted from langchain_core/utils/input.py

use std::collections::HashMap;
use std::io::{self, Write};
use tracing::warn;

/// Text color mapping for terminal output.
pub static TEXT_COLOR_MAPPING: std::sync::LazyLock<HashMap<&'static str, &'static str>> =
    std::sync::LazyLock::new(|| {
        let mut m = HashMap::new();
        m.insert("blue", "36;1");
        m.insert("yellow", "33;1");
        m.insert("pink", "38;5;200");
        m.insert("green", "32;1");
        m.insert("red", "31;1");
        m
    });

/// Get mapping for items to a support color.
///
/// # Arguments
///
/// * `items` - The items to map to colors.
/// * `excluded_colors` - The colors to exclude.
///
/// # Returns
///
/// A mapping of items to colors.
///
/// # Errors
///
/// Returns an error if no colors are available after applying exclusions.
///
/// # Example
///
/// ```
/// use agent_chain_core::utils::input::get_color_mapping;
///
/// let items = vec!["item1".to_string(), "item2".to_string()];
/// let mapping = get_color_mapping(&items, None).unwrap();
/// assert!(mapping.contains_key("item1"));
/// assert!(mapping.contains_key("item2"));
/// ```
pub fn get_color_mapping(
    items: &[String],
    excluded_colors: Option<&[&str]>,
) -> Result<HashMap<String, String>, InputError> {
    let colors: Vec<&str> = TEXT_COLOR_MAPPING
        .keys()
        .filter(|c| {
            excluded_colors
                .map(|excluded| !excluded.contains(c))
                .unwrap_or(true)
        })
        .copied()
        .collect();

    if colors.is_empty() {
        return Err(InputError::NoColorsAvailable);
    }

    let mut mapping = HashMap::new();
    for (i, item) in items.iter().enumerate() {
        mapping.insert(item.clone(), colors[i % colors.len()].to_string());
    }

    Ok(mapping)
}

/// Get colored text for terminal output.
///
/// # Arguments
///
/// * `text` - The text to color.
/// * `color` - The color to use (must be a key in `TEXT_COLOR_MAPPING`).
///
/// # Returns
///
/// The colored text with ANSI escape codes.
///
/// # Example
///
/// ```
/// use agent_chain_core::utils::input::get_colored_text;
///
/// let colored = get_colored_text("Hello", "blue");
/// // Returns text with ANSI color codes
/// ```
pub fn get_colored_text(text: &str, color: &str) -> String {
    let color_str = TEXT_COLOR_MAPPING.get(color).copied().unwrap_or("0");

    format!("\x1b[{}m\x1b[1;3m{}\x1b[0m", color_str, text)
}

/// Get bolded text for terminal output.
///
/// # Arguments
///
/// * `text` - The text to bold.
///
/// # Returns
///
/// The bolded text with ANSI escape codes.
///
/// # Example
///
/// ```
/// use agent_chain_core::utils::input::get_bolded_text;
///
/// let bold = get_bolded_text("Important");
/// // Returns text with ANSI bold codes
/// ```
pub fn get_bolded_text(text: &str) -> String {
    format!("\x1b[1m{}\x1b[0m", text)
}

/// Print text with highlighting and optional color.
///
/// # Arguments
///
/// * `text` - The text to print.
/// * `color` - Optional color for the text.
/// * `end` - The string to append at the end (default is empty).
/// * `writer` - Optional writer to output to (defaults to stdout).
///
/// # Example
///
/// ```
/// use agent_chain_core::utils::input::print_text;
///
/// print_text("Hello, World!", Some("blue"), "", None);
/// ```
pub fn print_text(text: &str, color: Option<&str>, end: &str, writer: Option<&mut dyn Write>) {
    let text_to_print = if let Some(c) = color {
        get_colored_text(text, c)
    } else {
        text.to_string()
    };

    let output = format!("{}{}", text_to_print, end);

    if let Some(w) = writer {
        if let Err(e) = write!(w, "{}", output) {
            warn!("print_text write error: {e}");
        }
        if let Err(e) = w.flush() {
            warn!("print_text flush error: {e}");
        }
    } else {
        print!("{}", output);
        if let Err(e) = io::stdout().flush() {
            warn!("print_text flush error: {e}");
        }
    }
}

/// Error types for input operations.
#[derive(Debug, Clone, PartialEq)]
pub enum InputError {
    /// No colors are available after applying exclusions.
    NoColorsAvailable,
}

impl std::fmt::Display for InputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InputError::NoColorsAvailable => {
                write!(f, "No colors available after applying exclusions")
            }
        }
    }
}

impl std::error::Error for InputError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_color_mapping() {
        let items = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let mapping = get_color_mapping(&items, None).unwrap();

        assert_eq!(mapping.len(), 3);
        assert!(mapping.contains_key("a"));
        assert!(mapping.contains_key("b"));
        assert!(mapping.contains_key("c"));
    }

    #[test]
    fn test_get_color_mapping_with_exclusions() {
        let items = vec!["a".to_string(), "b".to_string()];
        let excluded = vec!["blue", "yellow", "pink"];
        let mapping = get_color_mapping(&items, Some(&excluded)).unwrap();

        for color in mapping.values() {
            assert!(!excluded.contains(&color.as_str()));
        }
    }

    #[test]
    fn test_get_color_mapping_cycles() {
        let items: Vec<String> = (0..10).map(|i| i.to_string()).collect();
        let mapping = get_color_mapping(&items, None).unwrap();

        assert_eq!(mapping.len(), 10);
    }

    #[test]
    fn test_get_colored_text() {
        let colored = get_colored_text("test", "blue");
        assert!(colored.contains("36;1"));
        assert!(colored.contains("test"));
        assert!(colored.contains("\x1b[0m"));
    }

    #[test]
    fn test_get_bolded_text() {
        let bolded = get_bolded_text("test");
        assert!(bolded.contains("\x1b[1m"));
        assert!(bolded.contains("test"));
        assert!(bolded.contains("\x1b[0m"));
    }

    #[test]
    fn test_print_text_to_buffer() {
        let mut buffer = Vec::new();
        print_text("hello", Some("blue"), "\n", Some(&mut buffer));

        let output = String::from_utf8(buffer).unwrap();
        assert!(output.contains("hello"));
        assert!(output.ends_with('\n'));
    }

    #[test]
    fn test_print_text_no_color() {
        let mut buffer = Vec::new();
        print_text("plain", None, "", Some(&mut buffer));

        let output = String::from_utf8(buffer).unwrap();
        assert_eq!(output, "plain");
    }
}
