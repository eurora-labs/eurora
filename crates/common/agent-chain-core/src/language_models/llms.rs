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
#[derive(Clone, Default)]
pub struct LLMConfig {
    /// Base language model configuration.
    pub base: LanguageModelConfig,

    /// Optional local cache instance for this LLM.
    /// When set, this cache is used instead of the global cache.
    pub cache_instance: Option<std::sync::Arc<dyn crate::caches::BaseCache>>,
}

impl std::fmt::Debug for LLMConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LLMConfig")
            .field("base", &self.base)
            .field(
                "cache_instance",
                &self.cache_instance.as_ref().map(|_| "..."),
            )
            .finish()
    }
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

    /// Set a local cache instance for this LLM.
    pub fn with_cache_instance(
        mut self,
        cache: std::sync::Arc<dyn crate::caches::BaseCache>,
    ) -> Self {
        self.cache_instance = Some(cache);
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

    /// Generate with cache support.
    ///
    /// This method mirrors Python's `BaseLLM.generate()`:
    /// 1. Resolve which cache to use (local instance, global, or none)
    /// 2. Look up cached results for each prompt
    /// 3. Call `generate_prompts` only for cache misses
    /// 4. Update the cache with new results
    /// 5. Return the combined result
    async fn generate(&self, prompts: Vec<String>, stop: Option<Vec<String>>) -> Result<LLMResult> {
        use crate::caches::BaseCache;

        // Resolve which cache to use
        let cache_config = self.llm_config().base.cache;
        let cache_instance = self.llm_config().cache_instance.clone();

        let resolved_cache: Option<std::sync::Arc<dyn BaseCache>> =
            if let Some(instance) = cache_instance {
                // Local cache instance takes priority
                Some(instance)
            } else if cache_config == Some(false) {
                // Explicitly disabled
                None
            } else {
                // Use global cache (may be None)
                crate::globals::get_llm_cache()
            };

        if let Some(cache) = &resolved_cache {
            // Cache is available — look up existing results
            let params = self.identifying_params();
            let (mut existing, llm_string, missing_idxs, missing_prompts) =
                get_prompts_from_cache(&params, &prompts, Some(cache.as_ref()));

            if missing_prompts.is_empty() {
                // All prompts were cached
                let generations = (0..prompts.len())
                    .map(|i| {
                        existing
                            .remove(&i)
                            .unwrap_or_default()
                            .into_iter()
                            .map(GenerationType::Generation)
                            .collect()
                    })
                    .collect();
                return Ok(LLMResult::new(generations));
            }

            // Generate only for misses
            let new_results = self.generate_prompts(missing_prompts, stop, None).await?;

            // Update cache
            update_cache(
                Some(cache.as_ref()),
                &mut existing,
                &llm_string,
                &missing_idxs,
                &new_results,
                &prompts,
            );

            // Reconstruct full result in order
            let generations = (0..prompts.len())
                .map(|i| {
                    existing
                        .remove(&i)
                        .unwrap_or_default()
                        .into_iter()
                        .map(GenerationType::Generation)
                        .collect()
                })
                .collect();
            Ok(LLMResult::new(generations))
        } else {
            // No cache — generate directly
            self.generate_prompts(prompts, stop, None).await
        }
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

    /// Process multiple inputs, returning individual results or errors.
    ///
    /// Unlike `batch`, this method catches per-item errors and returns them
    /// in-place rather than failing the entire batch.
    async fn batch_with_exceptions(&self, inputs: Vec<LanguageModelInput>) -> Vec<Result<String>> {
        let mut results = Vec::new();
        for input in inputs {
            results.push(self.invoke(input).await);
        }
        results
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
    // Use BTreeMap for deterministic key ordering in the cache key
    let sorted: std::collections::BTreeMap<_, _> = params.iter().collect();
    let llm_string = serde_json::to_string(&sorted).unwrap_or_default();
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

/// Cache resolution value. Can be a boolean flag or a cache instance.
impl std::fmt::Debug for CacheValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CacheValue::Flag(b) => write!(f, "CacheValue::Flag({})", b),
            CacheValue::Instance(_) => write!(f, "CacheValue::Instance(...)"),
        }
    }
}

#[derive(Clone)]
pub enum CacheValue {
    /// Use/don't use the global cache.
    Flag(bool),
    /// Use a specific cache instance.
    Instance(std::sync::Arc<dyn crate::caches::BaseCache>),
}

/// Resolve a cache value to an optional cache instance.
///
/// Mirrors Python's `_resolve_cache` function.
///
/// - `CacheValue::Instance(cache)` -> returns that cache
/// - `CacheValue::Flag(false)` -> returns None (no caching)
/// - `CacheValue::Flag(true)` -> returns the global cache, or errors if none set
/// - `None` -> returns the global cache if set, otherwise None
pub fn resolve_cache(
    cache: Option<CacheValue>,
) -> Result<Option<std::sync::Arc<dyn crate::caches::BaseCache>>> {
    match cache {
        Some(CacheValue::Instance(c)) => Ok(Some(c)),
        Some(CacheValue::Flag(false)) => Ok(None),
        Some(CacheValue::Flag(true)) => {
            let global = crate::globals::get_llm_cache();
            if global.is_some() {
                Ok(global)
            } else {
                Err(crate::error::Error::Other(
                    "No global cache was configured. Set the global cache via `set_llm_cache` or pass a cache instance directly.".to_string(),
                ))
            }
        }
        None => Ok(crate::globals::get_llm_cache()),
    }
}

/// Run ID input for batch operations.
///
/// Allows passing a single UUID, a list of UUIDs, or None.
#[derive(Debug, Clone)]
pub enum RunIdInput {
    /// No run IDs specified.
    None,
    /// A single UUID (used for the first prompt, rest are None).
    Single(uuid::Uuid),
    /// A list of UUIDs (must match batch length).
    List(Vec<uuid::Uuid>),
}

/// Normalize run_id input into a list of `Option<Uuid>` matching the prompts length.
///
/// Mirrors Python's `BaseLLM._get_run_ids_list`.
pub fn get_run_ids_list(run_id: RunIdInput, prompts_len: usize) -> Result<Vec<Option<uuid::Uuid>>> {
    match run_id {
        RunIdInput::None => Ok(vec![Option::None; prompts_len]),
        RunIdInput::Single(uid) => {
            let mut result = vec![Option::None; prompts_len];
            if !result.is_empty() {
                result[0] = Some(uid);
            }
            Ok(result)
        }
        RunIdInput::List(uids) => {
            if uids.len() != prompts_len {
                return Err(crate::error::Error::Other(format!(
                    "run_id list length ({}) does not match batch length ({})",
                    uids.len(),
                    prompts_len
                )));
            }
            Ok(uids.into_iter().map(Some).collect())
        }
    }
}

/// Create a retry wrapper that retries a function on specified errors.
///
/// Mirrors Python's `create_base_retry_decorator`.
///
/// The `error_predicate` function determines whether a given error should
/// trigger a retry. The function is called up to `max_retries` times total.
pub fn create_base_retry<F, T>(
    error_predicate: impl Fn(&crate::error::Error) -> bool,
    max_retries: usize,
    mut function: F,
) -> Result<T>
where
    F: FnMut() -> Result<T>,
{
    let mut last_error = None;
    for _ in 0..max_retries {
        match function() {
            Ok(value) => return Ok(value),
            Err(err) => {
                if error_predicate(&err) {
                    last_error = Some(err);
                    continue;
                }
                return Err(err);
            }
        }
    }
    Err(last_error
        .unwrap_or_else(|| crate::error::Error::Other("max retries exceeded".to_string())))
}

/// Save model parameters to a JSON file.
///
/// Writes the model's `identifying_params` to a file. Only `.json` extension
/// is supported (YAML would require an additional dependency).
///
/// Mirrors Python's `BaseLLM.save()`.
pub fn save_llm(
    identifying_params: &std::collections::HashMap<String, serde_json::Value>,
    path: &std::path::Path,
) -> Result<()> {
    let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");

    match extension {
        "json" => {
            let json = serde_json::to_string_pretty(identifying_params).map_err(|e| {
                crate::error::Error::Other(format!("JSON serialization failed: {}", e))
            })?;
            std::fs::write(path, json)
                .map_err(|e| crate::error::Error::Other(format!("Failed to write file: {}", e)))?;
            Ok(())
        }
        _ => Err(crate::error::Error::Other(format!(
            "File extension must be json, got: {}",
            extension
        ))),
    }
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
