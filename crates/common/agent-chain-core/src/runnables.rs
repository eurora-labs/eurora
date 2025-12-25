//! Runnables module for agent-chain-core.
//!
//! This module provides the core Runnable abstraction and implementations,
//! mirroring `langchain_core.runnables`.

pub mod base;
pub mod config;

// Re-export commonly used types
pub use base::{
    AddableDict, DynRunnable, Runnable, RunnableBinding, RunnableEach, RunnableLambda,
    RunnableParallel, RunnablePassthrough, RunnableRetry, RunnableSequence, RunnableSerializable,
    coerce_to_runnable, pipe, runnable_lambda, to_dyn,
};
pub use config::{
    ConfigOrList, RunnableConfig, ensure_config, get_config_list, merge_configs, patch_config,
};
