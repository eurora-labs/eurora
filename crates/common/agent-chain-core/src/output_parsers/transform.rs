use async_trait::async_trait;
use futures::StreamExt;
use futures::stream::BoxStream;

use crate::error::{Error, Result};
use crate::messages::BaseMessage;
use crate::outputs::{ChatGenerationChunk, Generation, GenerationChunk};
use crate::runnables::RunnableConfig;

use super::base::BaseOutputParser;

#[async_trait]
pub trait BaseTransformOutputParser: BaseOutputParser {
    fn parse_generation(&self, generation: &Generation) -> Result<Self::Output> {
        self.parse(&generation.text)
    }

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
                let chunk = ChatGenerationChunk::builder().message(message).build();
                let generation = Generation::builder().text(chunk.text.clone()).build();
                yield self.parse_result(&[generation], false);
            }
        })
    }

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

#[async_trait]
pub trait BaseCumulativeTransformOutputParser: BaseTransformOutputParser {
    fn diff_mode(&self) -> bool {
        false
    }

    fn compute_diff(
        &self,
        _prev: Option<&Self::Output>,
        _next: Self::Output,
    ) -> Result<Self::Output> {
        Err(Error::Other("_diff not implemented".to_string()))
    }

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
                let chunk_gen = GenerationChunk::builder().text(message.text()).build();

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
        let generation = Generation::builder().text("world").build();
        let result = parser.parse_generation(&generation).unwrap();
        assert_eq!(result, "WORLD");
    }
}
