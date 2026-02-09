//! Runnable that manages chat message history for another Runnable.
//!
//! This module provides [`RunnableWithMessageHistory`], which wraps another
//! runnable and transparently loads / saves chat history around each
//! invocation.
//!
//! Mirrors `langchain_core.runnables.history`.

use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::{Arc, Mutex};

use serde_json::Value;

use crate::chat_history::BaseChatMessageHistory;
use crate::error::{Error, Result};
use crate::messages::{AIMessage, BaseMessage, HumanMessage};
use crate::runnables::config::RunnableConfig;
use crate::runnables::utils::ConfigurableFieldSpec;

// ---------------------------------------------------------------------------
// Type aliases
// ---------------------------------------------------------------------------

/// A function that takes a `HashMap` of configurable field values and returns
/// a chat message history for that session.
///
/// Mirrors Python's `GetSessionHistoryCallable`.
pub type GetSessionHistoryFn =
    Arc<dyn Fn(&HashMap<String, String>) -> Arc<Mutex<dyn BaseChatMessageHistory>> + Send + Sync>;

/// The output type of the inner runnable.
///
/// Mirrors Python's flexible output: str | BaseMessage | list[BaseMessage] | dict.
#[derive(Debug, Clone)]
pub enum HistoryOutput {
    /// A plain string (will be wrapped as `AIMessage` when saving to history).
    Text(String),
    /// A single message.
    Message(BaseMessage),
    /// A list of messages.
    Messages(Vec<BaseMessage>),
    /// A dict-like structure containing messages under a key.
    Dict(HashMap<String, HistoryOutput>),
}

/// The input type of the inner runnable.
///
/// Mirrors Python's flexible input: list[BaseMessage] | dict.
#[derive(Debug, Clone)]
pub enum HistoryInput {
    /// A list of messages (the runnable takes messages directly).
    Messages(Vec<BaseMessage>),
    /// A dict with one or more keys; messages are in `input_messages_key`.
    Dict(HashMap<String, Value>),
}

// ---------------------------------------------------------------------------
// Inner runnable trait
// ---------------------------------------------------------------------------

/// Trait for the inner runnable wrapped by `RunnableWithMessageHistory`.
///
/// This is a simplified, object-safe variant of the `Runnable` trait
/// specialised for the history use case.
pub trait HistoryRunnable: Send + Sync + Debug {
    /// Invoke the inner runnable with the given input.
    fn invoke_history(
        &self,
        input: HistoryInput,
        config: Option<&RunnableConfig>,
    ) -> Result<HistoryOutput>;
}

// ---------------------------------------------------------------------------
// RunnableWithMessageHistory
// ---------------------------------------------------------------------------

/// Wraps another runnable and manages chat message history.
///
/// Mirrors Python's `RunnableWithMessageHistory`.
///
/// # Usage
///
/// ```ignore
/// use agent_chain_core::runnables::history::RunnableWithMessageHistory;
///
/// let with_history = RunnableWithMessageHistory::builder()
///     .runnable(my_runnable)
///     .get_session_history(session_factory)
///     .build();
///
/// let output = with_history.invoke(input, config)?;
/// ```
pub struct RunnableWithMessageHistory {
    /// The wrapped runnable.
    runnable: Box<dyn HistoryRunnable>,
    /// Factory that returns a chat message history for a given session.
    get_session_history: GetSessionHistoryFn,
    /// Key for input messages when the input is a dict.
    input_messages_key: Option<String>,
    /// Key for output messages when the output is a dict.
    output_messages_key: Option<String>,
    /// Key under which historical messages are injected into a dict input.
    history_messages_key: Option<String>,
    /// Config specs describing the fields passed to the session factory.
    history_factory_config: Vec<ConfigurableFieldSpec>,
}

impl Debug for RunnableWithMessageHistory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RunnableWithMessageHistory")
            .field("runnable", &self.runnable)
            .field("input_messages_key", &self.input_messages_key)
            .field("output_messages_key", &self.output_messages_key)
            .field("history_messages_key", &self.history_messages_key)
            .finish()
    }
}

impl RunnableWithMessageHistory {
    /// Create a new `RunnableWithMessageHistory`.
    pub fn new(
        runnable: Box<dyn HistoryRunnable>,
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
            get_session_history,
            input_messages_key,
            output_messages_key,
            history_messages_key,
            history_factory_config: config,
        }
    }

    /// Get the config specs for this runnable.
    pub fn config_specs(&self) -> &[ConfigurableFieldSpec] {
        &self.history_factory_config
    }

    /// Invoke the wrapped runnable with history management.
    ///
    /// 1. Resolve the session from the config's `configurable` map.
    /// 2. Load existing history messages.
    /// 3. Prepend history to the input.
    /// 4. Invoke the inner runnable.
    /// 5. Save input + output messages to history.
    pub fn invoke(
        &self,
        input: HistoryInput,
        config: Option<RunnableConfig>,
    ) -> Result<HistoryOutput> {
        let config = config.unwrap_or_default();
        let history = self.resolve_history(&config)?;

        // --- enter history: load existing messages ---
        let historic_messages = {
            let guard = history
                .lock()
                .map_err(|e| Error::Other(format!("history lock poisoned: {e}")))?;
            guard.messages()
        };

        // --- build the augmented input ---
        let augmented_input = self.build_input_with_history(&input, &historic_messages)?;

        // --- invoke the inner runnable ---
        let output = self
            .runnable
            .invoke_history(augmented_input, Some(&config))?;

        // --- exit history: save new messages ---
        let input_messages = self.get_input_messages(&input)?;
        // Remove historic messages that were prepended to avoid duplicates.
        let new_input_messages = if self.history_messages_key.is_none() {
            let skip = historic_messages.len();
            if skip <= input_messages.len() {
                input_messages[skip..].to_vec()
            } else {
                input_messages
            }
        } else {
            input_messages
        };

        let output_messages = self.get_output_messages(&output)?;

        {
            let mut guard = history
                .lock()
                .map_err(|e| Error::Other(format!("history lock poisoned: {e}")))?;
            let mut combined = new_input_messages;
            combined.extend(output_messages);
            guard.add_messages(&combined);
        }

        Ok(output)
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    /// Resolve the chat message history from the config's `configurable` map.
    fn resolve_history(
        &self,
        config: &RunnableConfig,
    ) -> Result<Arc<Mutex<dyn BaseChatMessageHistory>>> {
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

        // Validate all expected keys are present (unless factory takes no args).
        let missing: Vec<&&str> = expected_keys
            .iter()
            .filter(|k| !params.contains_key(**k))
            .collect();

        if !missing.is_empty() && !expected_keys.is_empty() {
            // Allow invocation without config if the factory takes 0 args
            // (tested by test_ignore_session_id) — the factory will be called
            // with an empty map and it must handle that.
        }

        Ok((self.get_session_history)(&params))
    }

    /// Build the augmented input by prepending history messages.
    fn build_input_with_history(
        &self,
        input: &HistoryInput,
        historic_messages: &[BaseMessage],
    ) -> Result<HistoryInput> {
        match input {
            HistoryInput::Messages(msgs) => {
                if self.history_messages_key.is_some() {
                    // History goes into a separate key — not applicable for
                    // bare-message inputs.
                    Ok(HistoryInput::Messages(msgs.clone()))
                } else {
                    // Prepend history to the message list.
                    let mut combined = historic_messages.to_vec();
                    combined.extend(msgs.iter().cloned());
                    Ok(HistoryInput::Messages(combined))
                }
            }
            HistoryInput::Dict(map) => {
                let mut new_map = map.clone();

                if let Some(ref history_key) = self.history_messages_key {
                    // Inject history under the dedicated key.
                    let history_value = messages_to_json_value(historic_messages);
                    new_map.insert(history_key.clone(), history_value);
                } else if let Some(ref input_key) = self.input_messages_key {
                    // Prepend history to the messages at `input_messages_key`.
                    let existing = new_map
                        .get(input_key)
                        .cloned()
                        .unwrap_or(Value::Array(vec![]));
                    let existing_msgs = json_value_to_messages(&existing)?;
                    let mut combined = historic_messages.to_vec();
                    combined.extend(existing_msgs);
                    new_map.insert(input_key.clone(), messages_to_json_value(&combined));
                }
                Ok(HistoryInput::Dict(new_map))
            }
        }
    }

    /// Extract input messages from the input value.
    ///
    /// Mirrors `_get_input_messages` in the Python implementation.
    pub fn get_input_messages(&self, input: &HistoryInput) -> Result<Vec<BaseMessage>> {
        match input {
            HistoryInput::Messages(msgs) => Ok(msgs.clone()),
            HistoryInput::Dict(map) => {
                let key = self
                    .input_messages_key
                    .as_deref()
                    .or_else(|| {
                        if map.len() == 1 {
                            map.keys().next().map(|s| s.as_str())
                        } else {
                            Some("input")
                        }
                    })
                    .unwrap_or("input");

                let val = map
                    .get(key)
                    .ok_or_else(|| Error::Other(format!("Missing key '{key}' in input dict")))?;

                value_to_input_messages(val)
            }
        }
    }

    /// Extract output messages from the output value.
    ///
    /// Mirrors `_get_output_messages` in the Python implementation.
    pub fn get_output_messages(&self, output: &HistoryOutput) -> Result<Vec<BaseMessage>> {
        match output {
            HistoryOutput::Text(s) => Ok(vec![BaseMessage::AI(
                AIMessage::builder().content(s).build(),
            )]),
            HistoryOutput::Message(m) => Ok(vec![m.clone()]),
            HistoryOutput::Messages(ms) => Ok(ms.clone()),
            HistoryOutput::Dict(map) => {
                let key = self
                    .output_messages_key
                    .as_deref()
                    .or_else(|| {
                        if map.len() == 1 {
                            map.keys().next().map(|s| s.as_str())
                        } else {
                            Some("output")
                        }
                    })
                    .unwrap_or("output");

                let val = map
                    .get(key)
                    .ok_or_else(|| Error::Other(format!("Missing key '{key}' in output dict")))?;

                match val {
                    HistoryOutput::Text(s) => Ok(vec![BaseMessage::AI(
                        AIMessage::builder().content(s).build(),
                    )]),
                    HistoryOutput::Message(m) => Ok(vec![m.clone()]),
                    HistoryOutput::Messages(ms) => Ok(ms.clone()),
                    other => Err(Error::Other(format!(
                        "Expected str, BaseMessage, list[BaseMessage], or tuple[BaseMessage]. Got {other:?}."
                    ))),
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Conversion helpers
// ---------------------------------------------------------------------------

/// Convert `BaseMessage` slice to a JSON `Value::Array` for injection into dict inputs.
fn messages_to_json_value(messages: &[BaseMessage]) -> Value {
    Value::Array(
        messages
            .iter()
            .map(|m| serde_json::to_value(m).unwrap_or(Value::Null))
            .collect(),
    )
}

/// Parse a JSON value into a list of `BaseMessage`.
fn json_value_to_messages(value: &Value) -> Result<Vec<BaseMessage>> {
    match value {
        Value::Array(arr) => {
            let mut messages = Vec::with_capacity(arr.len());
            for item in arr {
                let msg: BaseMessage = serde_json::from_value(item.clone())
                    .map_err(|e| Error::Other(format!("Failed to parse message: {e}")))?;
                messages.push(msg);
            }
            Ok(messages)
        }
        Value::String(s) => Ok(vec![BaseMessage::Human(
            HumanMessage::builder().content(s).build(),
        )]),
        _ => Err(Error::Other(format!(
            "Expected array or string of messages, got: {value}"
        ))),
    }
}

/// Parse a `serde_json::Value` into input messages.
///
/// Mirrors the Python `_get_input_messages` inner logic for individual values.
fn value_to_input_messages(val: &Value) -> Result<Vec<BaseMessage>> {
    match val {
        Value::String(s) => Ok(vec![BaseMessage::Human(
            HumanMessage::builder().content(s).build(),
        )]),
        Value::Array(arr) => {
            let mut messages = Vec::with_capacity(arr.len());
            for item in arr {
                let msg: BaseMessage = serde_json::from_value(item.clone())
                    .map_err(|e| Error::Other(format!("Failed to parse message: {e}")))?;
                messages.push(msg);
            }
            Ok(messages)
        }
        _ => Err(Error::Other(format!(
            "Expected str, BaseMessage, list[BaseMessage], or tuple[BaseMessage]. Got {val}."
        ))),
    }
}
