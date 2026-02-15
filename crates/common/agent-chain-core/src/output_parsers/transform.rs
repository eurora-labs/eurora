//! Base classes for output parsers that can handle streaming input.
//!
//! Mirrors `langchain_core.output_parsers.transform`.

use async_trait::async_trait;
use futures::StreamExt;
use futures::stream::BoxStream;

use crate::error::{Error, Result};
use crate::messages::BaseMessage;
use crate::outputs::{ChatGenerationChunk, Generation, GenerationChunk};
use crate::runnables::RunnableConfig;

use super::base::BaseOutputParser;

/// Base trait for an output parser that can handle streaming input.
#[async_trait]
pub trait BaseTransformOutputParser: BaseOutputParser {
    /// Parse a generation into the output type.
    fn parse_generation(&self, generation: &Generation) -> Result<Self::Output> {
        self.parse(&generation.text)
    }

    /// Transform an input stream into an output stream.
    fn transform<'a>(
        &'a self,
        input: BoxStream<'a, BaseMessage>,
    ) -> BoxStream<'a, Result<Self::Output>>
    where
        Self::Output: 'a,
    {
        Box::pin(async_stream::stream! {
            let mut stream = input;
            while let Some(message) = stream.next().await {
                let chunk = ChatGenerationChunk::new(message);
                let generation = Generation::new(chunk.text.clone());
                yield self.parse_result(&[generation], false);
            }
        })
    }

    /// Async transform an input stream into an output stream.
    fn atransform<'a>(
        &'a self,
        input: BoxStream<'a, BaseMessage>,
    ) -> BoxStream<'a, Result<Self::Output>>
    where
        Self::Output: 'a,
    {
        self.transform(input)
    }
}

/// Base trait for an output parser that accumulates chunks before parsing.
///
/// Extends `BaseTransformOutputParser` - in diff mode, yields diffs between
/// the previous and current parsed output.
#[async_trait]
pub trait BaseCumulativeTransformOutputParser: BaseTransformOutputParser {
    /// Whether to yield diffs between the previous and current parsed output,
    /// or just the current parsed output.
    fn diff_mode(&self) -> bool {
        false
    }

    /// Convert parsed outputs into a diff format.
    ///
    /// Must be implemented by subclasses when `diff_mode` is true.
    /// Default implementation returns an error (matching Python's `raise NotImplementedError`).
    fn compute_diff(
        &self,
        _prev: Option<&Self::Output>,
        _next: Self::Output,
    ) -> Result<Self::Output> {
        Err(Error::Other("_diff not implemented".to_string()))
    }

    /// Transform an input stream into an output stream, accumulating chunks.
    fn cumulative_transform<'a>(
        &'a self,
        input: BoxStream<'a, BaseMessage>,
        _config: Option<RunnableConfig>,
    ) -> BoxStream<'a, Result<Self::Output>>
    where
        Self::Output: PartialEq + 'a,
    {
        let diff_mode = self.diff_mode();

        Box::pin(async_stream::stream! {
            let mut prev_parsed: Option<Self::Output> = None;
            let mut acc_gen: Option<GenerationChunk> = None;
            let mut stream = input;

            while let Some(message) = stream.next().await {
                let chunk_gen = GenerationChunk::new(message.content());

                acc_gen = Some(match acc_gen {
                    None => chunk_gen,
                    Some(acc) => acc + chunk_gen,
                });

                if let Some(ref acc) = acc_gen {
                    let generation = Generation::from(acc.clone());
                    if let Ok(parsed) = self.parse_result(&[generation], true) {
                        let should_yield = match &prev_parsed {
                            Some(prev) => parsed != *prev,
                            None => true,
                        };

                        if should_yield {
                            if diff_mode {
                                yield self.compute_diff(prev_parsed.as_ref(), parsed.clone());
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
}
