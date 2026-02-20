use std::collections::HashMap;
use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;
use serde_json::Value;

use super::base::{BaseLanguageModel, LangSmithParams, LanguageModelConfig, LanguageModelInput};
use crate::callbacks::{AsyncCallbackManagerForLLMRun, CallbackManagerForLLMRun, Callbacks};
use crate::error::Result;
use crate::outputs::{
    ChatGeneration, ChatResult, Generation, GenerationChunk, GenerationType, LLMResult, RunInfo,
};
use crate::prompt_values::PromptValue;
use crate::runnables::RunnableConfig;

pub type LLMStream = Pin<Box<dyn Stream<Item = Result<GenerationChunk>> + Send>>;

#[derive(Clone, Default)]
pub struct LLMConfig {
    pub base: LanguageModelConfig,

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
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_cache(mut self, cache: bool) -> Self {
        self.base.cache = Some(cache);
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.base.tags = Some(tags);
        self
    }

    pub fn with_metadata(mut self, metadata: HashMap<String, Value>) -> Self {
        self.base.metadata = Some(metadata);
        self
    }

    pub fn with_cache_instance(
        mut self,
        cache: std::sync::Arc<dyn crate::caches::BaseCache>,
    ) -> Self {
        self.cache_instance = Some(cache);
        self
    }
}

#[derive(Debug, Clone, Default, bon::Builder)]
pub struct LLMGenerateConfig {
    #[builder(into)]
    pub stop: Option<Vec<String>>,
    pub callbacks: Option<Callbacks>,
    #[builder(into)]
    pub tags: Option<Vec<String>>,
    #[builder(into)]
    pub metadata: Option<HashMap<String, Value>>,
    #[builder(into)]
    pub run_name: Option<String>,
    pub run_id: Option<uuid::Uuid>,
}

impl LLMGenerateConfig {
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

fn extract_text(generation: &GenerationType) -> String {
    match generation {
        GenerationType::Generation(g) => g.text.clone(),
        GenerationType::GenerationChunk(g) => g.text.clone(),
        GenerationType::ChatGeneration(g) => g.text.to_string(),
        GenerationType::ChatGenerationChunk(g) => g.text.to_string(),
    }
}

#[async_trait]
pub trait BaseLLM: BaseLanguageModel {
    fn llm_config(&self) -> &LLMConfig;

    async fn generate_prompts(
        &self,
        prompts: Vec<String>,
        stop: Option<Vec<String>>,
        run_manager: Option<&CallbackManagerForLLMRun>,
    ) -> Result<LLMResult>;

    async fn stream_prompt(
        &self,
        prompt: String,
        stop: Option<Vec<String>>,
        run_manager: Option<&CallbackManagerForLLMRun>,
    ) -> Result<LLMStream> {
        let result = self
            .generate_prompts(vec![prompt], stop, run_manager)
            .await?;

        if let Some(generations) = result.generations.first()
            && let Some(generation) = generations.first()
        {
            let text = extract_text(generation);
            let chunk = GenerationChunk::new(text);
            return Ok(Box::pin(futures::stream::once(async move { Ok(chunk) })));
        }

        Ok(Box::pin(futures::stream::empty()))
    }

    fn convert_input(&self, input: LanguageModelInput) -> Result<String> {
        match input {
            LanguageModelInput::Text(s) => Ok(s),
            LanguageModelInput::StringPrompt(p) => Ok(p.to_string()),
            LanguageModelInput::ChatPrompt(p) => {
                let messages = p.to_messages();
                let parts: Vec<String> = messages
                    .iter()
                    .map(|msg| format!("{}: {}", msg.message_type(), msg.text()))
                    .collect();
                Ok(parts.join("\n"))
            }
            LanguageModelInput::ImagePrompt(p) => Ok(p.image_url.url.clone().unwrap_or_default()),
            LanguageModelInput::Messages(m) => {
                let parts: Vec<String> = m
                    .iter()
                    .map(|msg| format!("{}: {}", msg.message_type(), msg.text()))
                    .collect();
                Ok(parts.join("\n"))
            }
        }
    }

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

        if let Some(generations) = result.generations.first()
            && let Some(generation) = generations.first()
        {
            return Ok(extract_text(generation));
        }

        Ok(String::new())
    }

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

        let callback_manager = CallbackManager::configure(
            callbacks,
            self.callbacks().cloned(),
            self.verbose(),
            tags,
            self.config().tags.clone(),
            Some(inheritable_metadata),
            self.config().metadata.clone(),
        );

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
            let (mut existing, llm_string, missing_idxs, missing_prompts) =
                get_prompts_from_cache(&params, &prompts, Some(cache.as_ref()));

            if missing_prompts.is_empty() {
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

            let run_managers = callback_manager.on_llm_start(&params, &missing_prompts, run_id);

            let new_results = self
                ._generate_helper(missing_prompts, stop, &run_managers)
                .await?;

            update_cache(
                Some(cache.as_ref()),
                &mut existing,
                &llm_string,
                &missing_idxs,
                &new_results,
                &prompts,
            );

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
            let run_managers = callback_manager.on_llm_start(&params, &prompts, run_id);

            let mut output = self._generate_helper(prompts, stop, &run_managers).await?;

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

    async fn stream(
        &self,
        input: LanguageModelInput,
        config: Option<&RunnableConfig>,
        stop: Option<Vec<String>>,
    ) -> Result<LLMStream> {
        let prompt = self.convert_input(input)?;

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

        let callback_manager = crate::callbacks::CallbackManager::configure(
            callbacks,
            self.callbacks().cloned(),
            self.verbose(),
            tags,
            self.config().tags.clone(),
            Some(inheritable_metadata),
            self.config().metadata.clone(),
        );

        let run_managers =
            callback_manager.on_llm_start(&params, std::slice::from_ref(&prompt), run_id);
        let run_manager = run_managers.into_iter().next();

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

    async fn astream(
        &self,
        input: LanguageModelInput,
        config: Option<&RunnableConfig>,
        stop: Option<Vec<String>>,
    ) -> Result<LLMStream> {
        let prompt = self.convert_input(input)?;

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

        let callback_manager = crate::callbacks::AsyncCallbackManager::configure(
            callbacks,
            self.callbacks().cloned(),
            self.verbose(),
            tags,
            self.config().tags.clone(),
            Some(inheritable_metadata),
            self.config().metadata.clone(),
        );

        let run_managers = callback_manager
            .on_llm_start(&params, std::slice::from_ref(&prompt), run_id)
            .await;
        let run_manager = run_managers.into_iter().next();

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

    async fn ainvoke(
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

        let result = self.agenerate(vec![prompt], generate_config).await?;

        if let Some(generations) = result.generations.first()
            && let Some(generation) = generations.first()
        {
            return Ok(extract_text(generation));
        }

        Ok(String::new())
    }

    async fn agenerate(
        &self,
        prompts: Vec<String>,
        config: LLMGenerateConfig,
    ) -> Result<LLMResult> {
        use crate::caches::BaseCache;
        use crate::callbacks::AsyncCallbackManager;

        let LLMGenerateConfig {
            stop,
            callbacks,
            tags,
            metadata,
            run_name: _run_name,
            run_id,
        } = config;

        let params = self.identifying_params();

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

        let callback_manager = AsyncCallbackManager::configure(
            callbacks,
            self.callbacks().cloned(),
            self.verbose(),
            tags,
            self.config().tags.clone(),
            Some(inheritable_metadata),
            self.config().metadata.clone(),
        );

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
            let (mut existing, llm_string, missing_idxs, missing_prompts) =
                aget_prompts_from_cache(&params, &prompts, Some(cache.as_ref())).await;

            if missing_prompts.is_empty() {
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

            let run_managers = callback_manager
                .on_llm_start(&params, &missing_prompts, run_id)
                .await;

            let new_results = self
                ._agenerate_helper(missing_prompts, stop, &run_managers)
                .await?;

            aupdate_cache(
                Some(cache.as_ref()),
                &mut existing,
                &llm_string,
                &missing_idxs,
                &new_results,
                &prompts,
            )
            .await;

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
            let run_managers = callback_manager
                .on_llm_start(&params, &prompts, run_id)
                .await;

            let mut output = self._agenerate_helper(prompts, stop, &run_managers).await?;

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

    async fn _agenerate_helper(
        &self,
        prompts: Vec<String>,
        stop: Option<Vec<String>>,
        run_managers: &[AsyncCallbackManagerForLLMRun],
    ) -> Result<LLMResult> {
        match self
            .generate_prompts(
                prompts,
                stop,
                run_managers.first().map(|rm| rm.get_sync()).as_ref(),
            )
            .await
        {
            Ok(output) => {
                let flattened = output.flatten();
                for (run_manager, flattened_output) in run_managers.iter().zip(flattened.iter()) {
                    let chat_result = llm_result_to_chat_result(flattened_output);
                    run_manager.on_llm_end(&chat_result).await;
                }
                Ok(output)
            }
            Err(e) => {
                for run_manager in run_managers {
                    run_manager.get_sync().on_llm_error(&e);
                }
                Err(e)
            }
        }
    }

    async fn abatch(
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

        let result = self.agenerate(prompts, generate_config).await?;

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

    fn get_llm_ls_params(&self, stop: Option<&[String]>) -> LangSmithParams {
        let mut params = self.get_ls_params(stop);
        params.ls_model_type = Some("llm".to_string());
        params
    }

    fn save(&self, path: &std::path::Path) -> Result<()> {
        save_llm(&self.identifying_params(), path)
    }
}

#[async_trait]
pub trait LLM: BaseLLM {
    async fn call(
        &self,
        prompt: String,
        stop: Option<Vec<String>>,
        run_manager: Option<&CallbackManagerForLLMRun>,
    ) -> Result<String>;
}

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

pub async fn aget_prompts_from_cache(
    params: &HashMap<String, Value>,
    prompts: &[String],
    cache: Option<&dyn crate::caches::BaseCache>,
) -> (
    HashMap<usize, Vec<Generation>>,
    String,
    Vec<usize>,
    Vec<String>,
) {
    let sorted: std::collections::BTreeMap<_, _> = params.iter().collect();
    let llm_string = serde_json::to_string(&sorted).unwrap_or_default();
    let mut existing_prompts = HashMap::new();
    let mut missing_prompt_idxs = Vec::new();
    let mut missing_prompts = Vec::new();

    if let Some(cache) = cache {
        for (i, prompt) in prompts.iter().enumerate() {
            if let Some(cached) = cache.alookup(prompt, &llm_string).await {
                existing_prompts.insert(i, cached);
            } else {
                missing_prompts.push(prompt.clone());
                missing_prompt_idxs.push(i);
            }
        }
    } else {
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

pub async fn aupdate_cache(
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
                    cache.aupdate(prompt, llm_string, generations).await;
                }
            }
        }
    }

    new_results.llm_output.clone()
}

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
    Flag(bool),
    Instance(std::sync::Arc<dyn crate::caches::BaseCache>),
}

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

#[derive(Debug, Clone)]
pub enum RunIdInput {
    None,
    Single(uuid::Uuid),
    List(Vec<uuid::Uuid>),
}

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
