//! Runnable that manages chat message history for another Runnable.
//!
//! This module provides [`RunnableWithMessageHistory`], which wraps another
//! runnable and transparently loads / saves chat history around each
//! invocation.
//!
//! Mirrors `langchain_core.runnables.history`.

use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Mutex};

use serde_json::Value;

use crate::chat_history::BaseChatMessageHistory;
use crate::error::{Error, Result};
use crate::messages::BaseMessage;
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

/// Function type for the inner runnable: takes messages and an optional config,
/// returns messages.
pub type HistoryInvokeFn = Arc<
    dyn Fn(Vec<BaseMessage>, Option<&RunnableConfig>) -> Result<Vec<BaseMessage>> + Send + Sync,
>;

// ---------------------------------------------------------------------------
// HistoryRunnable
// ---------------------------------------------------------------------------

/// The inner runnable wrapped by `RunnableWithMessageHistory`.
///
/// Takes `Vec<BaseMessage>` as input and returns `Vec<BaseMessage>` as output.
pub enum HistoryRunnable {
    /// A lambda/closure-based runnable.
    Lambda(HistoryInvokeFn),
}

impl HistoryRunnable {
    /// Create a `HistoryRunnable` from a closure.
    pub fn from_fn<F>(f: F) -> Self
    where
        F: Fn(Vec<BaseMessage>, Option<&RunnableConfig>) -> Result<Vec<BaseMessage>>
            + Send
            + Sync
            + 'static,
    {
        HistoryRunnable::Lambda(Arc::new(f))
    }

    /// Invoke the runnable with the given messages.
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
/// use agent_chain_core::runnables::history::{HistoryRunnable, RunnableWithMessageHistory};
///
/// let runnable = HistoryRunnable::from_fn(|msgs, _cfg| Ok(msgs));
///
/// let with_history = RunnableWithMessageHistory::new(
///     runnable,
///     session_factory,
///     None,
/// );
///
/// let output = with_history.invoke(vec![human("hello")], Some(config))?;
/// ```
pub struct RunnableWithMessageHistory {
    /// The wrapped runnable.
    runnable: HistoryRunnable,
    /// Factory that returns a chat message history for a given session.
    get_session_history: GetSessionHistoryFn,
    /// Config specs describing the fields passed to the session factory.
    history_factory_config: Vec<ConfigurableFieldSpec>,
}

impl fmt::Debug for RunnableWithMessageHistory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RunnableWithMessageHistory")
            .field("runnable", &self.runnable)
            .finish()
    }
}

impl RunnableWithMessageHistory {
    /// Create a new `RunnableWithMessageHistory`.
    pub fn new(
        runnable: HistoryRunnable,
        get_session_history: GetSessionHistoryFn,
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
            history_factory_config: config,
        }
    }

    /// Get the config specs for this runnable.
    pub fn config_specs(&self) -> &[ConfigurableFieldSpec] {
        &self.history_factory_config
    }

    /// Get a JSON schema describing the expected input.
    ///
    /// Mirrors `RunnableWithMessageHistory.get_input_schema()` from Python.
    pub fn get_input_schema(&self) -> Value {
        serde_json::json!({
            "title": "RunnableWithChatHistoryInput",
            "type": "array",
            "items": { "type": "object" }
        })
    }

    /// Get a JSON schema describing the expected output.
    ///
    /// Mirrors `RunnableWithMessageHistory.get_output_schema()` from Python.
    pub fn get_output_schema(&self) -> Value {
        serde_json::json!({
            "title": "RunnableWithChatHistoryOutput",
            "type": "array",
            "items": { "type": "object" }
        })
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
        input: Vec<BaseMessage>,
        config: Option<RunnableConfig>,
    ) -> Result<Vec<BaseMessage>> {
        let config = config.unwrap_or_default();
        let history = self.resolve_history(&config)?;

        // --- enter history: load existing messages ---
        let historic_messages = {
            let guard = history
                .lock()
                .map_err(|e| Error::Other(format!("history lock poisoned: {e}")))?;
            guard.messages()
        };

        // --- build the augmented input: prepend history ---
        let mut augmented_input = historic_messages.clone();
        augmented_input.extend(input.iter().cloned());

        // --- invoke the inner runnable ---
        let output = self.runnable.invoke(augmented_input, Some(&config))?;

        // --- exit history: save new messages ---
        {
            let mut guard = history
                .lock()
                .map_err(|e| Error::Other(format!("history lock poisoned: {e}")))?;
            let mut to_save = input;
            to_save.extend(output.clone());
            guard.add_messages(&to_save);
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

        Ok((self.get_session_history)(&params))
    }
}
