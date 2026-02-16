//! Output parser classes.
//!
//! `OutputParser` classes parse the output of an LLM call into structured data.
//!
//! Output parsers emerged as an early solution to the challenge of obtaining structured
//! output from LLMs. Today, most LLMs support structured output natively.
//! In such cases, using output parsers may be unnecessary, and you should
//! leverage the model's built-in capabilities for structured output.
//!
//! Output parsers remain valuable when working with models that do not support
//! structured output natively, or when you require additional processing or validation
//! of the model's output beyond its inherent capabilities.
//!
//! Mirrors `langchain_core.output_parsers`.
//!
//! # Available Parsers
//!
//! - [`StrOutputParser`] - Extract text content from model outputs as a string
//! - [`JsonOutputParser`] - Parse output as JSON
//! - [`CommaSeparatedListOutputParser`] - Parse comma-separated lists
//! - [`NumberedListOutputParser`] - Parse numbered lists
//! - [`MarkdownListOutputParser`] - Parse Markdown lists
//! - [`XMLOutputParser`] - Parse XML output
//!
//! # Traits
//!
//! - [`BaseOutputParser`] - Base trait for output parsers
//! - [`BaseLLMOutputParser`] - Low-level output parser trait
//! - [`BaseTransformOutputParser`] - Parser with streaming support
//! - [`BaseCumulativeTransformOutputParser`] - Parser that accumulates chunks
//!
//! # Example
//!
//! ```ignore
//! use agent_chain_core::output_parsers::{StrOutputParser, BaseOutputParser};
//!
//! let parser = StrOutputParser::new();
//! let result = parser.parse("Hello, world!").unwrap();
//! assert_eq!(result, "Hello, world!");
//! ```

mod base;
mod format_instructions;
mod json;
mod list;
mod openai_functions;
pub mod openai_tools;
mod pydantic;
mod string;
mod transform;
mod xml;

// Re-export base types
pub use base::{
    BaseGenerationOutputParser, BaseLLMOutputParser, BaseOutputParser, OutputParserError,
    RunnableOutputParser,
};

// Re-export format instructions
pub use format_instructions::JSON_FORMAT_INSTRUCTIONS;

// Re-export string parser
pub use string::StrOutputParser;

// Re-export transform types
pub use transform::{BaseCumulativeTransformOutputParser, BaseTransformOutputParser};

// Re-export JSON parser
pub use json::{JsonOutputParser, SimpleJsonOutputParser};

// Re-export list parsers
pub use list::{
    CommaSeparatedListOutputParser, ListOutputParser, MarkdownListOutputParser,
    NumberedListOutputParser, ParseMatch, drop_last_n,
};

// Re-export Pydantic (struct) parser
pub use pydantic::PydanticOutputParser;

// Re-export XML parser
pub use xml::{XMLOutputParser, nested_element};

// Re-export OpenAI functions parsers
pub use openai_functions::{
    JsonKeyOutputFunctionsParser, JsonOutputFunctionsParser, OutputFunctionsParser,
    PydanticAttrOutputFunctionsParser, PydanticOutputFunctionsParser, PydanticSchema,
};

// Re-export OpenAI tools parsers
pub use openai_tools::{
    JsonOutputKeyToolsParser, JsonOutputToolsParser, PydanticToolsParser, make_invalid_tool_call,
    parse_tool_call, parse_tool_calls,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_str_parser_export() {
        let parser = StrOutputParser::new();
        let result = parser.parse("test").unwrap();
        assert_eq!(result, "test");
    }

    #[test]
    fn test_json_parser_export() {
        let parser = JsonOutputParser::new();
        let result = parser.parse(r#"{"key": "value"}"#).unwrap();
        assert_eq!(result["key"], "value");
    }

    #[test]
    fn test_list_parser_export() {
        let parser = CommaSeparatedListOutputParser::new();
        let result = parser.parse("a, b, c").unwrap();
        assert_eq!(result, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_xml_parser_export() {
        let parser = XMLOutputParser::new();
        let result = parser.parse("<root>value</root>").unwrap();
        assert_eq!(result["root"], "value");
    }
}
