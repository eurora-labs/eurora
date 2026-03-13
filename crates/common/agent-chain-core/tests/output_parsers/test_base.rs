use std::sync::atomic::{AtomicBool, Ordering};

use agent_chain_core::error::{Error, Result};
use agent_chain_core::messages::{AIMessage, AnyMessage, HumanMessage};
use agent_chain_core::output_parsers::{BaseLLMOutputParser, BaseOutputParser};
use agent_chain_core::outputs::ChatGeneration;

#[derive(Debug)]
struct IntParser;

impl BaseOutputParser for IntParser {
    type Output = i64;

    fn parse(&self, text: &str) -> Result<i64> {
        text.trim()
            .parse::<i64>()
            .map_err(|_| Error::Other(format!("Cannot parse '{}' to int", text)))
    }

    fn parser_type(&self) -> &str {
        "int_parser"
    }
}

#[derive(Debug)]
struct BoolParser {
    true_val: String,
    false_val: String,
}

impl BoolParser {
    fn new() -> Self {
        Self {
            true_val: "YES".to_string(),
            false_val: "NO".to_string(),
        }
    }
}

impl BaseOutputParser for BoolParser {
    type Output = bool;

    fn parse(&self, text: &str) -> Result<bool> {
        let cleaned = text.trim().to_uppercase();
        if cleaned == self.true_val.to_uppercase() {
            return Ok(true);
        }
        if cleaned == self.false_val.to_uppercase() {
            return Ok(false);
        }
        Err(Error::Other(format!(
            "Expected {} or {}, got '{}'",
            self.true_val, self.false_val, text
        )))
    }

    fn parser_type(&self) -> &str {
        "bool_parser"
    }
}

#[derive(Debug)]
struct NoTypeParser;

impl BaseOutputParser for NoTypeParser {
    type Output = String;

    fn parse(&self, text: &str) -> Result<String> {
        Ok(text.to_string())
    }

    fn parser_type(&self) -> &str {
        "no_type_parser"
    }

    fn get_format_instructions(&self) -> Result<String> {
        Err(Error::NotImplemented(
            "_type property is not implemented".to_string(),
        ))
    }
}

#[test]
fn test_parse_valid_int() {
    let parser = IntParser;
    assert_eq!(parser.parse("42").unwrap(), 42);
}

#[test]
fn test_parse_invalid_int_raises() {
    let parser = IntParser;
    let result = parser.parse("not_a_number");
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("Cannot parse"), "Error was: {}", err);
}

#[test]
fn test_parse_with_whitespace() {
    let parser = IntParser;
    assert_eq!(parser.parse("  42  ").unwrap(), 42);
}

#[test]
fn test_parse_result_uses_first_generation() {
    let parser = IntParser;
    let generations = vec![
        ChatGeneration::builder()
            .message(AIMessage::builder().content("10").build().into())
            .build(),
        ChatGeneration::builder()
            .message(AIMessage::builder().content("20").build().into())
            .build(),
    ];
    let result = parser.parse_result(&generations, false).unwrap();
    assert_eq!(result, 10);
}

#[test]
fn test_parse_result_single_generation() {
    let parser = IntParser;
    let result = parser
        .parse_result(
            &[ChatGeneration::builder()
                .message(AIMessage::builder().content("99").build().into())
                .build()],
            false,
        )
        .unwrap();
    assert_eq!(result, 99);
}

#[test]
fn test_parse_result_with_chat_generation() {
    let parser = IntParser;
    let message: AnyMessage = AIMessage::builder().content("55").build().into();
    let chat_gen = ChatGeneration::builder().message(message).build();
    let result = parser.parse_result(&[chat_gen], false).unwrap();
    assert_eq!(result, 55);
}

#[test]
fn test_bool_parser_true() {
    let parser = BoolParser::new();
    assert!(parser.parse("YES").unwrap());
}

#[test]
fn test_bool_parser_false() {
    let parser = BoolParser::new();
    assert!(!parser.parse("NO").unwrap());
}

#[test]
fn test_bool_parser_case_insensitive() {
    let parser = BoolParser::new();
    assert!(parser.parse("yes").unwrap());
    assert!(!parser.parse("no").unwrap());
}

#[test]
fn test_bool_parser_invalid() {
    let parser = BoolParser::new();
    let result = parser.parse("MAYBE");
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("Expected"), "Error was: {}", err);
}

#[test]
fn test_invoke_with_ai_message() {
    let parser = IntParser;
    let message: AnyMessage = AIMessage::builder().content("42").build().into();
    assert_eq!(parser.invoke(message, None).unwrap(), 42);
}

#[test]
fn test_invoke_with_human_message() {
    let parser = IntParser;
    let message: AnyMessage = HumanMessage::builder().content("42").build().into();
    assert_eq!(parser.invoke(message, None).unwrap(), 42);
}

#[test]
fn test_invoke_with_ai_message_content() {
    let parser = IntParser;
    let message: AnyMessage = AIMessage::builder().content("42").build().into();
    let result = parser.invoke(message, None).unwrap();
    assert_eq!(result, 42);
}

#[test]
fn test_parse_result_partial_flag() {
    let parser = IntParser;
    let result = parser
        .parse_result(
            &[ChatGeneration::builder()
                .message(AIMessage::builder().content("42").build().into())
                .build()],
            true,
        )
        .unwrap();
    assert_eq!(result, 42);
}

#[test]
fn test_parse_with_prompt_ignores_prompt() {
    let parser = IntParser;
    let prompt = vec![AnyMessage::HumanMessage(
        HumanMessage::builder().content("Give me a number").build(),
    )];
    let result = parser.parse_with_prompt("42", &prompt).unwrap();
    assert_eq!(result, 42);
}

#[test]
fn test_get_format_instructions_returns_error() {
    let parser = IntParser;
    let result = parser.get_format_instructions();
    assert!(result.is_err());
}

#[test]
fn test_parser_type_returns_value() {
    let parser = IntParser;
    assert_eq!(parser.parser_type(), "int_parser");
}

#[test]
fn test_bool_parser_type_returns_value() {
    let parser = BoolParser::new();
    assert_eq!(parser.parser_type(), "bool_parser");
}

#[test]
fn test_no_type_parser_parse() {
    let parser = NoTypeParser;
    assert_eq!(parser.parse("hello").unwrap(), "hello");
}

#[test]
fn test_no_type_parser_get_format_instructions_returns_not_implemented() {
    let parser = NoTypeParser;
    let result = parser.get_format_instructions();
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("not implemented"), "Error was: {}", err);
}

#[derive(Debug)]
struct SimpleParser;

impl BaseLLMOutputParser for SimpleParser {
    type Output = String;

    fn parse_result(&self, result: &[ChatGeneration], _partial: bool) -> Result<String> {
        Ok(result[0].message.text().to_uppercase())
    }
}

#[test]
fn test_base_llm_parse_result_delegates() {
    let parser = SimpleParser;
    let result = parser
        .parse_result(
            &[ChatGeneration::builder()
                .message(AIMessage::builder().content("hello").build().into())
                .build()],
            false,
        )
        .unwrap();
    assert_eq!(result, "HELLO");
}

#[derive(Debug)]
struct PartialTracker {
    received_partial: AtomicBool,
}

impl PartialTracker {
    fn new() -> Self {
        Self {
            received_partial: AtomicBool::new(false),
        }
    }
}

impl BaseLLMOutputParser for PartialTracker {
    type Output = String;

    fn parse_result(&self, result: &[ChatGeneration], partial: bool) -> Result<String> {
        self.received_partial.store(partial, Ordering::SeqCst);
        Ok(result[0].message.text())
    }
}

#[test]
fn test_base_llm_parse_result_partial_flag() {
    let parser = PartialTracker::new();
    parser
        .parse_result(
            &[ChatGeneration::builder()
                .message(AIMessage::builder().content("test").build().into())
                .build()],
            true,
        )
        .unwrap();
    assert!(parser.received_partial.load(Ordering::SeqCst));
}
