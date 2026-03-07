use std::fmt::Debug;

use crate::error::{Error, Result};
use crate::messages::AnyMessage;
use crate::outputs::{ChatGeneration, Generation};
use crate::prompt_values::PromptValue;
use crate::runnables::RunnableConfig;
use crate::runnables::base::Runnable;
use crate::runnables::config::run_in_executor;

#[derive(Debug, Clone)]
pub enum ParserInput {
    Text(String),
    Message(Box<AnyMessage>),
}

impl From<String> for ParserInput {
    fn from(s: String) -> Self {
        ParserInput::Text(s)
    }
}

impl From<&str> for ParserInput {
    fn from(s: &str) -> Self {
        ParserInput::Text(s.to_string())
    }
}

impl From<AnyMessage> for ParserInput {
    fn from(m: AnyMessage) -> Self {
        ParserInput::Message(Box::new(m))
    }
}

impl ParserInput {
    pub fn to_generation(&self) -> Generation {
        match self {
            ParserInput::Text(s) => Generation::builder().text(s).build(),
            ParserInput::Message(m) => {
                let chat_gen = ChatGeneration::builder()
                    .message(m.as_ref().clone())
                    .build();
                Generation::builder().text(&chat_gen.text).build()
            }
        }
    }
}

pub trait BaseLLMOutputParser: Send + Sync + Debug {
    type Output: Send + Sync + Clone + Debug;

    fn parse_result(&self, result: &[Generation], partial: bool) -> Result<Self::Output>;
}

pub trait BaseGenerationOutputParser: BaseLLMOutputParser {
    fn invoke(
        &self,
        input: impl Into<ParserInput>,
        _config: Option<RunnableConfig>,
    ) -> Result<Self::Output> {
        let generation = Into::<ParserInput>::into(input).to_generation();
        self.parse_result(&[generation], false)
    }
}

pub trait BaseOutputParser: Send + Sync + Debug {
    type Output: Send + Sync + Clone + Debug;

    fn parse(&self, text: &str) -> Result<Self::Output>;

    fn parse_result(&self, result: &[Generation], _partial: bool) -> Result<Self::Output> {
        let first = result
            .first()
            .ok_or_else(|| Error::output_parser_simple("parse_result called with empty list"))?;
        self.parse(&first.text)
    }

    fn parse_with_prompt(
        &self,
        completion: &str,
        _prompt: &dyn PromptValue,
    ) -> Result<Self::Output> {
        self.parse(completion)
    }

    fn get_format_instructions(&self) -> Result<String> {
        Err(Error::NotImplemented(
            "get_format_instructions not implemented".to_string(),
        ))
    }

    fn parser_type(&self) -> &str;

    fn invoke(
        &self,
        input: impl Into<ParserInput>,
        _config: Option<RunnableConfig>,
    ) -> Result<Self::Output> {
        let generation = Into::<ParserInput>::into(input).to_generation();
        self.parse_result(&[generation], false)
    }

    fn into_runnable(self) -> RunnableOutputParser<Self>
    where
        Self: Sized,
    {
        RunnableOutputParser::new(self)
    }
}

#[derive(Debug)]
pub struct RunnableOutputParser<P: Debug> {
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

#[async_trait::async_trait]
impl<P> Runnable for RunnableOutputParser<P>
where
    P: BaseOutputParser + Clone + 'static,
    P::Output: 'static,
{
    type Input = ParserInput;
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
        let parser = self.parser.clone();
        run_in_executor(move || parser.invoke(input, config)).await?
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
    fn test_invoke_with_string() {
        let parser = TestParser;
        let result = parser.invoke("hello", None).unwrap();
        assert_eq!(result, "HELLO");
    }

    #[test]
    fn test_invoke_with_message() {
        use crate::messages::HumanMessage;
        let parser = TestParser;
        let msg = AnyMessage::Human(HumanMessage::builder().content("hello").build());
        let result = parser.invoke(msg, None).unwrap();
        assert_eq!(result, "HELLO");
    }

    #[test]
    fn test_parser_input_from_str() {
        let input: ParserInput = "hello".into();
        let generation = input.to_generation();
        assert_eq!(generation.text, "hello");
    }

    #[test]
    fn test_parser_input_from_message() {
        use crate::messages::HumanMessage;
        let msg = AnyMessage::Human(HumanMessage::builder().content("hello").build());
        let input: ParserInput = msg.into();
        let generation = input.to_generation();
        assert_eq!(generation.text, "hello");
    }

    #[test]
    fn test_output_parser_error() {
        let err = Error::output_parser_with_output("Invalid JSON", "{invalid}");
        match err {
            Error::OutputParser {
                ref message,
                ref llm_output,
                ..
            } => {
                assert_eq!(message, "Invalid JSON");
                assert_eq!(llm_output.as_deref(), Some("{invalid}"));
            }
            _ => panic!("Expected OutputParser variant"),
        }
    }
}
