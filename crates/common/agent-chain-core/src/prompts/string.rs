//! Base string prompt template.
//!
//! This module provides the base string prompt template and formatting utilities,
//! mirroring `langchain_core.prompts.string` in Python.

use std::collections::{HashMap, HashSet};

use crate::error::{Error, Result};
use crate::utils::formatting::{FORMATTER, FormattingError};
use crate::utils::mustache::{MustacheValue, render as mustache_render};

/// Template format types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PromptTemplateFormat {
    /// F-string format using `{variable}` syntax.
    #[default]
    FString,
    /// Mustache format using `{{variable}}` syntax.
    Mustache,
    /// Jinja2 format (requires jinja2 feature).
    Jinja2,
}

impl std::str::FromStr for PromptTemplateFormat {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "f-string" | "fstring" | "f_string" => Ok(Self::FString),
            "mustache" => Ok(Self::Mustache),
            "jinja2" => Ok(Self::Jinja2),
            _ => Err(Error::InvalidConfig(format!(
                "Invalid template format: {}. Expected one of: f-string, mustache, jinja2",
                s
            ))),
        }
    }
}

impl PromptTemplateFormat {
    /// Convert to a string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::FString => "f-string",
            Self::Mustache => "mustache",
            Self::Jinja2 => "jinja2",
        }
    }
}

impl std::fmt::Display for PromptTemplateFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl serde::Serialize for PromptTemplateFormat {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> serde::Deserialize<'de> for PromptTemplateFormat {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use std::str::FromStr;
        let s = String::deserialize(deserializer)?;
        Self::from_str(&s).map_err(serde::de::Error::custom)
    }
}

/// Format a template using jinja2.
///
/// **Security warning**: Jinja2 templates can execute arbitrary code.
/// Never use jinja2 templates from untrusted sources.
///
/// # Arguments
///
/// * `template` - The template string.
/// * `kwargs` - The keyword arguments to substitute.
///
/// # Returns
///
/// The formatted string, or an error if formatting fails.
pub fn jinja2_formatter(template: &str, kwargs: &HashMap<String, String>) -> Result<String> {
    let mut result = template.to_string();

    for (key, value) in kwargs {
        let pattern = format!("{{{{ {} }}}}", key);
        result = result.replace(&pattern, value);

        let pattern_no_space = format!("{{{{{}}}}}", key);
        result = result.replace(&pattern_no_space, value);
    }

    Ok(result)
}

/// Format a template using mustache.
///
/// # Arguments
///
/// * `template` - The template string.
/// * `kwargs` - The keyword arguments to substitute.
///
/// # Returns
///
/// The formatted string, or an error if formatting fails.
pub fn mustache_formatter(template: &str, kwargs: &HashMap<String, String>) -> Result<String> {
    let mut data = HashMap::new();
    for (key, value) in kwargs {
        data.insert(key.clone(), MustacheValue::String(value.clone()));
    }

    mustache_render(template, &MustacheValue::Map(data), None)
        .map_err(|e| Error::Other(format!("Mustache error: {}", e)))
}

/// Validate that input variables match the template for jinja2.
///
/// Issues a warning if missing or extra variables are found.
///
/// # Arguments
///
/// * `template` - The template string.
/// * `input_variables` - The input variables to validate.
pub fn validate_jinja2(template: &str, input_variables: &[String]) -> Result<()> {
    let template_vars = get_jinja2_variables(template);
    let input_set: HashSet<_> = input_variables.iter().cloned().collect();

    let missing: Vec<_> = template_vars.difference(&input_set).collect();
    let extra: Vec<_> = input_set.difference(&template_vars).collect();

    if !missing.is_empty() || !extra.is_empty() {
        let mut warning = String::new();
        if !missing.is_empty() {
            warning.push_str(&format!("Missing variables: {:?} ", missing));
        }
        if !extra.is_empty() {
            warning.push_str(&format!("Extra variables: {:?}", extra));
        }
        tracing::warn!(target: "agent_chain_core::prompts", "{}", warning.trim());
    }

    Ok(())
}

/// Get variables from a jinja2 template.
fn get_jinja2_variables(template: &str) -> HashSet<String> {
    let mut variables = HashSet::new();
    let mut chars = template.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '{' && chars.peek() == Some(&'{') {
            chars.next(); // consume second '{'

            while chars.peek() == Some(&' ') {
                chars.next();
            }

            let mut var_name = String::new();
            while let Some(&c) = chars.peek() {
                if c == '}' || c == ' ' || c == '|' || c == '.' {
                    break;
                }
                var_name.push(c);
                chars.next();
            }

            if !var_name.is_empty() && !var_name.starts_with('%') && !var_name.starts_with('#') {
                variables.insert(var_name);
            }
        }
    }

    variables
}

/// Get the top-level variables from a mustache template.
///
/// For nested variables like `{{person.name}}`, only the top-level
/// key (`person`) is returned.
pub fn mustache_template_vars(template: &str) -> HashSet<String> {
    let mut variables = HashSet::new();
    let mut chars = template.chars().peekable();
    let mut section_depth = 0;

    while let Some(c) = chars.next() {
        if c == '{' && chars.peek() == Some(&'{') {
            chars.next(); // consume second '{'

            let first_char = chars.peek().cloned();

            match first_char {
                Some('#') | Some('^') => {
                    section_depth += 1;
                    while let Some(&c) = chars.peek() {
                        if c == '}' {
                            break;
                        }
                        chars.next();
                    }
                }
                Some('/') => {
                    section_depth -= 1;
                    while let Some(&c) = chars.peek() {
                        if c == '}' {
                            break;
                        }
                        chars.next();
                    }
                }
                Some('!') | Some('>') => {
                    while let Some(&c) = chars.peek() {
                        if c == '}' {
                            break;
                        }
                        chars.next();
                    }
                }
                Some('{') => {
                    chars.next();
                    let mut var_name = String::new();
                    while let Some(&c) = chars.peek() {
                        if c == '}' {
                            break;
                        }
                        var_name.push(c);
                        chars.next();
                    }
                    let var_name = var_name.trim();
                    if !var_name.is_empty() && var_name != "." && section_depth == 0 {
                        let top_level = var_name.split('.').next().unwrap_or(var_name);
                        variables.insert(top_level.to_string());
                    }
                }
                Some('&') => {
                    chars.next();
                    let mut var_name = String::new();
                    while let Some(&c) = chars.peek() {
                        if c == '}' {
                            break;
                        }
                        var_name.push(c);
                        chars.next();
                    }
                    let var_name = var_name.trim();
                    if !var_name.is_empty() && var_name != "." && section_depth == 0 {
                        let top_level = var_name.split('.').next().unwrap_or(var_name);
                        variables.insert(top_level.to_string());
                    }
                }
                _ => {
                    let mut var_name = String::new();
                    while let Some(&c) = chars.peek() {
                        if c == '}' {
                            break;
                        }
                        var_name.push(c);
                        chars.next();
                    }
                    let var_name = var_name.trim();
                    if !var_name.is_empty() && var_name != "." && section_depth == 0 {
                        let top_level = var_name.split('.').next().unwrap_or(var_name);
                        variables.insert(top_level.to_string());
                    }
                }
            }
        }
    }

    variables
}

/// Check that template string is valid.
///
/// # Arguments
///
/// * `template` - The template string.
/// * `template_format` - The template format.
/// * `input_variables` - The input variables.
///
/// # Returns
///
/// Ok(()) if valid, or an error if invalid.
pub fn check_valid_template(
    template: &str,
    template_format: PromptTemplateFormat,
    input_variables: &[String],
) -> Result<()> {
    match template_format {
        PromptTemplateFormat::FString => FORMATTER
            .validate_input_variables(template, input_variables)
            .map_err(|e| match e {
                FormattingError::MissingKey(key) => Error::InvalidConfig(format!(
                    "Invalid prompt schema; missing input parameter: {}",
                    key
                )),
                FormattingError::InvalidFormat(msg) => {
                    Error::InvalidConfig(format!("Invalid format string: {}", msg))
                }
            }),
        PromptTemplateFormat::Jinja2 => validate_jinja2(template, input_variables),
        PromptTemplateFormat::Mustache => {
            Ok(())
        }
    }
}

/// Get the variables from the template.
///
/// # Arguments
///
/// * `template` - The template string.
/// * `template_format` - The template format.
///
/// # Returns
///
/// A sorted list of variable names from the template.
pub fn get_template_variables(
    template: &str,
    template_format: PromptTemplateFormat,
) -> Result<Vec<String>> {
    let variables: HashSet<String> = match template_format {
        PromptTemplateFormat::FString => {
            let placeholders = FORMATTER.extract_placeholders(template);
            for var in &placeholders {
                if var.contains('.') || var.contains('[') || var.contains(']') {
                    return Err(Error::InvalidConfig(format!(
                        "Invalid variable name '{}' in f-string template. \
                         Variable names cannot contain attribute access (.) or indexing ([]).",
                        var
                    )));
                }
                if var.chars().all(|c| c.is_ascii_digit()) {
                    return Err(Error::InvalidConfig(format!(
                        "Invalid variable name '{}' in f-string template. \
                         Variable names cannot be all digits as they are interpreted as positional arguments.",
                        var
                    )));
                }
            }
            placeholders
        }
        PromptTemplateFormat::Jinja2 => get_jinja2_variables(template),
        PromptTemplateFormat::Mustache => mustache_template_vars(template),
    };

    let mut vars: Vec<_> = variables.into_iter().collect();
    vars.sort();
    Ok(vars)
}

/// Format a template string with the given format and kwargs.
pub fn format_template(
    template: &str,
    template_format: PromptTemplateFormat,
    kwargs: &HashMap<String, String>,
) -> Result<String> {
    match template_format {
        PromptTemplateFormat::FString => FORMATTER.format(template, kwargs).map_err(|e| match e {
            FormattingError::MissingKey(key) => {
                Error::InvalidConfig(format!("Missing key in format string: {}", key))
            }
            FormattingError::InvalidFormat(msg) => {
                Error::InvalidConfig(format!("Invalid format string: {}", msg))
            }
        }),
        PromptTemplateFormat::Mustache => mustache_formatter(template, kwargs),
        PromptTemplateFormat::Jinja2 => jinja2_formatter(template, kwargs),
    }
}

/// Trait for string prompt templates.
///
/// String prompt templates format to a string (as opposed to a list of messages).
pub trait StringPromptTemplate: Send + Sync {
    /// Get the input variables for this template.
    fn input_variables(&self) -> &[String];

    /// Get the optional variables for this template.
    fn optional_variables(&self) -> &[String] {
        &[]
    }

    /// Get partial variables for this template.
    fn partial_variables(&self) -> &HashMap<String, String> {
        static EMPTY: std::sync::LazyLock<HashMap<String, String>> =
            std::sync::LazyLock::new(HashMap::new);
        &EMPTY
    }

    /// Get the template format.
    fn template_format(&self) -> PromptTemplateFormat {
        PromptTemplateFormat::FString
    }

    /// Format the prompt with the inputs.
    ///
    /// # Arguments
    ///
    /// * `kwargs` - The keyword arguments to format the template with.
    ///
    /// # Returns
    ///
    /// A formatted string, or an error if formatting fails.
    fn format(&self, kwargs: &HashMap<String, String>) -> Result<String>;

    /// Async format the prompt with the inputs.
    ///
    /// Default implementation calls the sync version.
    fn aformat(
        &self,
        kwargs: &HashMap<String, String>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String>> + Send + '_>> {
        let result = self.format(kwargs);
        Box::pin(async move { result })
    }

    /// Get a pretty representation of the prompt.
    fn pretty_repr(&self, html: bool) -> String;

    /// Print a pretty representation of the prompt.
    fn pretty_print(&self) {
        println!("{}", self.pretty_repr(false));
    }
}

/// Check if a value is a subsequence of another sequence.
///
/// This function checks if `child` is a prefix of `parent`.
/// Part of the Python langchain_core API.
#[allow(dead_code)]
pub fn is_subsequence<T: PartialEq>(child: &[T], parent: &[T]) -> bool {
    if child.is_empty() || parent.is_empty() {
        return false;
    }
    if parent.len() < child.len() {
        return false;
    }
    child.iter().zip(parent.iter()).all(|(c, p)| c == p)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_format_from_str() {
        use std::str::FromStr;
        assert_eq!(
            PromptTemplateFormat::from_str("f-string").unwrap(),
            PromptTemplateFormat::FString
        );
        assert_eq!(
            PromptTemplateFormat::from_str("mustache").unwrap(),
            PromptTemplateFormat::Mustache
        );
        assert_eq!(
            PromptTemplateFormat::from_str("jinja2").unwrap(),
            PromptTemplateFormat::Jinja2
        );
    }

    #[test]
    fn test_get_template_variables_fstring() {
        let vars = get_template_variables(
            "Hello, {name}! You are {age} years old.",
            PromptTemplateFormat::FString,
        )
        .unwrap();
        assert!(vars.contains(&"name".to_string()));
        assert!(vars.contains(&"age".to_string()));
        assert_eq!(vars.len(), 2);
    }

    #[test]
    fn test_get_template_variables_mustache() {
        let vars = get_template_variables(
            "Hello, {{name}}! You are {{age}} years old.",
            PromptTemplateFormat::Mustache,
        )
        .unwrap();
        assert!(vars.contains(&"name".to_string()));
        assert!(vars.contains(&"age".to_string()));
        assert_eq!(vars.len(), 2);
    }

    #[test]
    fn test_format_template_fstring() {
        let mut kwargs = HashMap::new();
        kwargs.insert("name".to_string(), "World".to_string());

        let result =
            format_template("Hello, {name}!", PromptTemplateFormat::FString, &kwargs).unwrap();
        assert_eq!(result, "Hello, World!");
    }

    #[test]
    fn test_format_template_mustache() {
        let mut kwargs = HashMap::new();
        kwargs.insert("name".to_string(), "World".to_string());

        let result =
            format_template("Hello, {{name}}!", PromptTemplateFormat::Mustache, &kwargs).unwrap();
        assert_eq!(result, "Hello, World!");
    }

    #[test]
    fn test_invalid_fstring_variable() {
        let result = get_template_variables("Hello {obj.attr}", PromptTemplateFormat::FString);
        assert!(result.is_err());
    }

    #[test]
    fn test_is_subsequence() {
        assert!(is_subsequence(&[1, 2], &[1, 2, 3]));
        assert!(!is_subsequence(&[1, 3], &[1, 2, 3]));
        assert!(!is_subsequence(&[1, 2, 3, 4], &[1, 2, 3]));
    }
}
