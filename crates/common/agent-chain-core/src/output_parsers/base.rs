//! Base parser for language model outputs.
//!
//! This module contains the base traits and types for output parsers,
//! mirroring langchain_core.output_parsers.base.

use std::fmt::Debug;

use async_trait::async_trait;

use crate::error::{Error, Result};
use crate::messages::BaseMessage;
use crate::outputs::{ChatGeneration, Generation};
use crate::prompt_values::PromptValue;
use crate::runnables::RunnableConfig;
use crate::runnables::base::Runnable;

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
    /// Invoke the parser on a message input.
    fn invoke(&self, input: BaseMessage, _config: Option<RunnableConfig>) -> Result<Self::Output> {
        let chat_gen = ChatGeneration::new(input);
        self.parse_result(&[Generation::new(&chat_gen.text)], false)
    }

    /// Async invoke the parser on a message input.
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
#[async_trait]
pub trait BaseOutputParser: Send + Sync + Debug {
    /// The output type of this parser.
    type Output: Send + Sync + Clone + Debug;

    /// Parse a single string model output into some structure.
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
    fn parse_result(&self, result: &[Generation], _partial: bool) -> Result<Self::Output> {
        let first = result.first().ok_or_else(|| {
            Error::Other("parse_result called with empty result list".to_string())
        })?;
        self.parse(&first.text)
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
    fn parse_with_prompt(
        &self,
        completion: &str,
        _prompt: &dyn PromptValue,
    ) -> Result<Self::Output> {
        self.parse(completion)
    }

    /// Instructions on how the LLM output should be formatted.
    fn get_format_instructions(&self) -> Result<String> {
        Err(Error::Other(
            "get_format_instructions not implemented".to_string(),
        ))
    }

    /// Return the output parser type for serialization.
    fn parser_type(&self) -> &str;

    /// Invoke the parser on input.
    fn invoke(&self, input: BaseMessage, _config: Option<RunnableConfig>) -> Result<Self::Output> {
        let chat_gen = ChatGeneration::new(input);
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

    /// Convert this parser into a Runnable for use in chains.
    ///
    /// Mirrors Python where BaseOutputParser extends RunnableSerializable.
    fn into_runnable(self) -> RunnableOutputParser<Self>
    where
        Self: Sized,
    {
        RunnableOutputParser::new(self)
    }
}

/// Error type for output parser operations.
#[derive(Debug, Clone)]
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

/// Adapter that wraps a BaseOutputParser as a Runnable.
///
/// Mirrors Python where BaseOutputParser extends RunnableSerializable,
/// allowing parsers to participate in chain composition via `pipe()`.
pub struct RunnableOutputParser<P> {
    parser: P,
}

impl<P: BaseOutputParser> RunnableOutputParser<P> {
    /// Create a new RunnableOutputParser wrapping the given parser.
    pub fn new(parser: P) -> Self {
        Self { parser }
    }

    /// Get a reference to the inner parser.
    pub fn parser(&self) -> &P {
        &self.parser
    }

    /// Consume the adapter and return the inner parser.
    pub fn into_inner(self) -> P {
        self.parser
    }
}

impl<P: BaseOutputParser> Debug for RunnableOutputParser<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RunnableOutputParser")
            .field("parser", &self.parser)
            .finish()
    }
}

#[async_trait]
impl<P> Runnable for RunnableOutputParser<P>
where
    P: BaseOutputParser + 'static,
    P::Output: 'static,
{
    type Input = BaseMessage;
    type Output = P::Output;

    fn name(&self) -> Option<String> {
        Some(format!(
            "RunnableOutputParser<{}>",
            self.parser.parser_type()
        ))
    }

    fn invoke(&self, input: Self::Input, config: Option<RunnableConfig>) -> Result<Self::Output> {
        self.parser.invoke(input, config)
    }

    async fn ainvoke(
        &self,
        input: Self::Input,
        config: Option<RunnableConfig>,
    ) -> Result<Self::Output>
    where
        Self: 'static,
    {
        self.parser.ainvoke(input, config).await
    }
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
    fn test_parse_result_empty() {
        let parser = TestParser;
        let result = parser.parse_result(&[], false);
        assert!(result.is_err());
    }

    #[test]
    fn test_output_parser_error() {
        let err = OutputParserError::parse_error("Invalid JSON", "{invalid}");
        assert_eq!(err.message, "Invalid JSON");
        assert_eq!(err.llm_output, Some("{invalid}".to_string()));
    }
}
