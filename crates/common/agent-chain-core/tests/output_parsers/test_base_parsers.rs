//! Tests for BaseGenerationOutputParser and BaseTransformOutputParser.
//!
//! Ported from langchain/libs/core/tests/unit_tests/output_parsers/test_base_parsers.py

use agent_chain_core::GenericFakeChatModel;
use agent_chain_core::error::Result;
use agent_chain_core::language_models::BaseChatModel;
use agent_chain_core::messages::{AIMessage, BaseMessage, HumanMessage};
use agent_chain_core::output_parsers::{
    BaseGenerationOutputParser, BaseLLMOutputParser, BaseOutputParser, BaseTransformOutputParser,
};
use agent_chain_core::outputs::Generation;
use futures::StreamExt;

/// Inverts the case of each character in a string, equivalent to Python's str.swapcase().
fn swap_case(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_uppercase() {
                c.to_lowercase().collect::<String>()
            } else if c.is_lowercase() {
                c.to_uppercase().collect::<String>()
            } else {
                c.to_string()
            }
        })
        .collect()
}

/// A parser that inverts the case of the characters in the message.
/// Implements BaseGenerationOutputParser (via BaseLLMOutputParser + BaseGenerationOutputParser).
#[derive(Debug)]
struct GenerationStrInvertCase;

impl BaseLLMOutputParser for GenerationStrInvertCase {
    type Output = String;

    fn parse_result(&self, result: &[Generation], _partial: bool) -> Result<String> {
        if result.len() != 1 {
            return Err(agent_chain_core::error::Error::NotImplemented(
                "This output parser can only be used with a single generation.".to_string(),
            ));
        }
        Ok(swap_case(&result[0].text))
    }
}

impl BaseGenerationOutputParser for GenerationStrInvertCase {}

#[tokio::test]
async fn test_base_generation_parser() {
    let model = GenericFakeChatModel::from_vec(vec![AIMessage::builder().content("hEllo").build()]);

    let model_output = model
        ._generate(
            vec![BaseMessage::Human(
                HumanMessage::builder().content("").build(),
            )],
            None,
            None,
        )
        .await
        .unwrap();

    let parser = GenerationStrInvertCase;
    let result = parser
        .invoke(model_output.generations[0].message.clone(), None)
        .unwrap();

    assert_eq!(result, "HeLLO");
}

/// A parser that inverts the case of the characters in the message.
/// Implements BaseTransformOutputParser (via BaseOutputParser + BaseTransformOutputParser).
#[derive(Debug)]
struct TransformStrInvertCase;

impl BaseOutputParser for TransformStrInvertCase {
    type Output = String;

    fn parse(&self, _text: &str) -> Result<String> {
        Err(agent_chain_core::error::Error::NotImplemented(
            "parse not implemented".to_string(),
        ))
    }

    fn parse_result(&self, result: &[Generation], _partial: bool) -> Result<String> {
        if result.len() != 1 {
            return Err(agent_chain_core::error::Error::NotImplemented(
                "This output parser can only be used with a single generation.".to_string(),
            ));
        }
        Ok(swap_case(&result[0].text))
    }

    fn parser_type(&self) -> &str {
        "str_invert_case"
    }
}

impl BaseTransformOutputParser for TransformStrInvertCase {}

#[tokio::test]
async fn test_base_transform_output_parser() {
    let model =
        GenericFakeChatModel::from_vec(vec![AIMessage::builder().content("hello world").build()]);

    let stream = model
        ._stream(
            vec![BaseMessage::Human(
                HumanMessage::builder().content("").build(),
            )],
            None,
            None,
        )
        .unwrap();

    let input_stream = stream.filter_map(|chunk| async { chunk.ok().map(|c| c.message) });

    let parser = TransformStrInvertCase;
    let chunks: Vec<String> = parser
        .transform(Box::pin(input_stream))
        .filter_map(|r| async { r.ok() })
        .collect()
        .await;

    assert_eq!(chunks, vec!["HELLO", " ", "WORLD"]);
}
