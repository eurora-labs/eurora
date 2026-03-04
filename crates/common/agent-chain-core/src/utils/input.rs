use std::collections::HashMap;
use std::io::{self, Write};

use owo_colors::{AnsiColors, OwoColorize, Style};

use crate::error::{Error, Result};

pub const AVAILABLE_COLORS: &[&str] = &["blue", "yellow", "pink", "green", "red"];

fn ansi_color(color: &str) -> AnsiColors {
    match color {
        "blue" => AnsiColors::Cyan,
        "yellow" => AnsiColors::Yellow,
        "pink" => AnsiColors::BrightMagenta,
        "green" => AnsiColors::Green,
        "red" => AnsiColors::Red,
        _ => AnsiColors::Default,
    }
}

pub fn get_color_mapping(
    items: &[String],
    excluded_colors: Option<&[&str]>,
) -> Result<HashMap<String, String>> {
    let colors: Vec<&str> = AVAILABLE_COLORS
        .iter()
        .filter(|c| {
            excluded_colors
                .map(|excluded| !excluded.contains(c))
                .unwrap_or(true)
        })
        .copied()
        .collect();

    if colors.is_empty() {
        return Err(Error::ValidationError(
            "No colors available after applying exclusions".to_string(),
        ));
    }

    let mut mapping = HashMap::new();
    for (i, item) in items.iter().enumerate() {
        mapping.insert(item.clone(), colors[i % colors.len()].to_string());
    }

    Ok(mapping)
}

pub fn get_colored_text(text: &str, color: &str) -> String {
    let style = Style::new().bold().italic().color(ansi_color(color));
    format!("{}", text.style(style))
}

pub fn get_bolded_text(text: &str) -> String {
    format!("{}", text.bold())
}

pub fn print_text(text: &str, color: Option<&str>, end: &str, writer: Option<&mut dyn Write>) {
    let text_to_print = if let Some(c) = color {
        get_colored_text(text, c)
    } else {
        text.to_string()
    };

    let output = format!("{}{}", text_to_print, end);

    if let Some(w) = writer {
        if let Err(e) = write!(w, "{}", output) {
            tracing::warn!("print_text write error: {e}");
        }
        if let Err(e) = w.flush() {
            tracing::warn!("print_text flush error: {e}");
        }
    } else {
        print!("{}", output);
        if let Err(e) = io::stdout().flush() {
            tracing::warn!("print_text flush error: {e}");
        }
    }
}

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
        assert!(colored.contains("test"));
        assert!(colored.contains("\x1b["));
        assert!(colored.contains("\x1b[0m"));
    }

    #[test]
    fn test_get_bolded_text() {
        let bolded = get_bolded_text("test");
        assert!(bolded.contains("test"));
        assert!(bolded.contains("\x1b["));
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
