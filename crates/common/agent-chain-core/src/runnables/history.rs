use std::collections::HashMap;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use serde_json::Value;

use async_trait::async_trait;

use crate::chat_history::BaseChatMessageHistory;
use crate::error::{Error, Result};
use crate::messages::{AIMessage, BaseMessage, HumanMessage};
use crate::runnables::base::Runnable;
use crate::runnables::config::RunnableConfig;
use crate::runnables::utils::ConfigurableFieldSpec;

pub type HistoryInvokeFn =
    Arc<dyn Fn(Value, Option<&RunnableConfig>) -> Result<Value> + Send + Sync>;

pub type HistoryAInvokeFn = Arc<
    dyn Fn(
            Value,
            Option<&RunnableConfig>,
        ) -> Pin<Box<dyn Future<Output = Result<Value>> + Send + '_>>
        + Send
        + Sync,
>;

pub type GetSessionHistoryFn =
    Arc<dyn Fn(&HashMap<String, String>) -> Arc<Mutex<dyn BaseChatMessageHistory>> + Send + Sync>;

pub enum HistoryRunnable {
    Lambda(
        Arc<
            dyn Fn(Vec<BaseMessage>, Option<&RunnableConfig>) -> Result<Vec<BaseMessage>>
                + Send
                + Sync,
        >,
    ),
}

impl HistoryRunnable {
    pub fn from_fn<F>(f: F) -> Self
    where
        F: Fn(Vec<BaseMessage>, Option<&RunnableConfig>) -> Result<Vec<BaseMessage>>
            + Send
            + Sync
            + 'static,
    {
        HistoryRunnable::Lambda(Arc::new(f))
    }

    pub fn invoke(
        &self,
        input: Vec<BaseMessage>,
        config: Option<&RunnableConfig>,
    ) -> Result<Vec<BaseMessage>> {
        match self {
            HistoryRunnable::Lambda(f) => f(input, config),
        }
    }
}

impl fmt::Debug for HistoryRunnable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HistoryRunnable::Lambda(_) => write!(f, "HistoryRunnable::Lambda(...)"),
        }
    }
}

pub struct RunnableWithMessageHistory {
    runnable: HistoryInvokeFn,
    runnable_async: Option<HistoryAInvokeFn>,
    get_session_history: GetSessionHistoryFn,
    input_messages_key: Option<String>,
    output_messages_key: Option<String>,
    history_messages_key: Option<String>,
    history_factory_config: Vec<ConfigurableFieldSpec>,
}

impl fmt::Debug for RunnableWithMessageHistory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RunnableWithMessageHistory")
            .field("input_messages_key", &self.input_messages_key)
            .field("output_messages_key", &self.output_messages_key)
            .field("history_messages_key", &self.history_messages_key)
            .finish()
    }
}

impl RunnableWithMessageHistory {
    pub fn new(
        runnable: HistoryInvokeFn,
        runnable_async: Option<HistoryAInvokeFn>,
        get_session_history: GetSessionHistoryFn,
        input_messages_key: Option<String>,
        output_messages_key: Option<String>,
        history_messages_key: Option<String>,
        history_factory_config: Option<Vec<ConfigurableFieldSpec>>,
    ) -> Self {
        let config = history_factory_config.unwrap_or_else(|| {
            vec![ConfigurableFieldSpec {
                id: "session_id".to_string(),
                annotation: "str".to_string(),
                name: Some("Session ID".to_string()),
                description: Some("Unique identifier for a session.".to_string()),
                default: Some(Value::String(String::new())),
                is_shared: true,
                dependencies: None,
            }]
        });

        Self {
            runnable,
            runnable_async,
            get_session_history,
            input_messages_key,
            output_messages_key,
            history_messages_key,
            history_factory_config: config,
        }
    }

    pub fn from_messages_fn<F>(
        func: F,
        get_session_history: GetSessionHistoryFn,
        input_messages_key: Option<String>,
        output_messages_key: Option<String>,
        history_messages_key: Option<String>,
        history_factory_config: Option<Vec<ConfigurableFieldSpec>>,
    ) -> Self
    where
        F: Fn(Vec<BaseMessage>, Option<&RunnableConfig>) -> Result<Vec<BaseMessage>>
            + Send
            + Sync
            + 'static,
    {
        let func = Arc::new(func);
        let runnable: HistoryInvokeFn = {
            let func = func.clone();
            Arc::new(move |input: Value, config: Option<&RunnableConfig>| {
                let messages: Vec<BaseMessage> = serde_json::from_value(input).map_err(|e| {
                    Error::Other(format!("Failed to deserialize input messages: {}", e))
                })?;
                let result = func(messages, config)?;
                serde_json::to_value(&result).map_err(|e| {
                    Error::Other(format!("Failed to serialize output messages: {}", e))
                })
            })
        };

        Self::new(
            runnable,
            None,
            get_session_history,
            input_messages_key,
            output_messages_key,
            history_messages_key,
            history_factory_config,
        )
    }

    pub fn from_history_runnable(
        runnable: HistoryRunnable,
        get_session_history: GetSessionHistoryFn,
        input_messages_key: Option<String>,
        output_messages_key: Option<String>,
        history_messages_key: Option<String>,
        history_factory_config: Option<Vec<ConfigurableFieldSpec>>,
    ) -> Self {
        let runnable = Arc::new(runnable);
        let invoke_fn: HistoryInvokeFn = {
            let runnable = runnable.clone();
            Arc::new(move |input: Value, config: Option<&RunnableConfig>| {
                let messages: Vec<BaseMessage> = serde_json::from_value(input).map_err(|e| {
                    Error::Other(format!("Failed to deserialize input messages: {}", e))
                })?;
                let result = runnable.invoke(messages, config)?;
                serde_json::to_value(&result).map_err(|e| {
                    Error::Other(format!("Failed to serialize output messages: {}", e))
                })
            })
        };

        Self::new(
            invoke_fn,
            None,
            get_session_history,
            input_messages_key,
            output_messages_key,
            history_messages_key,
            history_factory_config,
        )
    }

    pub fn config_specs(&self) -> &[ConfigurableFieldSpec] {
        &self.history_factory_config
    }

    pub fn get_input_schema(&self) -> Value {
        if let (Some(input_key), Some(_)) = (&self.input_messages_key, &self.history_messages_key) {
            serde_json::json!({
                "title": "RunnableWithChatHistoryInput",
                "type": "object",
                "properties": {
                    input_key: {
                        "anyOf": [
                            { "type": "string" },
                            { "type": "object" },
                            { "type": "array", "items": { "type": "object" } }
                        ]
                    }
                }
            })
        } else if let Some(input_key) = &self.input_messages_key {
            serde_json::json!({
                "title": "RunnableWithChatHistoryInput",
                "type": "object",
                "properties": {
                    input_key: {
                        "type": "array",
                        "items": { "type": "object" }
                    }
                }
            })
        } else {
            serde_json::json!({
                "title": "RunnableWithChatHistoryInput",
                "type": "array",
                "items": { "type": "object" }
            })
        }
    }

    pub fn get_output_schema(&self) -> Value {
        serde_json::json!({
            "title": "RunnableWithChatHistoryOutput",
            "type": "array",
            "items": { "type": "object" }
        })
    }

    pub fn get_input_messages(&self, input: &Value) -> Result<Vec<BaseMessage>> {
        let value = if let Some(obj) = input.as_object() {
            if let Some(ref key) = self.input_messages_key {
                obj.get(key).ok_or_else(|| {
                    Error::Other(format!("Expected input key '{}' in dict input", key))
                })?
            } else if obj.len() == 1 {
                obj.values()
                    .next()
                    .ok_or_else(|| Error::Other("Empty dict input".to_string()))?
            } else {
                obj.get("input").ok_or_else(|| {
                    Error::Other("Expected 'input' key in multi-key dict input".to_string())
                })?
            }
        } else {
            input
        };

        if let Some(s) = value.as_str() {
            return Ok(vec![BaseMessage::Human(
                HumanMessage::builder().content(s).build(),
            )]);
        }

        if let Some(arr) = value.as_array() {
            if arr.is_empty() {
                return Ok(Vec::new());
            }
            if arr.first().is_some_and(|v| v.is_array()) {
                if arr.len() != 1 {
                    return Err(Error::Other(format!(
                        "Expected a single list of messages. Got {} lists.",
                        arr.len()
                    )));
                }
                let inner = &arr[0];
                return serde_json::from_value::<Vec<BaseMessage>>(inner.clone()).map_err(|e| {
                    Error::Other(format!(
                        "Failed to deserialize nested input messages: {}",
                        e
                    ))
                });
            }
            return serde_json::from_value::<Vec<BaseMessage>>(Value::Array(arr.clone()))
                .map_err(|e| Error::Other(format!("Failed to deserialize input messages: {}", e)));
        }

        serde_json::from_value::<BaseMessage>(value.clone())
            .map(|m| vec![m])
            .map_err(|e| {
                Error::Other(format!(
                    "Expected str, BaseMessage, or list of BaseMessage. \
                 Failed to deserialize: {}",
                    e
                ))
            })
    }

    pub fn get_output_messages(&self, output: &Value) -> Result<Vec<BaseMessage>> {
        let value = if let Some(obj) = output.as_object() {
            let key = if let Some(ref key) = self.output_messages_key {
                key.as_str()
            } else if obj.len() == 1 {
                obj.keys().next().map(|s| s.as_str()).unwrap_or("output")
            } else {
                "output"
            };

            if let Some(val) = obj.get(key) {
                val
            } else if let Some(generations) = obj.get("generations") {
                generations
                    .get(0)
                    .and_then(|g| g.get(0))
                    .and_then(|g| g.get("message"))
                    .ok_or_else(|| {
                        Error::Other(
                            "Could not extract message from generations output".to_string(),
                        )
                    })?
            } else {
                return Err(Error::Other(format!(
                    "Expected key '{}' or 'generations' in output dict",
                    key
                )));
            }
        } else {
            output
        };

        if let Some(s) = value.as_str() {
            return Ok(vec![BaseMessage::AI(
                AIMessage::builder().content(s).build(),
            )]);
        }

        if let Some(arr) = value.as_array() {
            return serde_json::from_value::<Vec<BaseMessage>>(Value::Array(arr.clone())).map_err(
                |e| Error::Other(format!("Failed to deserialize output messages: {}", e)),
            );
        }

        serde_json::from_value::<BaseMessage>(value.clone())
            .map(|m| vec![m])
            .map_err(|e| {
                Error::Other(format!(
                    "Expected str, BaseMessage, or list of BaseMessage. \
                 Failed to deserialize output: {}",
                    e
                ))
            })
    }

    pub fn enter_history(
        &self,
        input: &Value,
        history: &Arc<Mutex<dyn BaseChatMessageHistory>>,
    ) -> Result<Vec<BaseMessage>> {
        let guard = history
            .lock()
            .map_err(|e| Error::Other(format!("history lock poisoned: {e}")))?;
        let mut messages = guard.messages();
        drop(guard);

        if self.history_messages_key.is_none() {
            let input_val = if self.input_messages_key.is_some() {
                input
                    .as_object()
                    .and_then(|obj| obj.get(self.input_messages_key.as_ref()?))
                    .unwrap_or(input)
            } else {
                input
            };
            let input_messages = self.get_input_messages(input_val)?;
            messages.extend(input_messages);
        }

        Ok(messages)
    }

    pub fn exit_history(
        &self,
        input: &Value,
        output: &Value,
        history: &Arc<Mutex<dyn BaseChatMessageHistory>>,
    ) -> Result<()> {
        let input_messages = self.get_input_messages(input)?;
        let output_messages = self.get_output_messages(output)?;

        let mut to_save = input_messages;
        to_save.extend(output_messages);

        let mut guard = history
            .lock()
            .map_err(|e| Error::Other(format!("history lock poisoned: {e}")))?;
        guard.add_messages(&to_save);

        Ok(())
    }

    pub fn merge_configs(
        &self,
        config: RunnableConfig,
    ) -> Result<(RunnableConfig, Arc<Mutex<dyn BaseChatMessageHistory>>)> {
        let expected_keys: Vec<&str> = self
            .history_factory_config
            .iter()
            .map(|s| s.id.as_str())
            .collect();

        let mut params = HashMap::new();
        for key in &expected_keys {
            if let Some(val) = config.configurable.get(*key) {
                let s = match val {
                    Value::String(s) => s.clone(),
                    other => other.to_string(),
                };
                params.insert(key.to_string(), s);
            }
        }

        let history = (self.get_session_history)(&params);
        Ok((config, history))
    }

    pub fn invoke_with_history(
        &self,
        input: Value,
        config: Option<RunnableConfig>,
    ) -> Result<Value> {
        let config = config.unwrap_or_default();
        let (config, history) = self.merge_configs(config)?;

        let history_messages = self.enter_history(&input, &history)?;

        let augmented_input = if let Some(ref history_key) = self.history_messages_key {
            let mut obj = match input.as_object() {
                Some(obj) => obj.clone(),
                None => {
                    let mut map = serde_json::Map::new();
                    if let Some(ref input_key) = self.input_messages_key {
                        map.insert(input_key.clone(), input.clone());
                    }
                    map
                }
            };
            let history_value = serde_json::to_value(&history_messages).map_err(|e| {
                Error::Other(format!("Failed to serialize history messages: {}", e))
            })?;
            obj.insert(history_key.clone(), history_value);
            Value::Object(obj)
        } else {
            serde_json::to_value(&history_messages).map_err(|e| {
                Error::Other(format!("Failed to serialize augmented messages: {}", e))
            })?
        };

        let output = (self.runnable)(augmented_input, Some(&config))?;

        self.exit_history(&input, &output, &history)?;

        Ok(output)
    }

    pub async fn ainvoke_with_history(
        &self,
        input: Value,
        config: Option<RunnableConfig>,
    ) -> Result<Value> {
        let config = config.unwrap_or_default();
        let (config, history) = self.merge_configs(config)?;

        let history_messages = self.enter_history(&input, &history)?;

        let augmented_input = if let Some(ref history_key) = self.history_messages_key {
            let mut obj = match input.as_object() {
                Some(obj) => obj.clone(),
                None => {
                    let mut map = serde_json::Map::new();
                    if let Some(ref input_key) = self.input_messages_key {
                        map.insert(input_key.clone(), input.clone());
                    }
                    map
                }
            };
            let history_value = serde_json::to_value(&history_messages).map_err(|e| {
                Error::Other(format!("Failed to serialize history messages: {}", e))
            })?;
            obj.insert(history_key.clone(), history_value);
            Value::Object(obj)
        } else {
            serde_json::to_value(&history_messages).map_err(|e| {
                Error::Other(format!("Failed to serialize augmented messages: {}", e))
            })?
        };

        let output = if let Some(ref async_fn) = self.runnable_async {
            async_fn(augmented_input, Some(&config)).await?
        } else {
            (self.runnable)(augmented_input, Some(&config))?
        };

        self.exit_history(&input, &output, &history)?;

        Ok(output)
    }

    pub fn invoke_messages(
        &self,
        input: Vec<BaseMessage>,
        config: Option<RunnableConfig>,
    ) -> Result<Vec<BaseMessage>> {
        let input_value = serde_json::to_value(&input)
            .map_err(|e| Error::Other(format!("Failed to serialize input messages: {}", e)))?;
        let output_value = self.invoke_with_history(input_value, config)?;
        serde_json::from_value::<Vec<BaseMessage>>(output_value)
            .map_err(|e| Error::Other(format!("Failed to deserialize output messages: {}", e)))
    }

    pub async fn ainvoke_messages(
        &self,
        input: Vec<BaseMessage>,
        config: Option<RunnableConfig>,
    ) -> Result<Vec<BaseMessage>> {
        let input_value = serde_json::to_value(&input)
            .map_err(|e| Error::Other(format!("Failed to serialize input messages: {}", e)))?;
        let output_value = self.ainvoke_with_history(input_value, config).await?;
        serde_json::from_value::<Vec<BaseMessage>>(output_value)
            .map_err(|e| Error::Other(format!("Failed to deserialize output messages: {}", e)))
    }
}

#[async_trait]
impl Runnable for RunnableWithMessageHistory {
    type Input = Value;
    type Output = Value;

    fn name(&self) -> Option<String> {
        Some("RunnableWithMessageHistory".to_string())
    }

    fn invoke(&self, input: Self::Input, config: Option<RunnableConfig>) -> Result<Self::Output> {
        self.invoke_with_history(input, config)
    }

    async fn ainvoke(
        &self,
        input: Self::Input,
        config: Option<RunnableConfig>,
    ) -> Result<Self::Output>
    where
        Self: 'static,
    {
        self.ainvoke_with_history(input, config).await
    }

    fn config_specs(&self) -> Result<Vec<ConfigurableFieldSpec>> {
        Ok(self.history_factory_config.clone())
    }

    fn get_input_schema(&self, _config: Option<&RunnableConfig>) -> Value {
        RunnableWithMessageHistory::get_input_schema(self)
    }

    fn get_output_schema(&self, _config: Option<&RunnableConfig>) -> Value {
        RunnableWithMessageHistory::get_output_schema(self)
    }
}
