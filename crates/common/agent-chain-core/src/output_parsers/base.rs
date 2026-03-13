use std::fmt::Debug;

use crate::error::{Error, Result};
use crate::messages::AnyMessage;
use crate::outputs::ChatGeneration;
use crate::runnables::RunnableConfig;
use crate::runnables::base::Runnable;
use crate::runnables::config::run_in_executor;

pub trait BaseLLMOutputParser: Send + Sync + Debug {
    type Output: Send + Sync + Clone + Debug;

    fn parse_result(&self, result: &[ChatGeneration], partial: bool) -> Result<Self::Output>;
}

pub trait BaseGenerationOutputParser: BaseLLMOutputParser {}

pub trait BaseOutputParser: Send + Sync + Debug {
    type Output: Send + Sync + Clone + Debug;

    fn parse(&self, text: &str) -> Result<Self::Output>;

    fn parse_result(&self, result: &[ChatGeneration], _partial: bool) -> Result<Self::Output> {
        let first = result
            .first()
            .ok_or_else(|| Error::output_parser_simple("parse_result called with empty list"))?;
        self.parse(&first.message.text())
    }

    fn parse_with_prompt(&self, completion: &str, _prompt: &[AnyMessage]) -> Result<Self::Output> {
        self.parse(completion)
    }

    fn get_format_instructions(&self) -> Result<String> {
        Err(Error::NotImplemented(
            "get_format_instructions not implemented".to_string(),
        ))
    }

    fn parser_type(&self) -> &str;
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
    type Input = AnyMessage;
    type Output = P::Output;

    fn name(&self) -> Option<String> {
        Some(format!(
            "RunnableOutputParser<{}>",
            self.parser.parser_type()
        ))
    }
    async fn invoke(
        &self,
        input: Self::Input,
        _config: Option<RunnableConfig>,
    ) -> Result<Self::Output>
    where
        Self: 'static,
    {
        let parser = self.parser.clone();
        run_in_executor(move || {
            let generation = ChatGeneration::builder().message(input).build();
            parser.parse_result(&[generation], false)
        })
        .await?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone)]
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
        use crate::messages::AIMessage;
        let parser = TestParser;
        let msg = AIMessage::builder().content("hello").build();
        let generations = vec![ChatGeneration::builder().message(msg.into()).build()];
        let result = parser.parse_result(&generations, false).unwrap();
        assert_eq!(result, "HELLO");
    }

    #[test]
    fn test_parse_result_empty() {
        let parser = TestParser;
        let result = parser.parse_result(&[], false);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_invoke_with_string() {
        let parser = TestParser;
        let runnable = parser.into_runnable();
        use crate::messages::HumanMessage;
        let msg = AnyMessage::HumanMessage(HumanMessage::builder().content("hello").build());
        let result = runnable.invoke(msg, None).await.unwrap();
        assert_eq!(result, "HELLO");
    }

    #[tokio::test]
    async fn test_invoke_with_message() {
        use crate::messages::HumanMessage;
        let parser = TestParser.into_runnable();
        let msg = AnyMessage::HumanMessage(HumanMessage::builder().content("hello").build());
        let result = parser.invoke(msg, None).await.unwrap();
        assert_eq!(result, "HELLO");
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
