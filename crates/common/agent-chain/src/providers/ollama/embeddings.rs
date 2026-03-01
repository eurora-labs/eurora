//! Ollama embeddings model implementation.
//!
//! Matches Python `langchain_ollama/embeddings.py`.

use std::collections::HashMap;
use std::env;

use async_trait::async_trait;
use serde::Deserialize;

use super::chat_models::KeepAlive;
use super::utils::{merge_auth_headers, parse_url_with_auth, validate_model};
use crate::error::{Error, Result};

const DEFAULT_API_BASE: &str = "http://localhost:11434";

/// Ollama embedding model integration.
///
/// Matches Python's `OllamaEmbeddings` class.
#[derive(Debug)]
pub struct OllamaEmbeddings {
    model: String,
    validate_model_on_init: bool,
    base_url: Option<String>,
    client_kwargs: HashMap<String, serde_json::Value>,
    async_client_kwargs: HashMap<String, serde_json::Value>,
    sync_client_kwargs: HashMap<String, serde_json::Value>,
    mirostat: Option<i32>,
    mirostat_eta: Option<f64>,
    mirostat_tau: Option<f64>,
    num_ctx: Option<u32>,
    num_gpu: Option<i32>,
    keep_alive: Option<KeepAlive>,
    num_thread: Option<i32>,
    repeat_last_n: Option<i32>,
    repeat_penalty: Option<f64>,
    temperature: Option<f64>,
    stop: Option<Vec<String>>,
    tfs_z: Option<f64>,
    top_k: Option<i32>,
    top_p: Option<f64>,
    model_validated: std::sync::atomic::AtomicBool,
}

impl Clone for OllamaEmbeddings {
    fn clone(&self) -> Self {
        Self {
            model: self.model.clone(),
            validate_model_on_init: self.validate_model_on_init,
            base_url: self.base_url.clone(),
            client_kwargs: self.client_kwargs.clone(),
            async_client_kwargs: self.async_client_kwargs.clone(),
            sync_client_kwargs: self.sync_client_kwargs.clone(),
            mirostat: self.mirostat,
            mirostat_eta: self.mirostat_eta,
            mirostat_tau: self.mirostat_tau,
            num_ctx: self.num_ctx,
            num_gpu: self.num_gpu,
            keep_alive: self.keep_alive.clone(),
            num_thread: self.num_thread,
            repeat_last_n: self.repeat_last_n,
            repeat_penalty: self.repeat_penalty,
            temperature: self.temperature,
            stop: self.stop.clone(),
            tfs_z: self.tfs_z,
            top_k: self.top_k,
            top_p: self.top_p,
            model_validated: std::sync::atomic::AtomicBool::new(
                self.model_validated
                    .load(std::sync::atomic::Ordering::Relaxed),
            ),
        }
    }
}

#[derive(Deserialize)]
struct OllamaEmbedResponse {
    embeddings: Vec<Vec<f32>>,
}

impl OllamaEmbeddings {
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            validate_model_on_init: false,
            base_url: None,
            client_kwargs: HashMap::new(),
            async_client_kwargs: HashMap::new(),
            sync_client_kwargs: HashMap::new(),
            mirostat: None,
            mirostat_eta: None,
            mirostat_tau: None,
            num_ctx: None,
            num_gpu: None,
            keep_alive: None,
            num_thread: None,
            repeat_last_n: None,
            repeat_penalty: None,
            temperature: None,
            stop: None,
            tfs_z: None,
            top_k: None,
            top_p: None,
            model_validated: std::sync::atomic::AtomicBool::new(false),
        }
    }

    pub fn validate_model_on_init(mut self, validate: bool) -> Self {
        self.validate_model_on_init = validate;
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

    pub fn keep_alive(mut self, duration: impl Into<String>) -> Self {
        self.keep_alive = Some(KeepAlive::Duration(duration.into()));
        self
    }

    pub fn keep_alive_seconds(mut self, seconds: i64) -> Self {
        self.keep_alive = Some(KeepAlive::Seconds(seconds));
        self
    }

    pub fn num_thread(mut self, thread: i32) -> Self {
        self.num_thread = Some(thread);
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

    pub fn temperature(mut self, temp: f64) -> Self {
        self.temperature = Some(temp);
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

    /// Build the options dict from generation parameters.
    /// Matches Python's `_default_params` property.
    pub fn build_options(&self) -> Result<serde_json::Value> {
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
        if let Some(n) = self.repeat_last_n {
            options.insert("repeat_last_n".to_string(), serde_json::json!(n));
        }
        if let Some(penalty) = self.repeat_penalty {
            options.insert("repeat_penalty".to_string(), serde_json::json!(penalty));
        }
        if let Some(temp) = self.temperature {
            options.insert("temperature".to_string(), serde_json::json!(temp));
        }
        if let Some(ref stop) = self.stop {
            options.insert("stop".to_string(), serde_json::json!(stop));
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

        Ok(serde_json::Value::Object(options))
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

    fn build_embed_payload(&self, input: &[String]) -> Result<serde_json::Value> {
        let options = self.build_options()?;

        let mut payload = serde_json::json!({
            "model": self.model,
            "input": input,
        });

        if let serde_json::Value::Object(ref opts) = options
            && !opts.is_empty()
        {
            payload["options"] = options;
        }

        if let Some(keep_alive) = &self.keep_alive {
            match keep_alive {
                KeepAlive::Seconds(s) => payload["keep_alive"] = serde_json::json!(s),
                KeepAlive::Duration(d) => payload["keep_alive"] = serde_json::json!(d),
            }
        }

        Ok(payload)
    }

    async fn embed_internal(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        self.ensure_model_validated().await?;

        let client = self.build_client();
        let base_url = self.get_base_url();
        let payload = self.build_embed_payload(&texts)?;

        let response = client
            .post(format!("{}/api/embed", base_url))
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

        let embed_response: OllamaEmbedResponse = response.json().await.map_err(|e| {
            Error::Json(serde_json::Error::io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                e.to_string(),
            )))
        })?;

        Ok(embed_response.embeddings)
    }
}

#[async_trait]
impl crate::embeddings::Embeddings for OllamaEmbeddings {
    fn embed_documents(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(self.aembed_documents(texts))
        })
    }

    fn embed_query(&self, text: &str) -> Result<Vec<f32>> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(self.aembed_query(text))
        })
    }

    async fn aembed_documents(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        self.embed_internal(texts).await
    }

    async fn aembed_query(&self, text: &str) -> Result<Vec<f32>> {
        let results = self.embed_internal(vec![text.to_string()]).await?;

        results
            .into_iter()
            .next()
            .ok_or_else(|| Error::Other("No embeddings returned".to_string()))
    }
}
