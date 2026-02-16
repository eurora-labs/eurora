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
use crate::callbacks::{CallbackManagerForLLMRun, Callbacks};
use crate::error::Result;
use crate::outputs::{
    ChatGeneration, ChatResult, Generation, GenerationChunk, GenerationType, LLMResult, RunInfo,
};
use crate::prompt_values::PromptValue;
use crate::runnables::RunnableConfig;

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

/// Configuration for `BaseLLM::generate()` calls.
///
/// Mirrors `GenerateConfig` from `chat_models.rs`.
/// Allows passing callbacks, tags, metadata, and run information
/// into the LLM generation pipeline.
#[derive(Debug, Clone, Default, bon::Builder)]
pub struct LLMGenerateConfig {
    /// Stop words to use when generating.
    #[builder(into)]
    pub stop: Option<Vec<String>>,
    /// Callbacks to pass through.
    pub callbacks: Option<Callbacks>,
    /// Tags to apply to the run.
    #[builder(into)]
    pub tags: Option<Vec<String>>,
    /// Metadata to apply to the run.
    #[builder(into)]
    pub metadata: Option<HashMap<String, Value>>,
    /// Name for the run (used in tracing).
    #[builder(into)]
    pub run_name: Option<String>,
    /// ID for the run (used in tracing).
    pub run_id: Option<uuid::Uuid>,
}

impl LLMGenerateConfig {
    /// Create an LLMGenerateConfig from a RunnableConfig.
    pub fn from_runnable_config(config: &RunnableConfig) -> Self {
        Self {
            stop: None,
            callbacks: config.callbacks.clone(),
            tags: Some(config.tags.clone()).filter(|t| !t.is_empty()),
            metadata: Some(config.metadata.clone()).filter(|m| !m.is_empty()),
            run_name: config.run_name.clone(),
            run_id: config.run_id,
        }
    }
}

/// Convert an `LLMResult` to a `ChatResult` for `on_llm_end` callbacks.
///
/// The callback system's `on_llm_end` takes `&ChatResult`, but LLMs produce
/// `LLMResult` with `Generation`s. This converts each generation's text into
/// a dummy `ChatGeneration` wrapping an `AIMessage`.
fn llm_result_to_chat_result(result: &LLMResult) -> ChatResult {
    let generations: Vec<ChatGeneration> = result
        .generations
        .iter()
        .flatten()
        .map(|g| {
            let text = extract_text(g);
            let msg = crate::messages::AIMessage::builder().content(&text).build();
            ChatGeneration::new(msg.into())
        })
        .collect();
    ChatResult::new(generations)
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
                    .map(|msg| format!("{}: {}", msg.message_type(), msg.text()))
                    .collect();
                Ok(parts.join("\n"))
            }
            LanguageModelInput::ImagePrompt(p) => Ok(p.image_url.url.clone().unwrap_or_default()),
            LanguageModelInput::Messages(m) => {
                // Convert messages to a string representation
                let parts: Vec<String> = m
                    .iter()
                    .map(|msg| format!("{}: {}", msg.message_type(), msg.text()))
                    .collect();
                Ok(parts.join("\n"))
            }
        }
    }

    /// Invoke the model with input.
    ///
    /// Routes through `generate()` to ensure the full callback pipeline
    /// (on_llm_start, on_llm_end, on_llm_error) is triggered.
    async fn invoke(
        &self,
        input: LanguageModelInput,
        config: Option<&RunnableConfig>,
    ) -> Result<String> {
        let prompt = self.convert_input(input)?;

        let generate_config = if let Some(cfg) = config {
            LLMGenerateConfig::from_runnable_config(cfg)
        } else {
            LLMGenerateConfig::default()
        };

        let result = self.generate(vec![prompt], generate_config).await?;

        // Get the first generation's text
        if let Some(generations) = result.generations.first()
            && let Some(generation) = generations.first()
        {
            return Ok(extract_text(generation));
        }

        Ok(String::new())
    }

    /// Generate with cache and callback support.
    ///
    /// This method mirrors Python's `BaseLLM.generate()`:
    /// 1. Configure callback manager from config
    /// 2. Resolve which cache to use (local instance, global, or none)
    /// 3. Look up cached results for each prompt
    /// 4. Fire `on_llm_start` for non-cached prompts
    /// 5. Call `_generate_helper` only for cache misses
    /// 6. Fire `on_llm_end`/`on_llm_error` via helper
    /// 7. Attach `RunInfo` to the result
    async fn generate(&self, prompts: Vec<String>, config: LLMGenerateConfig) -> Result<LLMResult> {
        use crate::caches::BaseCache;
        use crate::callbacks::CallbackManager;

        let LLMGenerateConfig {
            stop,
            callbacks,
            tags,
            metadata,
            run_name: _run_name,
            run_id,
        } = config;

        let params = self.identifying_params();

        // Build inheritable metadata with LangSmith params
        let mut inheritable_metadata = metadata.clone().unwrap_or_default();
        let ls_params = self.get_llm_ls_params(stop.as_deref());
        if let Some(provider) = ls_params.ls_provider {
            inheritable_metadata.insert("ls_provider".to_string(), Value::String(provider));
        }
        if let Some(model_name) = ls_params.ls_model_name {
            inheritable_metadata.insert("ls_model_name".to_string(), Value::String(model_name));
        }
        if let Some(model_type) = ls_params.ls_model_type {
            inheritable_metadata.insert("ls_model_type".to_string(), Value::String(model_type));
        }

        // Configure callback manager
        let callback_manager = CallbackManager::configure(
            callbacks,
            self.callbacks().cloned(),
            false,
            tags,
            self.config().tags.clone(),
            Some(inheritable_metadata),
            self.config().metadata.clone(),
        );

        // Resolve which cache to use
        let cache_config = self.llm_config().base.cache;
        let cache_instance = self.llm_config().cache_instance.clone();

        let resolved_cache: Option<std::sync::Arc<dyn BaseCache>> =
            if let Some(instance) = cache_instance {
                Some(instance)
            } else if cache_config == Some(false) {
                None
            } else {
                crate::globals::get_llm_cache()
            };

        if let Some(cache) = &resolved_cache {
            // Cache is available — look up existing results
            let (mut existing, llm_string, missing_idxs, missing_prompts) =
                get_prompts_from_cache(&params, &prompts, Some(cache.as_ref()));

            if missing_prompts.is_empty() {
                // All prompts were cached — no callbacks needed
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

            // Fire on_llm_start only for missing prompts
            let run_managers = callback_manager.on_llm_start(&params, &missing_prompts, run_id);

            // Generate only for misses
            let new_results = self
                ._generate_helper(missing_prompts, stop, &run_managers)
                .await?;

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

            let mut output = LLMResult::new(generations);

            // Attach run info
            if !run_managers.is_empty() {
                output.run = Some(
                    run_managers
                        .iter()
                        .map(|rm| RunInfo::new(rm.run_id()))
                        .collect(),
                );
            }

            Ok(output)
        } else {
            // No cache — fire on_llm_start for all prompts
            let run_managers = callback_manager.on_llm_start(&params, &prompts, run_id);

            let mut output = self._generate_helper(prompts, stop, &run_managers).await?;

            // Attach run info
            if !run_managers.is_empty() {
                output.run = Some(
                    run_managers
                        .iter()
                        .map(|rm| RunInfo::new(rm.run_id()))
                        .collect(),
                );
            }

            Ok(output)
        }
    }

    /// Helper that calls `generate_prompts` and fires `on_llm_end`/`on_llm_error`.
    ///
    /// Mirrors Python's `BaseLLM._generate_helper`.
    async fn _generate_helper(
        &self,
        prompts: Vec<String>,
        stop: Option<Vec<String>>,
        run_managers: &[CallbackManagerForLLMRun],
    ) -> Result<LLMResult> {
        match self
            .generate_prompts(prompts, stop, run_managers.first())
            .await
        {
            Ok(output) => {
                // Fire on_llm_end for each run manager with flattened output
                let flattened = output.flatten();
                for (run_manager, flattened_output) in run_managers.iter().zip(flattened.iter()) {
                    let chat_result = llm_result_to_chat_result(flattened_output);
                    run_manager.on_llm_end(&chat_result);
                }
                Ok(output)
            }
            Err(e) => {
                for run_manager in run_managers {
                    run_manager.on_llm_error(&e);
                }
                Err(e)
            }
        }
    }

    /// Process multiple inputs and return results.
    ///
    /// Routes through `generate()` to ensure the full callback pipeline
    /// (on_llm_start, on_llm_end, on_llm_error) is triggered.
    async fn batch(
        &self,
        inputs: Vec<LanguageModelInput>,
        config: Option<&RunnableConfig>,
    ) -> Result<Vec<String>> {
        if inputs.is_empty() {
            return Ok(Vec::new());
        }

        let prompts: Vec<String> = inputs
            .into_iter()
            .map(|i| self.convert_input(i))
            .collect::<Result<Vec<_>>>()?;

        let generate_config = if let Some(cfg) = config {
            LLMGenerateConfig::from_runnable_config(cfg)
        } else {
            LLMGenerateConfig::default()
        };

        let result = self.generate(prompts, generate_config).await?;

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
    async fn batch_with_exceptions(
        &self,
        inputs: Vec<LanguageModelInput>,
        config: Option<&RunnableConfig>,
    ) -> Vec<Result<String>> {
        let mut results = Vec::new();
        for input in inputs {
            results.push(self.invoke(input, config).await);
        }
        results
    }

    /// Stream the model output with full callback pipeline.
    ///
    /// Sets up `CallbackManager::configure()`, fires `on_llm_start` before
    /// streaming, `on_llm_new_token` for each chunk, `on_llm_end` at
    /// completion, and `on_llm_error` on failure.
    ///
    /// Mirrors Python's `BaseLLM.stream()`.
    async fn stream(
        &self,
        input: LanguageModelInput,
        config: Option<&RunnableConfig>,
        stop: Option<Vec<String>>,
    ) -> Result<LLMStream> {
        let prompt = self.convert_input(input)?;

        // Extract config fields
        let (callbacks, tags, metadata, _run_name, run_id) = if let Some(cfg) = config {
            (
                cfg.callbacks.clone(),
                Some(cfg.tags.clone()).filter(|t| !t.is_empty()),
                Some(cfg.metadata.clone()).filter(|m| !m.is_empty()),
                cfg.run_name.clone(),
                cfg.run_id,
            )
        } else {
            (None, None, None, None, None)
        };

        let params = self.identifying_params();

        // Build inheritable metadata with LangSmith params
        let mut inheritable_metadata = metadata.unwrap_or_default();
        let ls_params = self.get_llm_ls_params(stop.as_deref());
        if let Some(provider) = ls_params.ls_provider {
            inheritable_metadata.insert("ls_provider".to_string(), Value::String(provider));
        }
        if let Some(model_name) = ls_params.ls_model_name {
            inheritable_metadata.insert("ls_model_name".to_string(), Value::String(model_name));
        }
        if let Some(model_type) = ls_params.ls_model_type {
            inheritable_metadata.insert("ls_model_type".to_string(), Value::String(model_type));
        }

        // Configure callback manager
        let callback_manager = crate::callbacks::CallbackManager::configure(
            callbacks,
            self.callbacks().cloned(),
            false,
            tags,
            self.config().tags.clone(),
            Some(inheritable_metadata),
            self.config().metadata.clone(),
        );

        // Fire on_llm_start
        let run_managers =
            callback_manager.on_llm_start(&params, std::slice::from_ref(&prompt), run_id);
        let run_manager = run_managers.into_iter().next();

        // Get the inner stream
        let generation_stream = self
            .stream_prompt(prompt, stop, run_manager.as_ref())
            .await?;

        let chunk_stream = async_stream::stream! {
            use futures::StreamExt;

            let mut pinned_stream = generation_stream;
            let mut chunks: Vec<GenerationChunk> = Vec::new();

            while let Some(result) = pinned_stream.next().await {
                match result {
                    Ok(chunk) => {
                        // Fire on_llm_new_token callback
                        if let Some(ref rm) = run_manager {
                            rm.on_llm_new_token(&chunk.text, None);
                        }
                        chunks.push(chunk.clone());
                        yield Ok(chunk);
                    }
                    Err(e) => {
                        if let Some(ref rm) = run_manager {
                            rm.on_llm_error(&e);
                        }
                        yield Err(e);
                        return;
                    }
                }
            }

            // Fire on_llm_end with merged generation
            if let Some(ref rm) = run_manager
                && let Some(merged) = crate::outputs::merge_generation_chunks(chunks) {
                    let generation: Generation = merged.into();
                    let result = LLMResult::new(vec![vec![GenerationType::Generation(generation)]]);
                    let chat_result = llm_result_to_chat_result(&result);
                    rm.on_llm_end(&chat_result);
                }
        };

        Ok(Box::pin(chunk_stream))
    }

    /// Async stream the model output with full callback pipeline.
    ///
    /// Uses `AsyncCallbackManager::configure()` and async callback methods.
    ///
    /// Mirrors Python's `BaseLLM.astream()`.
    async fn astream(
        &self,
        input: LanguageModelInput,
        config: Option<&RunnableConfig>,
        stop: Option<Vec<String>>,
    ) -> Result<LLMStream> {
        let prompt = self.convert_input(input)?;

        // Extract config fields
        let (callbacks, tags, metadata, _run_name, run_id) = if let Some(cfg) = config {
            (
                cfg.callbacks.clone(),
                Some(cfg.tags.clone()).filter(|t| !t.is_empty()),
                Some(cfg.metadata.clone()).filter(|m| !m.is_empty()),
                cfg.run_name.clone(),
                cfg.run_id,
            )
        } else {
            (None, None, None, None, None)
        };

        let params = self.identifying_params();

        // Build inheritable metadata with LangSmith params
        let mut inheritable_metadata = metadata.unwrap_or_default();
        let ls_params = self.get_llm_ls_params(stop.as_deref());
        if let Some(provider) = ls_params.ls_provider {
            inheritable_metadata.insert("ls_provider".to_string(), Value::String(provider));
        }
        if let Some(model_name) = ls_params.ls_model_name {
            inheritable_metadata.insert("ls_model_name".to_string(), Value::String(model_name));
        }
        if let Some(model_type) = ls_params.ls_model_type {
            inheritable_metadata.insert("ls_model_type".to_string(), Value::String(model_type));
        }

        // Configure async callback manager
        let callback_manager = crate::callbacks::AsyncCallbackManager::configure(
            callbacks,
            self.callbacks().cloned(),
            false,
            tags,
            self.config().tags.clone(),
            Some(inheritable_metadata),
            self.config().metadata.clone(),
        );

        // Fire on_llm_start
        let run_managers = callback_manager
            .on_llm_start(&params, std::slice::from_ref(&prompt), run_id)
            .await;
        let run_manager = run_managers.into_iter().next();

        // Get the inner stream
        let generation_stream = self
            .stream_prompt(
                prompt,
                stop,
                run_manager.as_ref().map(|rm| rm.get_sync()).as_ref(),
            )
            .await?;

        let chunk_stream = async_stream::stream! {
            use futures::StreamExt;

            let mut pinned_stream = generation_stream;
            let mut chunks: Vec<GenerationChunk> = Vec::new();

            while let Some(result) = pinned_stream.next().await {
                match result {
                    Ok(chunk) => {
                        // Fire on_llm_new_token callback
                        if let Some(ref rm) = run_manager {
                            rm.on_llm_new_token(&chunk.text, None).await;
                        }
                        chunks.push(chunk.clone());
                        yield Ok(chunk);
                    }
                    Err(e) => {
                        if let Some(ref rm) = run_manager {
                            rm.get_sync().on_llm_error(&e);
                        }
                        yield Err(e);
                        return;
                    }
                }
            }

            // Fire on_llm_end with merged generation
            if let Some(ref rm) = run_manager
                && let Some(merged) = crate::outputs::merge_generation_chunks(chunks) {
                    let generation: Generation = merged.into();
                    let result = LLMResult::new(vec![vec![GenerationType::Generation(generation)]]);
                    let chat_result = llm_result_to_chat_result(&result);
                    rm.on_llm_end(&chat_result).await;
                }
        };

        Ok(Box::pin(chunk_stream))
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
