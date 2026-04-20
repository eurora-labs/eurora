//! Re-exports `RunnableConfig` from `agent-chain-core` and adds the
//! LangGraph-specific reserved-key helpers.
//!
//! Python's `langgraph.config` behaves the same way: it re-exports
//! `RunnableConfig` from `langchain_core.runnables` and supplies helpers
//! (`get_store`, `get_stream_writer`, …) that read reserved keys out of the
//! config's `configurable` map. We do the same without redefining the
//! struct, so that the many existing `agent-chain-core` helpers (builder,
//! merging, patching, context propagation) remain available to graph
//! callers.

pub use agent_chain_core::runnables::config::{
    ConfigContextGuard, ConfigOrList, DEFAULT_RECURSION_LIMIT, RunnableConfig, ensure_config,
    get_child_runnable_config, set_config_context,
};

use serde_json::Value;

use crate::constants::internal::{
    CONFIG_KEY_CHECKPOINT_ID, CONFIG_KEY_CHECKPOINT_NS, CONFIG_KEY_THREAD_ID,
};

/// Read the thread id recorded under `configurable.thread_id`.
///
/// Returns `None` when the key is absent or not a string. The Pregel loop
/// rejects non-string thread ids elsewhere, so this helper deliberately
/// short-circuits instead of panicking on malformed input.
pub fn thread_id(config: &RunnableConfig) -> Option<&str> {
    configurable_str(config, CONFIG_KEY_THREAD_ID)
}

/// Read the checkpoint namespace recorded under `configurable.checkpoint_ns`.
///
/// An empty string is valid — it marks the root graph — and is returned as
/// `Some("")` to preserve that signal.
pub fn checkpoint_ns(config: &RunnableConfig) -> Option<&str> {
    configurable_str(config, CONFIG_KEY_CHECKPOINT_NS)
}

/// Read the checkpoint id recorded under `configurable.checkpoint_id`.
pub fn checkpoint_id(config: &RunnableConfig) -> Option<&str> {
    configurable_str(config, CONFIG_KEY_CHECKPOINT_ID)
}

fn configurable_str<'a>(config: &'a RunnableConfig, key: &str) -> Option<&'a str> {
    match config.configurable.get(key)? {
        Value::String(s) => Some(s.as_str()),
        _ => None,
    }
}
