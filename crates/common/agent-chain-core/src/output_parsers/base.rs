use std::fmt::Debug;

use async_trait::async_trait;

use crate::error::{Error, Result};
use crate::messages::BaseMessage;
use crate::outputs::{ChatGeneration, Generation};
use crate::prompt_values::PromptValue;
use crate::runnables::RunnableConfig;
use crate::runnables::base::Runnable;

#[async_trait]
pub trait BaseLLMOutputParser: Send + Sync + Debug {
    type Output: Send + Sync + Clone + Debug;

    fn parse_result(&self, result: &[Generation], partial: bool) -> Result<Self::Output>;

    async fn aparse_result(&self, result: &[Generation], partial: bool) -> Result<Self::Output> {
        self.parse_result(result, partial)
    }
}

#[async_trait]
pub trait BaseGenerationOutputParser: BaseLLMOutputParser {
    fn invoke(&self, input: BaseMessage, _config: Option<RunnableConfig>) -> Result<Self::Output> {
        let chat_gen = ChatGeneration::builder().message(input).build();
        self.parse_result(&[Generation::builder().text(&chat_gen.text).build()], false)
    }

    async fn ainvoke(
        &self,
        input: BaseMessage,
        config: Option<RunnableConfig>,
    ) -> Result<Self::Output> {
        self.invoke(input, config)
    }
}

#[async_trait]
pub trait BaseOutputParser: Send + Sync + Debug {
    type Output: Send + Sync + Clone + Debug;

    fn parse(&self, text: &str) -> Result<Self::Output>;

    async fn aparse(&self, text: &str) -> Result<Self::Output> {
        self.parse(text)
    }

    fn parse_result(&self, result: &[Generation], _partial: bool) -> Result<Self::Output> {
        let first = result.first().ok_or_else(|| {
            Error::Other("parse_result called with empty result list".to_string())
        })?;
        self.parse(&first.text)
    }

    async fn aparse_result(&self, result: &[Generation], partial: bool) -> Result<Self::Output> {
        self.parse_result(result, partial)
    }

    fn parse_with_prompt(
        &self,
        completion: &str,
        _prompt: &dyn PromptValue,
    ) -> Result<Self::Output> {
        self.parse(completion)
    }

    fn get_format_instructions(&self) -> Result<String> {
        Err(Error::Other(
            "get_format_instructions not implemented".to_string(),
        ))
    }

    fn parser_type(&self) -> &str;

    fn invoke(&self, input: BaseMessage, _config: Option<RunnableConfig>) -> Result<Self::Output> {
        let chat_gen = ChatGeneration::builder().message(input).build();
        self.parse_result(&[Generation::builder().text(&chat_gen.text).build()], false)
    }

    async fn ainvoke(
        &self,
        input: BaseMessage,
        config: Option<RunnableConfig>,
    ) -> Result<Self::Output> {
        self.invoke(input, config)
    }

    fn into_runnable(self) -> RunnableOutputParser<Self>
    where
        Self: Sized,
    {
        RunnableOutputParser::new(self)
    }
}

#[derive(Debug, Clone)]
pub struct OutputParserError {
    pub message: String,
    pub llm_output: Option<String>,
    pub send_to_llm: bool,
    pub observation: Option<String>,
}

impl OutputParserError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            llm_output: None,
            send_to_llm: false,
            observation: None,
        }
    }

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

pub struct RunnableOutputParser<P> {
    parser: P,
}

impl<P: BaseOutputParser> RunnableOutputParser<P> {
    pub fn new(parser: P) -> Self {
        Self { parser }
    }

    pub fn parser(&self) -> &P {
        &self.parser
    }

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
        let generations = vec![Generation::builder().text("hello").build()];
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
