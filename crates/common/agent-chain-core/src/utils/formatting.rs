//! Utilities for formatting strings.
//!
//! Adapted from langchain_core/utils/formatting.py

use std::collections::{HashMap, HashSet};

/// A strict formatter that checks for extra keys and requires all arguments as keyword arguments.
///
/// This formatter is based on Python's string formatting but enforces stricter rules:
/// - All arguments must be provided as keyword arguments
/// - All placeholders in the format string must be used
#[derive(Debug, Clone, Default)]
pub struct StrictFormatter;

impl StrictFormatter {
    /// Create a new StrictFormatter.
    pub fn new() -> Self {
        Self
    }

    /// Format a string with keyword arguments.
    ///
    /// # Arguments
    ///
    /// * `format_string` - The format string with placeholders like `{name}`.
    /// * `kwargs` - The keyword arguments to substitute.
    ///
    /// # Returns
    ///
    /// The formatted string, or an error if formatting fails.
    ///
    /// # Example
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use agent_chain_core::utils::formatting::StrictFormatter;
    ///
    /// let formatter = StrictFormatter::new();
    /// let mut kwargs = HashMap::new();
    /// kwargs.insert("name".to_string(), "World".to_string());
    ///
    /// let result = formatter.format("Hello, {name}!", &kwargs).unwrap();
    /// assert_eq!(result, "Hello, World!");
    /// ```
    pub fn format(
        &self,
        format_string: &str,
        kwargs: &HashMap<String, String>,
    ) -> Result<String, FormattingError> {
        let placeholders = self.extract_placeholders(format_string);
        let mut result = format_string.to_string();

        for placeholder in &placeholders {
            if let Some(value) = kwargs.get(placeholder) {
                result = result.replace(&format!("{{{}}}", placeholder), value);
            } else {
                return Err(FormattingError::MissingKey(placeholder.clone()));
            }
        }

        Ok(result)
    }

    /// Validate that input variables can be used with the format string.
    ///
    /// # Arguments
    ///
    /// * `format_string` - The format string to validate.
    /// * `input_variables` - The input variables that will be provided.
    ///
    /// # Returns
    ///
    /// Ok(()) if validation passes, or an error if any input variables are not used.
    ///
    /// # Example
    ///
    /// ```
    /// use agent_chain_core::utils::formatting::StrictFormatter;
    ///
    /// let formatter = StrictFormatter::new();
    /// let result = formatter.validate_input_variables("Hello, {name}!", &["name".to_string()]);
    /// assert!(result.is_ok());
    /// ```
    pub fn validate_input_variables(
        &self,
        format_string: &str,
        input_variables: &[String],
    ) -> Result<(), FormattingError> {
        let mut dummy_inputs = HashMap::new();
        for var in input_variables {
            dummy_inputs.insert(var.clone(), "foo".to_string());
        }

        self.format(format_string, &dummy_inputs).map(|_| ())
    }

    /// Extract placeholders from a format string.
    ///
    /// # Arguments
    ///
    /// * `format_string` - The format string to extract placeholders from.
    ///
    /// # Returns
    ///
    /// A set of placeholder names found in the format string.
    pub fn extract_placeholders(&self, format_string: &str) -> HashSet<String> {
        let mut placeholders = HashSet::new();
        let mut chars = format_string.chars().peekable();
        let mut in_placeholder = false;
        let mut current_placeholder = String::new();

        while let Some(c) = chars.next() {
            match c {
                '{' => {
                    if chars.peek() == Some(&'{') {
                        chars.next();
                    } else {
                        in_placeholder = true;
                        current_placeholder.clear();
                    }
                }
                '}' => {
                    if in_placeholder {
                        if !current_placeholder.is_empty() {
                            let name = current_placeholder.split(':').next().unwrap_or("");
                            let name = name.split('!').next().unwrap_or("");
                            if !name.is_empty() {
                                placeholders.insert(name.to_string());
                            }
                        }
                        in_placeholder = false;
                        current_placeholder.clear();
                    } else if chars.peek() == Some(&'}') {
                        chars.next();
                    }
                }
                _ => {
                    if in_placeholder {
                        current_placeholder.push(c);
                    }
                }
            }
        }

        placeholders
    }
}

/// A global formatter instance.
pub static FORMATTER: std::sync::LazyLock<StrictFormatter> =
    std::sync::LazyLock::new(StrictFormatter::new);

/// Format a string using the global formatter.
///
/// This is a convenience function that uses the global [`FORMATTER`] instance.
///
/// # Arguments
///
/// * `format_string` - The format string with placeholders.
/// * `kwargs` - The keyword arguments to substitute.
///
/// # Returns
///
/// The formatted string, or an error if formatting fails.
pub fn format_string(
    format_string: &str,
    kwargs: &HashMap<String, String>,
) -> Result<String, FormattingError> {
    FORMATTER.format(format_string, kwargs)
}

/// Error types for formatting operations.
#[derive(Debug, Clone, PartialEq)]
pub enum FormattingError {
    /// A required key was missing from the kwargs.
    MissingKey(String),
    /// An invalid format string was provided.
    InvalidFormat(String),
}

impl std::fmt::Display for FormattingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FormattingError::MissingKey(key) => {
                write!(f, "Missing key in format string: {}", key)
            }
            FormattingError::InvalidFormat(msg) => {
                write!(f, "Invalid format string: {}", msg)
            }
        }
    }
}

impl std::error::Error for FormattingError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_basic() {
        let formatter = StrictFormatter::new();
        let mut kwargs = HashMap::new();
        kwargs.insert("name".to_string(), "World".to_string());

        let result = formatter.format("Hello, {name}!", &kwargs).unwrap();
        assert_eq!(result, "Hello, World!");
    }

    #[test]
    fn test_format_multiple() {
        let formatter = StrictFormatter::new();
        let mut kwargs = HashMap::new();
        kwargs.insert("first".to_string(), "John".to_string());
        kwargs.insert("last".to_string(), "Doe".to_string());

        let result = formatter.format("{first} {last}", &kwargs).unwrap();
        assert_eq!(result, "John Doe");
    }

    #[test]
    fn test_format_missing_key() {
        let formatter = StrictFormatter::new();
        let kwargs = HashMap::new();

        let result = formatter.format("Hello, {name}!", &kwargs);
        assert!(matches!(result, Err(FormattingError::MissingKey(_))));
    }

    #[test]
    fn test_extract_placeholders() {
        let formatter = StrictFormatter::new();

        let placeholders =
            formatter.extract_placeholders("Hello, {name}! You are {age} years old.");
        assert!(placeholders.contains("name"));
        assert!(placeholders.contains("age"));
        assert_eq!(placeholders.len(), 2);
    }

    #[test]
    fn test_extract_placeholders_escaped() {
        let formatter = StrictFormatter::new();

        let placeholders = formatter.extract_placeholders("Hello, {{name}}!");
        assert!(placeholders.is_empty());
    }

    #[test]
    fn test_validate_input_variables() {
        let formatter = StrictFormatter::new();

        let result = formatter.validate_input_variables("Hello, {name}!", &["name".to_string()]);
        assert!(result.is_ok());

        let result = formatter.validate_input_variables("Hello, {name}!", &[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_format_string_function() {
        let mut kwargs = HashMap::new();
        kwargs.insert("greeting".to_string(), "Hi".to_string());

        let result = format_string("{greeting}!", &kwargs).unwrap();
        assert_eq!(result, "Hi!");
    }
}
