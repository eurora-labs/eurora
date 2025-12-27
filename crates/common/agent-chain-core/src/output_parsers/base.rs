//! Base parser for language model outputs.
//!
//! This module contains the base traits and types for output parsers,
//! mirroring `langchain_core.output_parsers.base`.

use std::fmt::Debug;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::{Error, Result};
use crate::messages::BaseMessage;
use crate::outputs::{ChatGeneration, Generation};
use crate::prompt_values::PromptValue;
use crate::runnables::RunnableConfig;

/// Abstract base trait for parsing the outputs of a model.
///
/// This is the most basic output parser trait. It requires implementing
/// `parse_result` which takes a list of candidate `Generation` objects
/// and parses them into a specific format.
#[async_trait]
pub trait BaseLLMOutputParser: Send + Sync + Debug {
    /// The output type of this parser.
    type Output: Send + Sync + Clone + Debug;

    /// Parse a list of candidate model `Generation` objects into a specific format.
    ///
    /// # Arguments
    ///
    /// * `result` - A list of `Generation` to be parsed. The `Generation` objects are
    ///   assumed to be different candidate outputs for a single model input.
    /// * `partial` - Whether to parse the output as a partial result. This is useful
    ///   for parsers that can parse partial results.
    ///
    /// # Returns
    ///
    /// Structured output.
    fn parse_result(&self, result: &[Generation], partial: bool) -> Result<Self::Output>;

    /// Async parse a list of candidate model `Generation` objects into a specific format.
    ///
    /// Default implementation calls the sync version.
    async fn aparse_result(&self, result: &[Generation], partial: bool) -> Result<Self::Output> {
        self.parse_result(result, partial)
    }
}

/// Base trait to parse the output of an LLM call.
///
/// `BaseGenerationOutputParser` extends `BaseLLMOutputParser` and integrates with
/// the Runnable interface. It processes raw generation outputs from language models.
#[async_trait]
pub trait BaseGenerationOutputParser: BaseLLMOutputParser {
    /// Invoke the parser on a string or message input.
    ///
    /// For string inputs, creates a `Generation` with the text.
    /// For message inputs, creates a `ChatGeneration` with the message,
    /// matching the Python implementation.
    ///
    /// # Arguments
    ///
    /// * `input` - Either a string or a BaseMessage.
    /// * `config` - Optional runnable configuration.
    fn invoke(&self, input: BaseMessage, _config: Option<RunnableConfig>) -> Result<Self::Output> {
        // Match Python: use ChatGeneration for message inputs
        let chat_gen = ChatGeneration::new(input);
        self.parse_result(&[Generation::new(&chat_gen.text)], false)
    }

    /// Async invoke the parser on a string or message input.
    async fn ainvoke(
        &self,
        input: BaseMessage,
        config: Option<RunnableConfig>,
    ) -> Result<Self::Output> {
        self.invoke(input, config)
    }
}

/// Base trait to parse the output of an LLM call.
///
/// Output parsers help structure language model responses.
/// This is the main trait that most output parsers implement.
///
/// # Example
///
/// ```ignore
/// struct BooleanOutputParser {
///     true_val: String,
///     false_val: String,
/// }
///
/// impl BaseOutputParser for BooleanOutputParser {
///     type Output = bool;
///
///     fn parse(&self, text: &str) -> Result<bool> {
///         let cleaned_text = text.trim().to_uppercase();
///         if cleaned_text == self.true_val.to_uppercase() {
///             Ok(true)
///         } else if cleaned_text == self.false_val.to_uppercase() {
///             Ok(false)
///         } else {
///             Err(OutputParserError::parse_error(format!(
///                 "Expected {} or {}, got {}",
///                 self.true_val, self.false_val, cleaned_text
///             )).into())
///         }
///     }
///
///     fn parser_type(&self) -> &str {
///         "boolean_output_parser"
///     }
/// }
/// ```
#[async_trait]
pub trait BaseOutputParser: Send + Sync + Debug {
    /// The output type of this parser.
    type Output: Send + Sync + Clone + Debug;

    /// Parse a single string model output into some structure.
    ///
    /// # Arguments
    ///
    /// * `text` - String output of a language model.
    ///
    /// # Returns
    ///
    /// Structured output.
    fn parse(&self, text: &str) -> Result<Self::Output>;

    /// Async parse a single string model output into some structure.
    ///
    /// Default implementation calls the sync version.
    async fn aparse(&self, text: &str) -> Result<Self::Output> {
        self.parse(text)
    }

    /// Parse a list of candidate model `Generation` objects into a specific format.
    ///
    /// The return value is parsed from only the first `Generation` in the result,
    /// which is assumed to be the highest-likelihood `Generation`.
    ///
    /// # Arguments
    ///
    /// * `result` - A list of `Generation` to be parsed.
    /// * `partial` - Whether to parse the output as a partial result.
    ///
    /// # Panics
    ///
    /// This method will panic if `result` is empty, matching the Python behavior
    /// which raises an IndexError when accessing `result[0]` on an empty list.
    fn parse_result(&self, result: &[Generation], _partial: bool) -> Result<Self::Output> {
        // Match Python behavior: access result[0] directly (panics if empty)
        self.parse(&result[0].text)
    }

    /// Async parse a list of candidate model `Generation` objects into a specific format.
    async fn aparse_result(&self, result: &[Generation], partial: bool) -> Result<Self::Output> {
        self.parse_result(result, partial)
    }

    /// Parse the output of an LLM call with the input prompt for context.
    ///
    /// The prompt is largely provided in the event the `OutputParser` wants
    /// to retry or fix the output in some way, and needs information from
    /// the prompt to do so.
    ///
    /// # Arguments
    ///
    /// * `completion` - String output of a language model.
    /// * `prompt` - Input `PromptValue`.
    fn parse_with_prompt(
        &self,
        completion: &str,
        _prompt: &dyn PromptValue,
    ) -> Result<Self::Output> {
        self.parse(completion)
    }

    /// Instructions on how the LLM output should be formatted.
    ///
    /// # Errors
    ///
    /// Returns an error if format instructions are not implemented for this parser.
    /// Subclasses should override this method to provide format instructions.
    fn get_format_instructions(&self) -> Result<String> {
        Err(Error::Other(
            "get_format_instructions not implemented".to_string(),
        ))
    }

    /// Return the output parser type for serialization.
    fn parser_type(&self) -> &str;

    /// Invoke the parser on input.
    ///
    /// For string inputs, creates a `Generation` with the text.
    /// For message inputs, creates a `ChatGeneration` with the message,
    /// matching the Python implementation.
    fn invoke(&self, input: BaseMessage, _config: Option<RunnableConfig>) -> Result<Self::Output> {
        // Match Python: use ChatGeneration for message inputs
        let chat_gen = ChatGeneration::new(input);
        // ChatGeneration has a text field that extracts content from message
        self.parse_result(&[Generation::new(&chat_gen.text)], false)
    }

    /// Async invoke the parser on input.
    async fn ainvoke(
        &self,
        input: BaseMessage,
        config: Option<RunnableConfig>,
    ) -> Result<Self::Output> {
        self.invoke(input, config)
    }
}

/// Error type for output parser operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputParserError {
    /// The error message.
    pub message: String,
    /// The raw LLM output that caused the error.
    pub llm_output: Option<String>,
    /// Whether the error is retryable.
    pub send_to_llm: bool,
    /// Observation to send back to the LLM if retrying.
    pub observation: Option<String>,
}

impl OutputParserError {
    /// Create a new output parser error.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            llm_output: None,
            send_to_llm: false,
            observation: None,
        }
    }

    /// Create a parse error with the LLM output.
    pub fn parse_error(message: impl Into<String>, llm_output: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            llm_output: Some(llm_output.into()),
            send_to_llm: false,
            observation: None,
        }
    }

    /// Set whether this error should be sent back to the LLM.
    pub fn with_send_to_llm(mut self, send: bool) -> Self {
        self.send_to_llm = send;
        self
    }

    /// Set the observation to send back to the LLM.
    pub fn with_observation(mut self, observation: impl Into<String>) -> Self {
        self.observation = Some(observation.into());
        self
    }

    /// Set the LLM output.
    pub fn with_llm_output(mut self, llm_output: impl Into<String>) -> Self {
        self.llm_output = Some(llm_output.into());
        self
    }
}

impl std::fmt::Display for OutputParserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for OutputParserError {}

impl From<OutputParserError> for Error {
    fn from(err: OutputParserError) -> Self {
        Error::Other(err.message)
    }
}

/// Convert a Generation to a Value for JSON operations.
pub fn generation_to_value(generation: &Generation) -> Value {
    serde_json::json!({
        "text": generation.text,
        "generation_info": generation.generation_info,
    })
}

/// Convert a ChatGeneration to a Value for JSON operations.
pub fn chat_generation_to_value(generation: &ChatGeneration) -> Value {
    serde_json::json!({
        "text": generation.text,
        "message": generation.message,
        "generation_info": generation.generation_info,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct TestParser;

    impl BaseOutputParser for TestParser {
        type Output = String;

        fn parse(&self, text: &str) -> Result<String> {
            Ok(text.to_uppercase())
        }

        fn parser_type(&self) -> &str {
            "test"
        }
    }

    #[test]
    fn test_base_output_parser() {
        let parser = TestParser;
        let result = parser.parse("hello").unwrap();
        assert_eq!(result, "HELLO");
    }

    #[test]
    fn test_parse_result() {
        let parser = TestParser;
        let generations = vec![Generation::new("hello")];
        let result = parser.parse_result(&generations, false).unwrap();
        assert_eq!(result, "HELLO");
    }

    #[test]
    fn test_output_parser_error() {
        let err = OutputParserError::parse_error("Invalid JSON", "{invalid}");
        assert_eq!(err.message, "Invalid JSON");
        assert_eq!(err.llm_output, Some("{invalid}".to_string()));
    }
}
