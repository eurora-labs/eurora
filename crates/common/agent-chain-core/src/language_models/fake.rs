use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use serde_json::Value;

use super::base::{BaseLanguageModel, LanguageModelConfig, LanguageModelInput};
use super::llms::{BaseLLM, LLM, LLMConfig, LLMStream};
use crate::caches::BaseCache;
use crate::callbacks::{CallbackManagerForLLMRun, Callbacks};
use crate::error::Result;
use crate::outputs::{Generation, GenerationChunk, GenerationType, LLMResult};

#[derive(Debug)]
pub struct FakeListLLM {
    responses: Vec<String>,
    sleep: Option<Duration>,
    index: AtomicUsize,
    config: LLMConfig,
}

impl Clone for FakeListLLM {
    fn clone(&self) -> Self {
        Self {
            responses: self.responses.clone(),
            sleep: self.sleep,
            index: AtomicUsize::new(self.index.load(Ordering::SeqCst)),
            config: self.config.clone(),
        }
    }
}

#[bon::bon]
impl FakeListLLM {
    #[builder]
    pub fn new(
        responses: Vec<String>,
        sleep: Option<Duration>,
        config: Option<LLMConfig>,
        cache_instance: Option<std::sync::Arc<dyn crate::caches::BaseCache>>,
        cache: Option<bool>,
    ) -> Self {
        let mut config = config.unwrap_or_default();
        if let Some(instance) = cache_instance {
            config.cache_instance = Some(instance);
        }
        if let Some(cache) = cache {
            config.base.cache = Some(cache);
        }
        Self {
            responses,
            sleep,
            index: AtomicUsize::new(0),
            config,
        }
    }

    pub fn current_index(&self) -> usize {
        self.index.load(Ordering::SeqCst)
    }

    pub fn reset(&self) {
        self.index.store(0, Ordering::SeqCst);
    }

    fn get_next_response(&self) -> String {
        let i = self.index.load(Ordering::SeqCst);
        let response = self.responses.get(i).cloned().unwrap_or_default();

        let next_i = if i + 1 < self.responses.len() {
            i + 1
        } else {
            0
        };
        self.index.store(next_i, Ordering::SeqCst);

        response
    }
}

#[async_trait]
impl BaseLanguageModel for FakeListLLM {
    fn llm_type(&self) -> &str {
        "fake-list"
    }

    fn model_name(&self) -> &str {
        "fake-list-llm"
    }

    fn config(&self) -> &LanguageModelConfig {
        &self.config.base
    }

    fn cache(&self) -> Option<&dyn BaseCache> {
        None
    }

    fn callbacks(&self) -> Option<&Callbacks> {
        None
    }

    async fn generate_prompt(
        &self,
        prompts: Vec<LanguageModelInput>,
        stop: Option<Vec<String>>,
        _callbacks: Option<Callbacks>,
    ) -> Result<LLMResult> {
        let prompt_strings: Vec<String> = prompts.iter().map(|p| p.to_string()).collect();
        self.generate_prompts(prompt_strings, stop, None).await
    }

    fn identifying_params(&self) -> HashMap<String, Value> {
        let mut params = HashMap::new();
        params.insert(
            "_type".to_string(),
            Value::String(self.llm_type().to_string()),
        );
        params.insert(
            "responses".to_string(),
            serde_json::to_value(&self.responses).unwrap_or_default(),
        );
        params
    }
}

#[async_trait]
impl BaseLLM for FakeListLLM {
    fn llm_config(&self) -> &LLMConfig {
        &self.config
    }

    async fn generate_prompts(
        &self,
        prompts: Vec<String>,
        _stop: Option<Vec<String>>,
        _run_manager: Option<&CallbackManagerForLLMRun>,
    ) -> Result<LLMResult> {
        let mut generations = Vec::new();

        for _ in prompts {
            let response = self.get_next_response();
            let generation = Generation::builder().text(response).build();
            generations.push(vec![GenerationType::Generation(generation)]);
        }

        Ok(LLMResult::builder().generations(generations).build())
    }
}

#[async_trait]
impl LLM for FakeListLLM {
    async fn call(
        &self,
        _prompt: String,
        _stop: Option<Vec<String>>,
        _run_manager: Option<&CallbackManagerForLLMRun>,
    ) -> Result<String> {
        Ok(self.get_next_response())
    }
}

#[derive(Debug, Clone, thiserror::Error)]
#[error("FakeListLLM error")]
pub struct FakeListLLMError;

#[derive(Debug)]
pub struct FakeStreamingListLLM {
    inner: FakeListLLM,
    error_on_chunk_number: Option<usize>,
}

#[bon::bon]
impl FakeStreamingListLLM {
    #[builder]
    pub fn new(
        responses: Vec<String>,
        sleep: Option<Duration>,
        config: Option<LLMConfig>,
        error_on_chunk: Option<usize>,
    ) -> Self {
        Self {
            inner: FakeListLLM::builder()
                .responses(responses)
                .maybe_sleep(sleep)
                .maybe_config(config)
                .build(),
            error_on_chunk_number: error_on_chunk,
        }
    }

    pub fn current_index(&self) -> usize {
        self.inner.current_index()
    }

    pub fn reset(&self) {
        self.inner.reset();
    }
}

impl Clone for FakeStreamingListLLM {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            error_on_chunk_number: self.error_on_chunk_number,
        }
    }
}

#[async_trait]
impl BaseLanguageModel for FakeStreamingListLLM {
    fn llm_type(&self) -> &str {
        "fake-list"
    }

    fn model_name(&self) -> &str {
        "fake-list-llm"
    }

    fn config(&self) -> &LanguageModelConfig {
        self.inner.config()
    }

    fn cache(&self) -> Option<&dyn BaseCache> {
        None
    }

    fn callbacks(&self) -> Option<&Callbacks> {
        None
    }

    async fn generate_prompt(
        &self,
        prompts: Vec<LanguageModelInput>,
        stop: Option<Vec<String>>,
        callbacks: Option<Callbacks>,
    ) -> Result<LLMResult> {
        self.inner.generate_prompt(prompts, stop, callbacks).await
    }

    fn identifying_params(&self) -> HashMap<String, Value> {
        self.inner.identifying_params()
    }
}

#[async_trait]
impl BaseLLM for FakeStreamingListLLM {
    fn llm_config(&self) -> &LLMConfig {
        self.inner.llm_config()
    }

    async fn generate_prompts(
        &self,
        prompts: Vec<String>,
        stop: Option<Vec<String>>,
        run_manager: Option<&CallbackManagerForLLMRun>,
    ) -> Result<LLMResult> {
        self.inner
            .generate_prompts(prompts, stop, run_manager)
            .await
    }

    async fn stream_prompt(
        &self,
        prompt: String,
        _stop: Option<Vec<String>>,
        _run_manager: Option<&CallbackManagerForLLMRun>,
    ) -> Result<LLMStream> {
        let response = self.inner.call(prompt, None, None).await?;
        let sleep = self.inner.sleep;
        let error_on_chunk = self.error_on_chunk_number;

        let stream = async_stream::stream! {
            for (i, c) in response.chars().enumerate() {
                if let Some(duration) = sleep {
                    tokio::time::sleep(duration).await;
                }

                if let Some(error_chunk) = error_on_chunk
                    && i == error_chunk
                {
                    yield Err(crate::error::Error::Other(
                        "FakeListLLM error".to_string()
                    ));
                    return;
                }

                yield Ok(GenerationChunk::builder().text(c.to_string()).build());
            }
        };

        Ok(Box::pin(stream))
    }
}

#[async_trait]
impl LLM for FakeStreamingListLLM {
    async fn call(
        &self,
        prompt: String,
        stop: Option<Vec<String>>,
        run_manager: Option<&CallbackManagerForLLMRun>,
    ) -> Result<String> {
        self.inner.call(prompt, stop, run_manager).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fake_list_llm_responses() {
        let llm = FakeListLLM::builder()
            .responses(vec![
                "Response 1".to_string(),
                "Response 2".to_string(),
                "Response 3".to_string(),
            ])
            .build();

        let result = llm.call("prompt".to_string(), None, None).await.unwrap();
        assert_eq!(result, "Response 1");

        let result = llm.call("prompt".to_string(), None, None).await.unwrap();
        assert_eq!(result, "Response 2");

        let result = llm.call("prompt".to_string(), None, None).await.unwrap();
        assert_eq!(result, "Response 3");

        let result = llm.call("prompt".to_string(), None, None).await.unwrap();
        assert_eq!(result, "Response 1");
    }

    #[tokio::test]
    async fn test_fake_list_llm_reset() {
        let llm = FakeListLLM::builder()
            .responses(vec!["Response 1".to_string(), "Response 2".to_string()])
            .build();

        let _ = llm.call("prompt".to_string(), None, None).await;
        assert_eq!(llm.current_index(), 1);

        llm.reset();
        assert_eq!(llm.current_index(), 0);

        let result = llm.call("prompt".to_string(), None, None).await.unwrap();
        assert_eq!(result, "Response 1");
    }

    #[tokio::test]
    async fn test_fake_list_llm_generate_prompts() {
        let llm = FakeListLLM::builder()
            .responses(vec!["Response 1".to_string(), "Response 2".to_string()])
            .build();

        let result = llm
            .generate_prompts(
                vec!["prompt1".to_string(), "prompt2".to_string()],
                None,
                None,
            )
            .await
            .unwrap();

        assert_eq!(result.generations.len(), 2);
    }

    #[tokio::test]
    async fn test_fake_streaming_list_llm() {
        use futures::StreamExt;

        let llm = FakeStreamingListLLM::builder()
            .responses(vec!["Hello".to_string()])
            .build();

        let mut stream = llm
            .stream_prompt("prompt".to_string(), None, None)
            .await
            .unwrap();

        let mut result = String::new();
        while let Some(chunk) = stream.next().await {
            result.push_str(&chunk.unwrap().text);
        }

        assert_eq!(result, "Hello");
    }

    #[test]
    fn test_fake_list_llm_identifying_params() {
        let llm = FakeListLLM::builder()
            .responses(vec!["Response".to_string()])
            .build();
        let params = llm.identifying_params();

        assert!(params.contains_key("responses"));
        assert!(params.contains_key("_type"));
        assert_eq!(params["_type"], Value::String("fake-list".to_string()));
    }
}
