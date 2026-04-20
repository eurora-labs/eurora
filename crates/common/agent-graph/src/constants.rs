//! Constants used throughout agent-graph.
//!
//! Public constants are exposed directly. Internals (Pregel reserved keys,
//! configurable keys) live in the `internal` submodule and mirror Python's
//! `langgraph._internal._constants`.

/// The first (virtual) node in a graph.
pub const START: &str = "__start__";

/// The last (virtual) node in a graph.
pub const END: &str = "__end__";

/// Tag to disable streaming for a node.
pub const TAG_NOSTREAM: &str = "nostream";

/// Tag to hide a node/edge from certain tracing/streaming environments.
pub const TAG_HIDDEN: &str = "langsmith:hidden";

#[allow(dead_code)] // consumers land in later phases
pub(crate) mod internal {
    // --- Reserved write keys ---

    /// Values passed as input to the graph.
    pub const INPUT: &str = "__input__";
    /// Dynamic interrupts raised by nodes.
    pub const INTERRUPT: &str = "__interrupt__";
    /// Values passed to resume a node after an interrupt.
    pub const RESUME: &str = "__resume__";
    /// Errors raised by nodes.
    pub const ERROR: &str = "__error__";
    /// Marker indicating a node did not write anything.
    pub const NO_WRITES: &str = "__no_writes__";
    /// `Send` objects returned by nodes/edges, corresponds to [`PUSH`].
    pub const TASKS: &str = "__pregel_tasks";
    /// Writes of a task where the return value is recorded directly.
    pub const RETURN: &str = "__return__";
    /// Implicit branch that handles each node's `Control` values.
    pub const PREVIOUS: &str = "__previous__";

    // --- Reserved cache namespaces ---

    /// Cache namespace for node writes.
    pub const CACHE_NS_WRITES: &str = "__pregel_ns_writes";

    // --- Reserved `config.configurable` keys ---

    /// `write` function that accepts writes to state/edges/reserved keys.
    pub const CONFIG_KEY_SEND: &str = "__pregel_send";
    /// `read` function that returns a copy of the current state.
    pub const CONFIG_KEY_READ: &str = "__pregel_read";
    /// `call` function that accepts a node/func, args and returns a future.
    pub const CONFIG_KEY_CALL: &str = "__pregel_call";
    /// `BaseCheckpointSaver` passed from parent graph to child graphs.
    pub const CONFIG_KEY_CHECKPOINTER: &str = "__pregel_checkpointer";
    /// `StreamProtocol` passed from parent graph to child graphs.
    pub const CONFIG_KEY_STREAM: &str = "__pregel_stream";
    /// `BaseCache` made available to subgraphs.
    pub const CONFIG_KEY_CACHE: &str = "__pregel_cache";
    /// Whether subgraphs should resume from a previous checkpoint.
    pub const CONFIG_KEY_RESUMING: &str = "__pregel_resuming";
    /// Task ID for the current task.
    pub const CONFIG_KEY_TASK_ID: &str = "__pregel_task_id";
    /// Thread ID for the current invocation.
    pub const CONFIG_KEY_THREAD_ID: &str = "thread_id";
    /// Mapping of `checkpoint_ns` → `checkpoint_id` for parent graphs.
    pub const CONFIG_KEY_CHECKPOINT_MAP: &str = "checkpoint_map";
    /// Current `checkpoint_id`, if any.
    pub const CONFIG_KEY_CHECKPOINT_ID: &str = "checkpoint_id";
    /// Current `checkpoint_ns`; empty string for the root graph.
    pub const CONFIG_KEY_CHECKPOINT_NS: &str = "checkpoint_ns";
    /// Callback invoked when a node finishes.
    pub const CONFIG_KEY_NODE_FINISHED: &str = "__pregel_node_finished";
    /// Mutable dict for temporary storage scoped to the current task.
    pub const CONFIG_KEY_SCRATCHPAD: &str = "__pregel_scratchpad";
    /// Function receiving tasks from the runner, executing them and returning results.
    pub const CONFIG_KEY_RUNNER_SUBMIT: &str = "__pregel_runner_submit";
    /// Durability mode: one of `"sync"`, `"async"`, or `"exit"`.
    pub const CONFIG_KEY_DURABILITY: &str = "__pregel_durability";
    /// `Runtime` instance with context, store, stream writer, etc.
    pub const CONFIG_KEY_RUNTIME: &str = "__pregel_runtime";
    /// Mapping of task ns → resume value for resuming tasks.
    pub const CONFIG_KEY_RESUME_MAP: &str = "__pregel_resume_map";

    // --- Task kinds + namespace separators ---

    /// Push-style tasks — created by `Send` objects.
    pub const PUSH: &str = "__pregel_push";
    /// Pull-style tasks — triggered by edges.
    pub const PULL: &str = "__pregel_pull";
    /// `checkpoint_ns` level separator (e.g. `graph|subgraph|subsubgraph`).
    pub const NS_SEP: &str = "|";
    /// Within a `checkpoint_ns` level, separates the namespace from the task_id.
    pub const NS_END: &str = ":";
    /// Key for the configurable dict in `RunnableConfig`.
    pub const CONF: &str = "configurable";
    /// Task ID used for writes that are not associated with a task.
    pub const NULL_TASK_ID: &str = "00000000-0000-0000-0000-000000000000";
    /// Dict key for the overwrite value, used as `{"__overwrite__": value}`.
    pub const OVERWRITE: &str = "__overwrite__";

    /// Set of keys that must not be written to by user code.
    pub const RESERVED: &[&str] = &[
        super::TAG_HIDDEN,
        INPUT,
        INTERRUPT,
        RESUME,
        ERROR,
        NO_WRITES,
        CONFIG_KEY_SEND,
        CONFIG_KEY_READ,
        CONFIG_KEY_CHECKPOINTER,
        CONFIG_KEY_STREAM,
        CONFIG_KEY_CHECKPOINT_MAP,
        CONFIG_KEY_RESUMING,
        CONFIG_KEY_TASK_ID,
        CONFIG_KEY_CHECKPOINT_ID,
        CONFIG_KEY_CHECKPOINT_NS,
        CONFIG_KEY_RESUME_MAP,
        PUSH,
        PULL,
        NS_SEP,
        NS_END,
        CONF,
    ];
}

#[cfg(test)]
mod tests {
    use super::internal::RESERVED;

    #[test]
    fn reserved_has_no_duplicates() {
        let mut seen = std::collections::HashSet::new();
        for key in RESERVED {
            assert!(seen.insert(key), "duplicate reserved key: {key}");
        }
    }
}
