//! Tests for the formatting module.
//!
//! These tests mirror the tests for `langchain_core/utils/formatting.py`.

use std::collections::HashMap;

use agent_chain_core::utils::formatting::{
    FORMATTER, FormattingError, StrictFormatter, format_string,
};

/// Test that the formatter can format a basic string with a single placeholder.
#[test]
fn test_format_basic() {
    let formatter = StrictFormatter::new();
    let mut kwargs = HashMap::new();
    kwargs.insert("name".to_string(), "World".to_string());

    let result = formatter.format("Hello, {name}!", &kwargs).unwrap();
    assert_eq!(result, "Hello, World!");
}

/// Test that the formatter can handle multiple placeholders.
#[test]
fn test_format_multiple_placeholders() {
    let formatter = StrictFormatter::new();
    let mut kwargs = HashMap::new();
    kwargs.insert("first".to_string(), "John".to_string());
    kwargs.insert("last".to_string(), "Doe".to_string());

    let result = formatter.format("{first} {last}", &kwargs).unwrap();
    assert_eq!(result, "John Doe");
}

/// Test that the formatter returns an error when a key is missing.
#[test]
fn test_format_missing_key() {
    let formatter = StrictFormatter::new();
    let kwargs = HashMap::new();

    let result = formatter.format("Hello, {name}!", &kwargs);
    assert!(matches!(result, Err(FormattingError::MissingKey(_))));

    if let Err(FormattingError::MissingKey(key)) = result {
        assert_eq!(key, "name");
    }
}

/// Test that the formatter can extract placeholders from a format string.
#[test]
fn test_extract_placeholders() {
    let formatter = StrictFormatter::new();

    let placeholders = formatter.extract_placeholders("Hello, {name}! You are {age} years old.");
    assert!(placeholders.contains("name"));
    assert!(placeholders.contains("age"));
    assert_eq!(placeholders.len(), 2);
}

/// Test that escaped braces ({{ and }}) are not treated as placeholders.
#[test]
fn test_extract_placeholders_escaped() {
    let formatter = StrictFormatter::new();

    let placeholders = formatter.extract_placeholders("Hello, {{name}}!");
    assert!(placeholders.is_empty());
}

/// Test that escaped braces are preserved in the output.
#[test]
fn test_format_escaped_braces() {
    let formatter = StrictFormatter::new();
    let kwargs = HashMap::new();

    let result = formatter.format("Hello, {{name}}!", &kwargs).unwrap();
    assert_eq!(result, "Hello, {{name}}!");
}

/// Test that validate_input_variables succeeds when all variables are provided.
#[test]
fn test_validate_input_variables_success() {
    let formatter = StrictFormatter::new();

    let result = formatter.validate_input_variables("Hello, {name}!", &["name".to_string()]);
    assert!(result.is_ok());
}

/// Test that validate_input_variables fails when a variable is missing.
#[test]
fn test_validate_input_variables_missing() {
    let formatter = StrictFormatter::new();

    let result = formatter.validate_input_variables("Hello, {name}!", &[]);
    assert!(result.is_err());
}

/// Test that validate_input_variables works with multiple variables.
#[test]
fn test_validate_input_variables_multiple() {
    let formatter = StrictFormatter::new();

    let result = formatter.validate_input_variables(
        "Hello, {first} {last}!",
        &["first".to_string(), "last".to_string()],
    );
    assert!(result.is_ok());
}

/// Test the global format_string function.
#[test]
fn test_format_string_function() {
    let mut kwargs = HashMap::new();
    kwargs.insert("greeting".to_string(), "Hi".to_string());

    let result = format_string("{greeting}!", &kwargs).unwrap();
    assert_eq!(result, "Hi!");
}

/// Test that the global FORMATTER instance works correctly.
#[test]
fn test_global_formatter() {
    let mut kwargs = HashMap::new();
    kwargs.insert("name".to_string(), "World".to_string());

    let result = FORMATTER.format("Hello, {name}!", &kwargs).unwrap();
    assert_eq!(result, "Hello, World!");
}

/// Test formatting with empty string values.
#[test]
fn test_format_empty_value() {
    let formatter = StrictFormatter::new();
    let mut kwargs = HashMap::new();
    kwargs.insert("name".to_string(), "".to_string());

    let result = formatter.format("Hello, {name}!", &kwargs).unwrap();
    assert_eq!(result, "Hello, !");
}

/// Test formatting with a string containing no placeholders.
#[test]
fn test_format_no_placeholders() {
    let formatter = StrictFormatter::new();
    let kwargs = HashMap::new();

    let result = formatter.format("Hello, World!", &kwargs).unwrap();
    assert_eq!(result, "Hello, World!");
}

/// Test formatting with repeated placeholders.
#[test]
fn test_format_repeated_placeholder() {
    let formatter = StrictFormatter::new();
    let mut kwargs = HashMap::new();
    kwargs.insert("name".to_string(), "World".to_string());

    let result = formatter
        .format("{name} says hello to {name}!", &kwargs)
        .unwrap();
    assert_eq!(result, "World says hello to World!");
}

/// Test that extra kwargs (not in the format string) are allowed.
/// This mirrors Python's behavior where extra kwargs are simply ignored.
#[test]
fn test_format_extra_kwargs() {
    let formatter = StrictFormatter::new();
    let mut kwargs = HashMap::new();
    kwargs.insert("name".to_string(), "World".to_string());
    kwargs.insert("extra".to_string(), "ignored".to_string());

    let result = formatter.format("Hello, {name}!", &kwargs).unwrap();
    assert_eq!(result, "Hello, World!");
}

/// Test extracting placeholders with format specifiers (e.g., {name:>10}).
#[test]
fn test_extract_placeholders_with_format_spec() {
    let formatter = StrictFormatter::new();

    let placeholders = formatter.extract_placeholders("Hello, {name:>10}!");
    assert!(placeholders.contains("name"));
    assert_eq!(placeholders.len(), 1);
}

/// Test extracting placeholders with conversion flags (e.g., {name!r}).
#[test]
fn test_extract_placeholders_with_conversion() {
    let formatter = StrictFormatter::new();

    let placeholders = formatter.extract_placeholders("Hello, {name!r}!");
    assert!(placeholders.contains("name"));
    assert_eq!(placeholders.len(), 1);
}

/// Test the Default implementation for StrictFormatter.
#[test]
fn test_formatter_default() {
    let formatter = StrictFormatter::new();
    let mut kwargs = HashMap::new();
    kwargs.insert("name".to_string(), "World".to_string());

    let result = formatter.format("Hello, {name}!", &kwargs).unwrap();
    assert_eq!(result, "Hello, World!");
}

/// Test FormattingError Display implementation.
#[test]
fn test_formatting_error_display() {
    let error = FormattingError::MissingKey("name".to_string());
    assert_eq!(error.to_string(), "Missing key in format string: name");

    let error = FormattingError::InvalidFormat("bad format".to_string());
    assert_eq!(error.to_string(), "Invalid format string: bad format");
}

/// Test formatting a complex template.
#[test]
fn test_format_complex_template() {
    let formatter = StrictFormatter::new();
    let mut kwargs = HashMap::new();
    kwargs.insert(
        "question".to_string(),
        "What is the capital of France?".to_string(),
    );
    kwargs.insert(
        "context".to_string(),
        "France is a country in Western Europe.".to_string(),
    );

    let template = "Given the following context:\n{context}\n\nAnswer the question: {question}";
    let result = formatter.format(template, &kwargs).unwrap();

    let expected = "Given the following context:\nFrance is a country in Western Europe.\n\nAnswer the question: What is the capital of France?";
    assert_eq!(result, expected);
}
