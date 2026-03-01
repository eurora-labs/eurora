//! Ollama large language model implementation.
//!
//! Matches Python `langchain_ollama/llms.py`.

use std::collections::HashMap;
use std::env;

use async_trait::async_trait;
use futures::StreamExt;
use serde::Deserialize;
use tokio::io::AsyncBufReadExt;
use tokio_util::io::StreamReader;

use super::chat_models::{KeepAlive, OllamaFormat};
use super::utils::{merge_auth_headers, parse_url_with_auth, validate_model};
use crate::callbacks::{CallbackManagerForLLMRun, Callbacks};
use crate::error::{Error, Result};
use crate::language_models::{BaseLLM, LLMConfig, LLMStream};
use crate::language_models::{
    BaseLanguageModel, LangSmithParams, LanguageModelConfig, LanguageModelInput,
};
use crate::outputs::{GenerationChunk, GenerationType, LLMResult};

const DEFAULT_API_BASE: &str = "http://localhost:11434";

/// Response from the Ollama `/api/generate` endpoint.
#[derive(Debug, Deserialize)]
struct OllamaGenerateResponse {
    model: Option<String>,
    response: Option<String>,
    thinking: Option<String>,
    done: Option<bool>,
    done_reason: Option<String>,
    prompt_eval_count: Option<u32>,
    eval_count: Option<u32>,
    total_duration: Option<u64>,
    load_duration: Option<u64>,
    prompt_eval_duration: Option<u64>,
    eval_duration: Option<u64>,
    created_at: Option<String>,
}

/// Ollama large language model.
///
/// Matches Python's `OllamaLLM` class.
#[derive(Debug)]
pub struct OllamaLLM {
    model: String,
    reasoning: Option<bool>,
    validate_model_on_init: bool,
    mirostat: Option<i32>,
    mirostat_eta: Option<f64>,
    mirostat_tau: Option<f64>,
    num_ctx: Option<u32>,
    num_gpu: Option<i32>,
    num_thread: Option<i32>,
    num_predict: Option<i32>,
    repeat_last_n: Option<i32>,
    repeat_penalty: Option<f64>,
    temperature: Option<f64>,
    seed: Option<i64>,
    stop: Option<Vec<String>>,
    tfs_z: Option<f64>,
    top_k: Option<i32>,
    top_p: Option<f64>,
    format: Option<OllamaFormat>,
    keep_alive: Option<KeepAlive>,
    base_url: Option<String>,
    client_kwargs: HashMap<String, serde_json::Value>,
    async_client_kwargs: HashMap<String, serde_json::Value>,
    sync_client_kwargs: HashMap<String, serde_json::Value>,
    llm_config: LLMConfig,
    language_model_config: LanguageModelConfig,
    model_validated: std::sync::atomic::AtomicBool,
}

impl Clone for OllamaLLM {
    fn clone(&self) -> Self {
        Self {
            model: self.model.clone(),
            reasoning: self.reasoning,
            validate_model_on_init: self.validate_model_on_init,
            mirostat: self.mirostat,
            mirostat_eta: self.mirostat_eta,
            mirostat_tau: self.mirostat_tau,
            num_ctx: self.num_ctx,
            num_gpu: self.num_gpu,
            num_thread: self.num_thread,
            num_predict: self.num_predict,
            repeat_last_n: self.repeat_last_n,
            repeat_penalty: self.repeat_penalty,
            temperature: self.temperature,
            seed: self.seed,
            stop: self.stop.clone(),
            tfs_z: self.tfs_z,
            top_k: self.top_k,
            top_p: self.top_p,
            format: self.format.clone(),
            keep_alive: self.keep_alive.clone(),
            base_url: self.base_url.clone(),
            client_kwargs: self.client_kwargs.clone(),
            async_client_kwargs: self.async_client_kwargs.clone(),
            sync_client_kwargs: self.sync_client_kwargs.clone(),
            llm_config: self.llm_config.clone(),
            language_model_config: self.language_model_config.clone(),
            model_validated: std::sync::atomic::AtomicBool::new(
                self.model_validated
                    .load(std::sync::atomic::Ordering::Relaxed),
            ),
        }
    }
}

impl OllamaLLM {
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            reasoning: None,
            validate_model_on_init: false,
            mirostat: None,
            mirostat_eta: None,
            mirostat_tau: None,
            num_ctx: None,
            num_gpu: None,
            num_thread: None,
            num_predict: None,
            repeat_last_n: None,
            repeat_penalty: None,
            temperature: None,
            seed: None,
            stop: None,
            tfs_z: None,
            top_k: None,
            top_p: None,
            format: None,
            keep_alive: None,
            base_url: None,
            client_kwargs: HashMap::new(),
            async_client_kwargs: HashMap::new(),
            sync_client_kwargs: HashMap::new(),
            llm_config: LLMConfig::default(),
            language_model_config: LanguageModelConfig::new(),
            model_validated: std::sync::atomic::AtomicBool::new(false),
        }
    }

    pub fn reasoning(mut self, enabled: bool) -> Self {
        self.reasoning = Some(enabled);
        self
    }

    pub fn validate_model_on_init(mut self, validate: bool) -> Self {
        self.validate_model_on_init = validate;
        self
    }

    pub fn temperature(mut self, temp: f64) -> Self {
        self.temperature = Some(temp);
        self
    }

    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = Some(url.into());
        self
    }

    pub fn mirostat(mut self, mode: i32) -> Self {
        self.mirostat = Some(mode);
        self
    }

    pub fn mirostat_eta(mut self, eta: f64) -> Self {
        self.mirostat_eta = Some(eta);
        self
    }

    pub fn mirostat_tau(mut self, tau: f64) -> Self {
        self.mirostat_tau = Some(tau);
        self
    }

    pub fn num_ctx(mut self, ctx: u32) -> Self {
        self.num_ctx = Some(ctx);
        self
    }

    pub fn num_gpu(mut self, gpu: i32) -> Self {
        self.num_gpu = Some(gpu);
        self
    }

    pub fn num_thread(mut self, thread: i32) -> Self {
        self.num_thread = Some(thread);
        self
    }

    pub fn num_predict(mut self, predict: i32) -> Self {
        self.num_predict = Some(predict);
        self
    }

    pub fn repeat_last_n(mut self, n: i32) -> Self {
        self.repeat_last_n = Some(n);
        self
    }

    pub fn repeat_penalty(mut self, penalty: f64) -> Self {
        self.repeat_penalty = Some(penalty);
        self
    }

    pub fn seed(mut self, seed: i64) -> Self {
        self.seed = Some(seed);
        self
    }

    pub fn stop(mut self, sequences: Vec<String>) -> Self {
        self.stop = Some(sequences);
        self
    }

    pub fn tfs_z(mut self, z: f64) -> Self {
        self.tfs_z = Some(z);
        self
    }

    pub fn top_k(mut self, k: i32) -> Self {
        self.top_k = Some(k);
        self
    }

    pub fn top_p(mut self, p: f64) -> Self {
        self.top_p = Some(p);
        self
    }

    pub fn format(mut self, format: OllamaFormat) -> Self {
        self.format = Some(format);
        self
    }

    pub fn keep_alive(mut self, duration: impl Into<String>) -> Self {
        self.keep_alive = Some(KeepAlive::Duration(duration.into()));
        self
    }

    pub fn keep_alive_seconds(mut self, seconds: i64) -> Self {
        self.keep_alive = Some(KeepAlive::Seconds(seconds));
        self
    }

    pub fn client_kwargs(mut self, kwargs: HashMap<String, serde_json::Value>) -> Self {
        self.client_kwargs = kwargs;
        self
    }

    pub fn async_client_kwargs(mut self, kwargs: HashMap<String, serde_json::Value>) -> Self {
        self.async_client_kwargs = kwargs;
        self
    }

    pub fn sync_client_kwargs(mut self, kwargs: HashMap<String, serde_json::Value>) -> Self {
        self.sync_client_kwargs = kwargs;
        self
    }

    /// Build the options dict for the request.
    /// Matches Python's options construction in `_generate_params`.
    pub fn build_options(&self, stop: Option<Vec<String>>) -> Result<serde_json::Value> {
        if self.stop.is_some() && stop.is_some() {
            return Err(Error::Other(
                "`stop` found in both the input and default params.".into(),
            ));
        }

        let mut options = serde_json::Map::new();

        if let Some(mirostat) = self.mirostat {
            options.insert("mirostat".to_string(), serde_json::json!(mirostat));
        }
        if let Some(eta) = self.mirostat_eta {
            options.insert("mirostat_eta".to_string(), serde_json::json!(eta));
        }
        if let Some(tau) = self.mirostat_tau {
            options.insert("mirostat_tau".to_string(), serde_json::json!(tau));
        }
        if let Some(ctx) = self.num_ctx {
            options.insert("num_ctx".to_string(), serde_json::json!(ctx));
        }
        if let Some(gpu) = self.num_gpu {
            options.insert("num_gpu".to_string(), serde_json::json!(gpu));
        }
        if let Some(thread) = self.num_thread {
            options.insert("num_thread".to_string(), serde_json::json!(thread));
        }
        if let Some(predict) = self.num_predict {
            options.insert("num_predict".to_string(), serde_json::json!(predict));
        }
        if let Some(n) = self.repeat_last_n {
            options.insert("repeat_last_n".to_string(), serde_json::json!(n));
        }
        if let Some(penalty) = self.repeat_penalty {
            options.insert("repeat_penalty".to_string(), serde_json::json!(penalty));
        }
        if let Some(temp) = self.temperature {
            options.insert("temperature".to_string(), serde_json::json!(temp));
        }
        if let Some(seed) = self.seed {
            options.insert("seed".to_string(), serde_json::json!(seed));
        }
        if let Some(z) = self.tfs_z {
            options.insert("tfs_z".to_string(), serde_json::json!(z));
        }
        if let Some(k) = self.top_k {
            options.insert("top_k".to_string(), serde_json::json!(k));
        }
        if let Some(p) = self.top_p {
            options.insert("top_p".to_string(), serde_json::json!(p));
        }

        let stop_sequences = stop.or_else(|| self.stop.clone());
        if let Some(stop) = stop_sequences {
            options.insert("stop".to_string(), serde_json::json!(stop));
        }

        Ok(serde_json::Value::Object(options))
    }

    /// Build the request payload for the generate endpoint.
    /// Matches Python's `_generate_params`.
    pub fn generate_params(
        &self,
        prompt: &str,
        stop: Option<Vec<String>>,
    ) -> Result<serde_json::Value> {
        let options = self.build_options(stop)?;

        let mut payload = serde_json::json!({
            "prompt": prompt,
            "stream": true,
            "model": self.model,
        });

        if let serde_json::Value::Object(ref opts) = options
            && !opts.is_empty()
        {
            payload["options"] = options;
        }

        if let Some(reasoning) = self.reasoning {
            payload["think"] = serde_json::json!(reasoning);
        }

        if let Some(format) = &self.format {
            match format {
                OllamaFormat::Raw => {}
                OllamaFormat::Json => {
                    payload["format"] = serde_json::json!("json");
                }
                OllamaFormat::JsonSchema(schema) => {
                    payload["format"] = schema.clone();
                }
            }
        }

        if let Some(keep_alive) = &self.keep_alive {
            match keep_alive {
                KeepAlive::Seconds(s) => payload["keep_alive"] = serde_json::json!(s),
                KeepAlive::Duration(d) => payload["keep_alive"] = serde_json::json!(d),
            }
        }

        Ok(payload)
    }

    pub fn get_base_url(&self) -> String {
        let raw_url = match &self.base_url {
            Some(url) => url.clone(),
            None => env::var("OLLAMA_HOST").unwrap_or_else(|_| DEFAULT_API_BASE.to_string()),
        };

        let (cleaned_url, _) = parse_url_with_auth(Some(&raw_url));
        let url = cleaned_url.unwrap_or(raw_url);
        url.trim_end_matches('/').to_string()
    }

    fn build_client(&self) -> reqwest::Client {
        let raw_url = match &self.base_url {
            Some(url) => url.clone(),
            None => env::var("OLLAMA_HOST").unwrap_or_else(|_| DEFAULT_API_BASE.to_string()),
        };

        let (_, auth_headers) = parse_url_with_auth(Some(&raw_url));

        let mut all_headers = HashMap::new();
        merge_auth_headers(&mut all_headers, auth_headers);

        let mut builder = reqwest::Client::builder();

        if !all_headers.is_empty() {
            let mut header_map = reqwest::header::HeaderMap::new();
            for (key, value) in &all_headers {
                if let (Ok(name), Ok(val)) = (
                    reqwest::header::HeaderName::from_bytes(key.as_bytes()),
                    reqwest::header::HeaderValue::from_str(value),
                ) {
                    header_map.insert(name, val);
                }
            }
            builder = builder.default_headers(header_map);
        }

        builder.build().unwrap_or_else(|_| reqwest::Client::new())
    }

    async fn ensure_model_validated(&self) -> Result<()> {
        if self.validate_model_on_init
            && !self
                .model_validated
                .load(std::sync::atomic::Ordering::Relaxed)
        {
            let client = self.build_client();
            let base_url = self.get_base_url();
            validate_model(&client, &base_url, &self.model).await?;
            self.model_validated
                .store(true, std::sync::atomic::Ordering::Relaxed);
        }
        Ok(())
    }

    /// Build generation_info from a generate response.
    fn build_generation_info(
        response: &OllamaGenerateResponse,
    ) -> HashMap<String, serde_json::Value> {
        let mut info = HashMap::new();
        if let Some(model) = &response.model {
            info.insert("model".to_string(), serde_json::json!(model));
        }
        if let Some(done) = response.done {
            info.insert("done".to_string(), serde_json::json!(done));
        }
        if let Some(done_reason) = &response.done_reason {
            info.insert("done_reason".to_string(), serde_json::json!(done_reason));
        }
        if let Some(prompt_eval_count) = response.prompt_eval_count {
            info.insert(
                "prompt_eval_count".to_string(),
                serde_json::json!(prompt_eval_count),
            );
        }
        if let Some(eval_count) = response.eval_count {
            info.insert("eval_count".to_string(), serde_json::json!(eval_count));
        }
        if let Some(total_duration) = response.total_duration {
            info.insert(
                "total_duration".to_string(),
                serde_json::json!(total_duration),
            );
        }
        if let Some(load_duration) = response.load_duration {
            info.insert(
                "load_duration".to_string(),
                serde_json::json!(load_duration),
            );
        }
        if let Some(prompt_eval_duration) = response.prompt_eval_duration {
            info.insert(
                "prompt_eval_duration".to_string(),
                serde_json::json!(prompt_eval_duration),
            );
        }
        if let Some(eval_duration) = response.eval_duration {
            info.insert(
                "eval_duration".to_string(),
                serde_json::json!(eval_duration),
            );
        }
        if let Some(created_at) = &response.created_at {
            info.insert("created_at".to_string(), serde_json::json!(created_at));
        }
        info
    }

    /// Stream with aggregation — matches Python's `_stream_with_aggregation`.
    /// Aggregates all streaming chunks into a single `GenerationChunk`,
    /// accumulating thinking content separately.
    async fn stream_with_aggregation(
        &self,
        prompt: &str,
        stop: Option<Vec<String>>,
        run_manager: Option<&CallbackManagerForLLMRun>,
    ) -> Result<GenerationChunk> {
        self.ensure_model_validated().await?;

        let client = self.build_client();
        let base_url = self.get_base_url();
        let payload = self.generate_params(prompt, stop)?;

        let response = client
            .post(format!("{}/api/generate", base_url))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(Error::Http)?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|e| format!("<failed to read response body: {e}>"));
            return Err(Error::api(status, error_text));
        }

        let byte_stream = response
            .bytes_stream()
            .map(|r| r.map_err(std::io::Error::other));
        let reader = tokio::io::BufReader::new(StreamReader::new(byte_stream));
        let mut lines = reader.lines();

        let mut final_chunk: Option<GenerationChunk> = None;
        let mut thinking_content = String::new();

        while let Some(line) = lines
            .next_line()
            .await
            .map_err(|e| Error::Other(e.to_string()))?
        {
            if line.is_empty() {
                continue;
            }

            let stream_resp: OllamaGenerateResponse = serde_json::from_str(&line)
                .map_err(|e| Error::Other(format!("Failed to parse Ollama response: {}", e)))?;

            if let Some(thinking) = &stream_resp.thinking
                && !thinking.is_empty()
            {
                thinking_content.push_str(thinking);
            }

            let text = stream_resp.response.as_deref().unwrap_or("");

            let generation_info = if stream_resp.done == Some(true) {
                Some(Self::build_generation_info(&stream_resp))
            } else {
                None
            };

            let chunk = if let Some(info) = generation_info {
                GenerationChunk::with_info(text, info)
            } else {
                GenerationChunk::new(text)
            };

            if let Some(rm) = run_manager {
                rm.on_llm_new_token(&chunk.text, None);
            }

            final_chunk = Some(match final_chunk {
                Some(existing) => existing + chunk,
                None => chunk,
            });
        }

        let mut final_chunk = final_chunk
            .ok_or_else(|| Error::Other("No data received from Ollama stream.".to_string()))?;

        if !thinking_content.is_empty() {
            let info = final_chunk.generation_info.get_or_insert_with(HashMap::new);
            info.insert(
                "thinking".to_string(),
                serde_json::Value::String(thinking_content),
            );
        }

        Ok(final_chunk)
    }

    /// Create a streaming connection to the generate endpoint.
    /// Returns a stream of parsed NDJSON responses.
    /// Matches Python's `_stream` / `_astream`.
    async fn create_generate_stream(
        &self,
        prompt: &str,
        stop: Option<Vec<String>>,
    ) -> Result<std::pin::Pin<Box<dyn futures::Stream<Item = Result<OllamaGenerateResponse>> + Send>>>
    {
        self.ensure_model_validated().await?;

        let client = self.build_client();
        let base_url = self.get_base_url();
        let payload = self.generate_params(prompt, stop)?;

        let response = client
            .post(format!("{}/api/generate", base_url))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(Error::Http)?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|e| format!("<failed to read response body: {e}>"));
            return Err(Error::api(status, error_text));
        }

        let byte_stream = response
            .bytes_stream()
            .map(|r| r.map_err(std::io::Error::other));
        let reader = tokio::io::BufReader::new(StreamReader::new(byte_stream));
        let mut lines = reader.lines();

        let stream = async_stream::try_stream! {
            while let Some(line) = lines.next_line().await.map_err(|e| Error::Other(e.to_string()))? {
                if line.is_empty() {
                    continue;
                }

                let stream_resp: OllamaGenerateResponse = serde_json::from_str(&line)
                    .map_err(|e| Error::Other(format!("Failed to parse Ollama response: {}", e)))?;

                yield stream_resp;
            }
        };

        Ok(Box::pin(stream))
    }
}

#[async_trait]
impl BaseLanguageModel for OllamaLLM {
    fn llm_type(&self) -> &str {
        "ollama-llm"
    }

    fn model_name(&self) -> &str {
        &self.model
    }

    fn config(&self) -> &LanguageModelConfig {
        &self.language_model_config
    }

    async fn generate_prompt(
        &self,
        prompts: Vec<LanguageModelInput>,
        stop: Option<Vec<String>>,
        _callbacks: Option<Callbacks>,
    ) -> Result<LLMResult> {
        let string_prompts: Vec<String> = prompts
            .into_iter()
            .map(|p| self.convert_input(p))
            .collect::<Result<Vec<_>>>()?;
        self.generate_prompts(string_prompts, stop, None).await
    }

    fn get_ls_params(&self, stop: Option<&[String]>) -> LangSmithParams {
        let mut params = LangSmithParams::new()
            .with_provider("ollama")
            .with_model_name(&self.model);

        if let Some(num_predict) = self.num_predict {
            params = params.with_max_tokens(num_predict as u32);
        }
        if let Some(stop) = stop {
            params = params.with_stop(stop.to_vec());
        }

        params
    }
}

#[async_trait]
impl BaseLLM for OllamaLLM {
    fn llm_config(&self) -> &LLMConfig {
        &self.llm_config
    }

    async fn generate_prompts(
        &self,
        prompts: Vec<String>,
        stop: Option<Vec<String>>,
        run_manager: Option<&CallbackManagerForLLMRun>,
    ) -> Result<LLMResult> {
        let mut generations = Vec::new();
        for prompt in prompts {
            let chunk = self
                .stream_with_aggregation(&prompt, stop.clone(), run_manager)
                .await?;
            generations.push(vec![GenerationType::GenerationChunk(chunk)]);
        }
        Ok(LLMResult::new(generations))
    }

    async fn stream_prompt(
        &self,
        prompt: String,
        stop: Option<Vec<String>>,
        _run_manager: Option<&CallbackManagerForLLMRun>,
    ) -> Result<LLMStream> {
        let reasoning_enabled = self.reasoning == Some(true);
        let stop_sequences = self.stop.clone();

        let raw_stream = self.create_generate_stream(&prompt, stop).await?;

        let stream = async_stream::try_stream! {
            let mut pinned_stream = raw_stream;

            while let Some(result) = pinned_stream.next().await {
                let stream_resp = result?;

                let text = stream_resp.response.as_deref().unwrap_or("");
                let is_done = stream_resp.done == Some(true);

                let mut generation_info = HashMap::new();
                generation_info.insert(
                    "finish_reason".to_string(),
                    serde_json::json!(stop_sequences),
                );

                if reasoning_enabled && let Some(thinking) = &stream_resp.thinking && !thinking.is_empty() {
                            generation_info.insert(
                                "reasoning_content".to_string(),
                                serde_json::Value::String(thinking.clone()),
                            );
                }

                if is_done {
                    let done_info = OllamaLLM::build_generation_info(&stream_resp);
                    generation_info.extend(done_info);
                }

                let chunk = GenerationChunk::with_info(text, generation_info);

                yield chunk;
            }
        };

        Ok(Box::pin(stream))
    }
}
