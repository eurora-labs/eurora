//! Base classes for output parsers that can handle streaming input.
//!
//! This module contains `BaseTransformOutputParser` and
//! `BaseCumulativeTransformOutputParser` which provide streaming support.
//! Mirrors `langchain_core.output_parsers.transform`.

use std::fmt::Debug;

use async_trait::async_trait;
use futures::StreamExt;
use futures::stream::BoxStream;

use crate::error::Result;
use crate::messages::BaseMessage;
use crate::outputs::{ChatGenerationChunk, Generation, GenerationChunk};
use crate::runnables::RunnableConfig;

use super::base::BaseOutputParser;

/// Base trait for an output parser that can handle streaming input.
///
/// Transform output parsers can process input streams chunk by chunk,
/// which is useful for streaming responses from LLMs.
#[async_trait]
pub trait BaseTransformOutputParser: BaseOutputParser {
    /// Parse a generation into the output type.
    fn parse_generation(&self, generation: &Generation) -> Result<Self::Output> {
        self.parse(&generation.text)
    }

    /// Transform an input stream into an output stream.
    ///
    /// Default implementation yields a parsed result for each chunk.
    fn transform<'a>(
        &'a self,
        input: BoxStream<'a, StringOrMessage>,
    ) -> BoxStream<'a, Result<Self::Output>>
    where
        Self::Output: 'a,
    {
        Box::pin(async_stream::stream! {
            let mut stream = input;
            while let Some(chunk) = stream.next().await {
                let generation = match chunk {
                    StringOrMessage::Text(text) => Generation::new(text),
                    StringOrMessage::Message(msg) => Generation::new((*msg).content()),
                };
                yield self.parse_result(&[generation], false);
            }
        })
    }

    /// Async transform an input stream into an output stream.
    fn atransform<'a>(
        &'a self,
        input: BoxStream<'a, StringOrMessage>,
    ) -> BoxStream<'a, Result<Self::Output>>
    where
        Self::Output: 'a,
    {
        self.transform(input)
    }
}

/// Base trait for an output parser that accumulates chunks before parsing.
///
/// This is useful for parsers that need to see the complete output before
/// parsing, but want to yield intermediate results during streaming.
/// For example, a JSON parser might yield partial JSON objects as they're built up.
#[async_trait]
pub trait BaseCumulativeTransformOutputParser: BaseOutputParser {
    /// Whether to yield diffs between the previous and current parsed output,
    /// or just the current parsed output.
    fn diff_mode(&self) -> bool {
        false
    }

    /// Convert parsed outputs into a diff format.
    ///
    /// The semantics of this are up to the output parser.
    /// Default implementation returns the next value unchanged.
    fn compute_diff(&self, _prev: Option<&Self::Output>, next: Self::Output) -> Self::Output {
        next
    }

    /// Transform an input stream into an output stream, accumulating chunks.
    ///
    /// This accumulates input chunks and parses the accumulated result,
    /// yielding intermediate results as they change.
    fn transform<'a>(
        &'a self,
        input: BoxStream<'a, StringOrMessage>,
        _config: Option<RunnableConfig>,
    ) -> BoxStream<'a, Result<Self::Output>>
    where
        Self::Output: PartialEq + 'a,
    {
        let diff_mode = self.diff_mode();

        Box::pin(async_stream::stream! {
            let mut prev_parsed: Option<Self::Output> = None;
            let mut acc_gen: Option<AccumulatedGeneration> = None;
            let mut stream = input;

            while let Some(chunk) = stream.next().await {
                let chunk_gen = match chunk {
                    StringOrMessage::Text(text) => AccumulatedGeneration::Text(text),
                    StringOrMessage::Message(msg) => {
                        AccumulatedGeneration::Text((*msg).content().to_string())
                    }
                };

                acc_gen = Some(match acc_gen {
                    None => chunk_gen,
                    Some(acc) => acc.add(chunk_gen),
                });

                if let Some(ref acc) = acc_gen {
                    let generation = acc.to_generation();
                    if let Ok(parsed) = self.parse_result(&[generation], true) {
                        let should_yield = match &prev_parsed {
                            Some(prev) => parsed != *prev,
                            None => true,
                        };

                        if should_yield {
                            if diff_mode {
                                yield Ok(self.compute_diff(prev_parsed.as_ref(), parsed.clone()));
                            } else {
                                yield Ok(parsed.clone());
                            }
                            prev_parsed = Some(parsed);
                        }
                    }
                }
            }
        })
    }

    /// Async transform an input stream into an output stream.
    fn atransform<'a>(
        &'a self,
        input: BoxStream<'a, StringOrMessage>,
        config: Option<RunnableConfig>,
    ) -> BoxStream<'a, Result<Self::Output>>
    where
        Self::Output: PartialEq + 'a,
    {
        self.transform(input, config)
    }
}

/// Input type that can be either a string or a message.
#[derive(Debug, Clone)]
pub enum StringOrMessage {
    /// Raw text input.
    Text(String),
    /// Message input.
    Message(Box<BaseMessage>),
}

impl From<String> for StringOrMessage {
    fn from(text: String) -> Self {
        StringOrMessage::Text(text)
    }
}

impl From<&str> for StringOrMessage {
    fn from(text: &str) -> Self {
        StringOrMessage::Text(text.to_string())
    }
}

impl From<BaseMessage> for StringOrMessage {
    fn from(msg: BaseMessage) -> Self {
        StringOrMessage::Message(Box::new(msg))
    }
}

/// Accumulated generation state for streaming.
#[derive(Debug, Clone)]
#[allow(dead_code)]
enum AccumulatedGeneration {
    /// Accumulated text.
    Text(String),
    /// Accumulated generation chunk.
    GenerationChunk(GenerationChunk),
    /// Accumulated chat generation chunk.
    ChatGenerationChunk(Box<ChatGenerationChunk>),
}

impl AccumulatedGeneration {
    /// Add another chunk to this accumulation.
    fn add(self, other: AccumulatedGeneration) -> Self {
        match (self, other) {
            (AccumulatedGeneration::Text(mut left), AccumulatedGeneration::Text(right)) => {
                left.push_str(&right);
                AccumulatedGeneration::Text(left)
            }
            (
                AccumulatedGeneration::GenerationChunk(left),
                AccumulatedGeneration::GenerationChunk(right),
            ) => AccumulatedGeneration::GenerationChunk(left + right),
            (
                AccumulatedGeneration::ChatGenerationChunk(left),
                AccumulatedGeneration::ChatGenerationChunk(right),
            ) => AccumulatedGeneration::ChatGenerationChunk(Box::new(*left + *right)),
            (AccumulatedGeneration::Text(text), AccumulatedGeneration::GenerationChunk(chunk)) => {
                let combined = GenerationChunk::new(text) + chunk;
                AccumulatedGeneration::GenerationChunk(combined)
            }
            (AccumulatedGeneration::GenerationChunk(chunk), AccumulatedGeneration::Text(text)) => {
                let combined = chunk + GenerationChunk::new(text);
                AccumulatedGeneration::GenerationChunk(combined)
            }
            (left, right) => {
                let left_gen = left.to_generation();
                let right_gen = right.to_generation();
                let combined_text = format!("{}{}", left_gen.text, right_gen.text);
                AccumulatedGeneration::Text(combined_text)
            }
        }
    }

    /// Convert to a Generation for parsing.
    fn to_generation(&self) -> Generation {
        match self {
            AccumulatedGeneration::Text(text) => Generation::new(text.clone()),
            AccumulatedGeneration::GenerationChunk(chunk) => Generation::from(chunk.clone()),
            AccumulatedGeneration::ChatGenerationChunk(chunk) => {
                Generation::new(chunk.as_ref().text.clone())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone)]
    struct TestTransformParser;

    impl BaseOutputParser for TestTransformParser {
        type Output = String;

        fn parse(&self, text: &str) -> Result<String> {
            Ok(text.to_uppercase())
        }

        fn parser_type(&self) -> &str {
            "test_transform"
        }
    }

    impl BaseTransformOutputParser for TestTransformParser {}

    #[test]
    fn test_transform_parser_parse() {
        let parser = TestTransformParser;
        let result = parser.parse("hello").unwrap();
        assert_eq!(result, "HELLO");
    }

    #[test]
    fn test_transform_parser_parse_generation() {
        let parser = TestTransformParser;
        let generation = Generation::new("world");
        let result = parser.parse_generation(&generation).unwrap();
        assert_eq!(result, "WORLD");
    }

    #[test]
    fn test_string_or_message_from_string() {
        let input: StringOrMessage = "test".into();
        match input {
            StringOrMessage::Text(t) => assert_eq!(t, "test"),
            _ => panic!("Expected Text variant"),
        }
    }

    #[test]
    fn test_accumulated_generation_add_text() {
        let left = AccumulatedGeneration::Text("Hello ".to_string());
        let right = AccumulatedGeneration::Text("World".to_string());
        let result = left.add(right);

        if let AccumulatedGeneration::Text(text) = result {
            assert_eq!(text, "Hello World");
        } else {
            panic!("Expected Text variant");
        }
    }

    #[test]
    fn test_accumulated_generation_to_generation() {
        let acc = AccumulatedGeneration::Text("test".to_string());
        let generation = acc.to_generation();
        assert_eq!(generation.text, "test");
    }
}
