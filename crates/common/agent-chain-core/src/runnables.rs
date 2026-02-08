//! Runnables module for agent-chain-core.
//!
//! This module provides the core Runnable abstraction and implementations,
//! mirroring `langchain_core.runnables`.

pub mod base;
pub mod branch;
pub mod config;
pub mod configurable;
pub mod fallbacks;
pub mod graph;
pub mod graph_mermaid;
pub mod passthrough;
pub mod retry;
pub mod router;
pub mod schema;
pub mod utils;

// Re-export commonly used types
pub use base::{
    DynRunnable, Runnable, RunnableBinding, RunnableEach, RunnableLambda, RunnableParallel,
    RunnableSequence, RunnableSerializable, coerce_to_runnable, pipe, runnable_lambda, to_dyn,
};
pub use branch::{RunnableBranch, RunnableBranchBuilder};
pub use config::{
    AsyncVariableArgsFn, ConfigOrList, RunnableConfig, VariableArgsFn,
    acall_func_with_variable_args, call_func_with_variable_args, ensure_config, get_config_list,
    merge_configs, patch_config,
};
pub use configurable::{
    Alternative, ConfigurableRunnable, DynamicRunnable, Reconfigurable,
    RunnableConfigurableAlternatives, RunnableConfigurableFields, make_options_spec_multi,
    make_options_spec_single, prefix_config_spec,
};
pub use fallbacks::{
    ExceptionInserter, FallbackErrorPredicate, RunnableWithFallbacks, RunnableWithFallbacksExt,
};
pub use graph::{CurveStyle, Edge, Graph, MermaidDrawMethod, MermaidOptions, Node, NodeStyles};
pub use graph_mermaid::to_safe_id;
pub use passthrough::{
    PickKeys, RunnableAssign, RunnableAssignBuilder, RunnablePassthrough, RunnablePick,
    graph_passthrough,
};
pub use retry::{
    ExponentialJitterParams, RetryCallState, RetryErrorPredicate, RunnableRetry,
    RunnableRetryConfig, RunnableRetryExt,
};
pub use router::{DynRouterRunnable, RouterInput, RouterRunnable};
pub use schema::{
    BaseStreamEvent, CUSTOM_EVENT_TYPE, CustomStreamEvent, EventData, StandardStreamEvent,
    StreamEvent,
};
pub use utils::{
    AddableDict, AnyConfigurableField, ConfigurableField, ConfigurableFieldMultiOption,
    ConfigurableFieldSingleOption, ConfigurableFieldSpec, RootEventFilter, aadd, add,
    gather_with_concurrency, get_unique_config_specs, indent_lines_after_first,
};
