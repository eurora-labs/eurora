//! Base interface for traditional large language models (LLMs).
//!
//! These are traditionally older models (newer models generally are chat models).
//! LLMs take a string as input and return a string as output.
//! Mirrors `langchain_core.language_models.llms`.

use std::collections::HashMap;
use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;
use serde_json::Value;

use super::base::{BaseLanguageModel, LangSmithParams, LanguageModelConfig, LanguageModelInput};
use crate::callbacks::CallbackManagerForLLMRun;
use crate::error::Result;

use crate::outputs::{Generation, GenerationChunk, GenerationType, LLMResult};
use crate::prompt_values::PromptValue;

/// Type alias for a streaming LLM output.
pub type LLMStream = Pin<Box<dyn Stream<Item = Result<GenerationChunk>> + Send>>;

/// Configuration specific to LLMs.
#[derive(Debug, Clone, Default)]
pub struct LLMConfig {
    /// Base language model configuration.
    pub base: LanguageModelConfig,
}

impl LLMConfig {
    /// Create a new LLM configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable caching.
    pub fn with_cache(mut self, cache: bool) -> Self {
        self.base.cache = Some(cache);
        self
    }

    /// Set tags.
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.base.tags = Some(tags);
        self
    }

    /// Set metadata.
    pub fn with_metadata(mut self, metadata: HashMap<String, Value>) -> Self {
        self.base.metadata = Some(metadata);
        self
    }
}

/// Helper function to extract text from a GenerationType.
fn extract_text(generation: &GenerationType) -> String {
    match generation {
        GenerationType::Generation(g) => g.text.clone(),
        GenerationType::GenerationChunk(g) => g.text.clone(),
        GenerationType::ChatGeneration(g) => g.text.to_string(),
        GenerationType::ChatGenerationChunk(g) => g.text.to_string(),
    }
}

/// Base LLM abstract interface.
///
/// It should take in a prompt and return a string.
///
/// # Implementation Guide
///
/// | Method/Property         | Description                                        | Required |
/// |------------------------|----------------------------------------------------|---------:|
/// | `generate_prompts`     | Use to generate from prompts                       | Required |
/// | `llm_type` (property)  | Used to uniquely identify the type of the model    | Required |
/// | `stream_prompt`        | Use to implement streaming                         | Optional |
#[async_trait]
pub trait BaseLLM: BaseLanguageModel {
    /// Get the LLM configuration.
    fn llm_config(&self) -> &LLMConfig;

    /// Run the LLM on the given prompts.
    ///
    /// # Arguments
    ///
    /// * `prompts` - The prompts to generate from.
    /// * `stop` - Stop words to use when generating.
    /// * `run_manager` - Callback manager for the run.
    ///
    /// # Returns
    ///
    /// The LLM result.
    async fn generate_prompts(
        &self,
        prompts: Vec<String>,
        stop: Option<Vec<String>>,
        run_manager: Option<&CallbackManagerForLLMRun>,
    ) -> Result<LLMResult>;

    /// Stream the LLM on the given prompt.
    ///
    /// Default implementation falls back to `generate_prompts` and returns
    /// the output as a single chunk.
    ///
    /// # Arguments
    ///
    /// * `prompt` - The prompt to generate from.
    /// * `stop` - Stop words to use when generating.
    /// * `run_manager` - Callback manager for the run.
    ///
    /// # Returns
    ///
    /// A stream of generation chunks.
    async fn stream_prompt(
        &self,
        prompt: String,
        stop: Option<Vec<String>>,
        run_manager: Option<&CallbackManagerForLLMRun>,
    ) -> Result<LLMStream> {
        let result = self
            .generate_prompts(vec![prompt], stop, run_manager)
            .await?;

        // Get the first generation
        if let Some(generations) = result.generations.first()
            && let Some(generation) = generations.first()
        {
            let text = extract_text(generation);
            let chunk = GenerationChunk::new(text);
            return Ok(Box::pin(futures::stream::once(async move { Ok(chunk) })));
        }

        // Empty result
        Ok(Box::pin(futures::stream::empty()))
    }

    /// Convert input to a prompt string.
    fn convert_input(&self, input: LanguageModelInput) -> Result<String> {
        match input {
            LanguageModelInput::Text(s) => Ok(s),
            LanguageModelInput::StringPrompt(p) => Ok(p.to_string()),
            LanguageModelInput::ChatPrompt(p) => {
                // Convert chat prompt to string representation
                let messages = p.to_messages();
                let parts: Vec<String> = messages
                    .iter()
                    .map(|msg| format!("{}: {}", msg.message_type(), msg.content()))
                    .collect();
                Ok(parts.join("\n"))
            }
            LanguageModelInput::ImagePrompt(p) => Ok(p.image_url.url.clone().unwrap_or_default()),
            LanguageModelInput::Messages(m) => {
                // Convert messages to a string representation
                let parts: Vec<String> = m
                    .iter()
                    .map(|msg| format!("{}: {}", msg.message_type(), msg.content()))
                    .collect();
                Ok(parts.join("\n"))
            }
        }
    }

    /// Invoke the model with input.
    async fn invoke(&self, input: LanguageModelInput) -> Result<String> {
        let prompt = self.convert_input(input)?;
        let result = self.generate_prompts(vec![prompt], None, None).await?;

        // Get the first generation's text
        if let Some(generations) = result.generations.first()
            && let Some(generation) = generations.first()
        {
            return Ok(extract_text(generation));
        }

        Ok(String::new())
    }

    /// Process multiple inputs and return results.
    ///
    /// Calls `generate_prompts` with all inputs at once and extracts text
    /// from each generation.
    async fn batch(&self, inputs: Vec<LanguageModelInput>) -> Result<Vec<String>> {
        if inputs.is_empty() {
            return Ok(Vec::new());
        }

        let prompts: Vec<String> = inputs.iter().map(|i| i.to_string()).collect();
        let result = self.generate_prompts(prompts, None, None).await?;

        let mut outputs = Vec::new();
        for generations in &result.generations {
            if let Some(generation) = generations.first() {
                outputs.push(extract_text(generation));
            } else {
                outputs.push(String::new());
            }
        }
        Ok(outputs)
    }

    /// Get standard params for tracing.
    fn get_llm_ls_params(&self, stop: Option<&[String]>) -> LangSmithParams {
        let mut params = self.get_ls_params(stop);
        params.ls_model_type = Some("llm".to_string());
        params
    }
}

/// Simple interface for implementing a custom LLM.
///
/// You should subclass this class and implement the following:
///
/// - `call` method: Run the LLM on the given prompt.
/// - `llm_type` property: Return a unique identifier for this LLM.
/// - `identifying_params` property: Return identifying parameters for caching/tracing.
///
/// Optional: Override the following methods for more optimizations:
///
/// - `acall`: Provide a native async version of `call`.
/// - `stream_prompt`: Stream the LLM output.
/// - `astream_prompt`: Async version of streaming.
#[async_trait]
pub trait LLM: BaseLLM {
    /// Run the LLM on the given input.
    ///
    /// # Arguments
    ///
    /// * `prompt` - The prompt to generate from.
    /// * `stop` - Stop words to use when generating.
    /// * `run_manager` - Callback manager for the run.
    ///
    /// # Returns
    ///
    /// The model output as a string. Should NOT include the prompt.
    async fn call(
        &self,
        prompt: String,
        stop: Option<Vec<String>>,
        run_manager: Option<&CallbackManagerForLLMRun>,
    ) -> Result<String>;
}

/// Helper function to get prompts from cache.
///
/// Returns existing prompts, llm string, missing prompt indices, and missing prompts.
pub fn get_prompts_from_cache(
    params: &HashMap<String, Value>,
    prompts: &[String],
    cache: Option<&dyn crate::caches::BaseCache>,
) -> (
    HashMap<usize, Vec<Generation>>,
    String,
    Vec<usize>,
    Vec<String>,
) {
    let llm_string = serde_json::to_string(&params).unwrap_or_default();
    let mut existing_prompts = HashMap::new();
    let mut missing_prompt_idxs = Vec::new();
    let mut missing_prompts = Vec::new();

    if let Some(cache) = cache {
        for (i, prompt) in prompts.iter().enumerate() {
            if let Some(cached) = cache.lookup(prompt, &llm_string) {
                existing_prompts.insert(i, cached);
            } else {
                missing_prompts.push(prompt.clone());
                missing_prompt_idxs.push(i);
            }
        }
    } else {
        // No cache, all prompts are missing
        for (i, prompt) in prompts.iter().enumerate() {
            missing_prompts.push(prompt.clone());
            missing_prompt_idxs.push(i);
        }
    }

    (
        existing_prompts,
        llm_string,
        missing_prompt_idxs,
        missing_prompts,
    )
}

/// Helper function to update cache with new results.
pub fn update_cache(
    cache: Option<&dyn crate::caches::BaseCache>,
    existing_prompts: &mut HashMap<usize, Vec<Generation>>,
    llm_string: &str,
    missing_prompt_idxs: &[usize],
    new_results: &LLMResult,
    prompts: &[String],
) -> Option<HashMap<String, Value>> {
    if let Some(cache) = cache {
        for (i, result) in new_results.generations.iter().enumerate() {
            if let Some(&idx) = missing_prompt_idxs.get(i) {
                let generations: Vec<Generation> = result
                    .iter()
                    .filter_map(|g| match g {
                        GenerationType::Generation(generation) => Some(generation.clone()),
                        GenerationType::GenerationChunk(chunk) => Some(chunk.clone().into()),
                        _ => None,
                    })
                    .collect();

                existing_prompts.insert(idx, generations.clone());

                if let Some(prompt) = prompts.get(idx) {
                    cache.update(prompt, llm_string, generations);
                }
            }
        }
    }

    new_results.llm_output.clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_llm_config_builder() {
        let config = LLMConfig::new()
            .with_cache(true)
            .with_tags(vec!["test".to_string()]);

        assert_eq!(config.base.cache, Some(true));
        assert_eq!(config.base.tags, Some(vec!["test".to_string()]));
    }

    #[test]
    fn test_get_prompts_from_cache_no_cache() {
        let params = HashMap::new();
        let prompts = vec!["Hello".to_string(), "World".to_string()];

        let (existing, _llm_string, missing_idxs, missing) =
            get_prompts_from_cache(&params, &prompts, None);

        assert!(existing.is_empty());
        assert_eq!(missing_idxs, vec![0, 1]);
        assert_eq!(missing, prompts);
    }
}
