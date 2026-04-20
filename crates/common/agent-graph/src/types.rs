//! Core types shared across the agent-graph API surface.
//!
//! These are the plain data types that nodes, the Pregel loop, and
//! user-facing callers exchange. Every type here is `Serialize` +
//! `Deserialize` so it can survive a checkpoint round-trip and is `Clone`
//! because Pregel replays values across super-steps.
//!
//! The module mirrors `langgraph/libs/langgraph/langgraph/types.py`, with
//! two deliberate deviations:
//!
//! * Callable fields (`RetryPolicy.retry_on`, `CachePolicy.key_func`) become
//!   enums instead of function pointers. Closures don't round-trip through
//!   a checkpoint, and Phase 2's surface area is designed to survive that
//!   round-trip. Richer variants can be added later without a breaking
//!   change.
//! * `PregelExecutableTask` is intentionally omitted — it owns a `Runnable`
//!   and lives inside the Pregel loop. It arrives in Phase 4 when that
//!   infrastructure is in place.

use agent_graph_checkpoint::CheckpointMetadata;
use bon::Builder;
use serde::{Deserialize, Serialize};
use xxhash_rust::xxh3::xxh3_128;

use crate::channels::Value;
use crate::config::RunnableConfig;
use crate::constants::internal::OVERWRITE;

// ---------------------------------------------------------------------------
// Overwrite
// ---------------------------------------------------------------------------

/// Bypass a reducer and write the wrapped value directly to a
/// [`crate::channels::BinaryOperatorAggregate`] channel.
///
/// Mirrors `langgraph.types.Overwrite`. Receiving multiple `Overwrite`
/// values for the same channel in a single super-step raises
/// [`crate::errors::Error::InvalidUpdate`]. Serialises to
/// `{"__overwrite__": value}` so Rust- and Python-produced payloads are
/// on-the-wire compatible.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Overwrite {
    #[serde(rename = "__overwrite__")]
    pub value: Value,
}

impl Overwrite {
    pub fn new(value: Value) -> Self {
        Self { value }
    }

    /// Detect the dict-form overwrite marker used on the Python side.
    ///
    /// Python encodes an overwrite either with an `Overwrite` instance or a
    /// dict `{"__overwrite__": value}`. Rust callers can construct an
    /// [`Overwrite`] directly, but incoming JSON values may carry the dict
    /// form (e.g. from a Python-written checkpoint), so the channel layer
    /// must recognise it.
    pub(crate) fn from_value(value: &Value) -> Option<Value> {
        let object = value.as_object()?;
        if object.len() == 1
            && let Some(inner) = object.get(OVERWRITE)
        {
            return Some(inner.clone());
        }
        None
    }
}

// ---------------------------------------------------------------------------
// StreamMode / Durability
// ---------------------------------------------------------------------------

/// Shapes of data a graph emits from `stream`.
///
/// Mirrors Python's `Literal["values", "updates", ...]` union. Serialises as
/// the lowercase mode name to stay on-the-wire compatible with the Python
/// client vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StreamMode {
    /// Emit the full state after each super-step.
    Values,
    /// Emit only the deltas returned by each node.
    Updates,
    /// Emit an event when a checkpoint is created.
    Checkpoints,
    /// Emit task-start / task-finish events with results.
    Tasks,
    /// Emit `Checkpoints` + `Tasks` together.
    Debug,
    /// Emit LLM messages token-by-token.
    Messages,
    /// Emit caller-provided payloads written via the stream writer.
    Custom,
}

/// When to persist state to the checkpointer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Durability {
    /// Flush before starting the next super-step.
    Sync,
    /// Flush concurrently with the next super-step.
    Async,
    /// Flush only when the graph terminates.
    Exit,
}

// ---------------------------------------------------------------------------
// RetryPolicy
// ---------------------------------------------------------------------------

/// Which errors a node's retry loop should treat as transient.
///
/// Python exposes `type[Exception] | Sequence[type[Exception]] |
/// Callable[[Exception], bool]` here; we use an enum because closures can't
/// be serialised into a checkpoint. For now the policy is coarse-grained —
/// once the retry layer lands we can add a richer `Custom(Arc<dyn Fn>)`
/// variant without breaking callers of the default.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetryPredicate {
    /// Retry on transient errors (timeouts, rate limits, 5xx). Mirrors
    /// Python's `default_retry_on`.
    #[default]
    TransientErrors,
    /// Retry on every error.
    All,
    /// Never retry — useful when a node must remain idempotent at the
    /// application layer.
    None,
}

/// Configuration for retrying a node after a failure.
///
/// Defaults mirror `langgraph.types.RetryPolicy` exactly: `initial_interval
/// = 0.5s`, `backoff_factor = 2.0`, `max_interval = 128s`, `max_attempts =
/// 3`, `jitter = true`, `retry_on = TransientErrors`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Builder)]
pub struct RetryPolicy {
    /// Seconds before the first retry.
    #[builder(default = 0.5)]
    pub initial_interval: f64,
    /// Multiplier applied to the interval after each retry.
    #[builder(default = 2.0)]
    pub backoff_factor: f64,
    /// Upper bound on the retry interval in seconds.
    #[builder(default = 128.0)]
    pub max_interval: f64,
    /// Maximum attempts, including the first.
    #[builder(default = 3)]
    pub max_attempts: u32,
    /// Whether to jitter the retry interval.
    #[builder(default = true)]
    pub jitter: bool,
    /// Error classifier that decides whether to retry.
    #[builder(default)]
    pub retry_on: RetryPredicate,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self::builder().build()
    }
}

// ---------------------------------------------------------------------------
// CachePolicy
// ---------------------------------------------------------------------------

/// How a task's cache key is derived from its input.
///
/// Python accepts an arbitrary callable; we surface a coarse enum for the
/// same reason as [`RetryPredicate`]. `Hash` — the default — is equivalent
/// to `langgraph._internal._cache.default_cache_key`, which hashes the
/// serialised input.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CacheKeyStrategy {
    /// Hash the (serialised) input to produce a cache key.
    #[default]
    Hash,
}

/// Configuration for caching a node's output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct CachePolicy {
    /// Time-to-live in seconds. `None` means entries never expire.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ttl: Option<u64>,
    /// How to derive the cache key.
    #[serde(default)]
    pub key_strategy: CacheKeyStrategy,
}

impl CachePolicy {
    pub const fn new() -> Self {
        Self {
            ttl: None,
            key_strategy: CacheKeyStrategy::Hash,
        }
    }

    pub const fn with_ttl(ttl: u64) -> Self {
        Self {
            ttl: Some(ttl),
            key_strategy: CacheKeyStrategy::Hash,
        }
    }
}

// ---------------------------------------------------------------------------
// Send
// ---------------------------------------------------------------------------

/// Dynamic invocation of a specific node with a bespoke argument.
///
/// Emitted from conditional edges and `Command.goto` to implement map-reduce
/// style fan-out.
///
/// Unlike Python, `Send` is not `Hash`-able — its `arg` is a
/// [`serde_json::Value`] which may contain `f64` (not `Eq`) or
/// key-dependent-ordered maps. Sends are routed by reference rather than
/// hashed into a set anywhere in the Rust port.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Send {
    pub node: String,
    pub arg: Value,
}

impl Send {
    pub fn new(node: impl Into<String>, arg: Value) -> Self {
        Self {
            node: node.into(),
            arg,
        }
    }
}

// ---------------------------------------------------------------------------
// Interrupt
// ---------------------------------------------------------------------------

/// Placeholder id used when an [`Interrupt`] is constructed without a
/// namespace — overwritten by the Pregel loop before the value reaches a
/// caller. Matches Python's `_DEFAULT_INTERRUPT_ID`.
pub const PLACEHOLDER_INTERRUPT_ID: &str = "placeholder-id";

/// Human-in-the-loop pause surfaced to the client.
///
/// Raised internally by `interrupt()` (Phase 4) and by the `Command.resume`
/// machinery. The `id` is typically derived from the node's checkpoint
/// namespace so that resumes can be matched to the right pause on replay.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Interrupt {
    pub value: Value,
    pub id: String,
}

impl Interrupt {
    /// Build an interrupt with the placeholder id. The Pregel loop replaces
    /// this with a namespace-derived id before the value leaves the node.
    pub fn new(value: Value) -> Self {
        Self {
            value,
            id: PLACEHOLDER_INTERRUPT_ID.to_owned(),
        }
    }

    /// Build an interrupt with an explicit id.
    pub fn with_id(value: Value, id: impl Into<String>) -> Self {
        Self {
            value,
            id: id.into(),
        }
    }

    /// Derive the id from a checkpoint namespace.
    ///
    /// Matches Python's `Interrupt.from_ns`, which hashes the namespace
    /// string with xxh3-128 and formats the result as a 32-char lowercase
    /// hex digest.
    pub fn from_ns(value: Value, ns: &str) -> Self {
        Self {
            value,
            id: interrupt_id_from_ns(ns),
        }
    }
}

/// Compute the interrupt id for a given checkpoint namespace.
fn interrupt_id_from_ns(ns: &str) -> String {
    format!("{:032x}", xxh3_128(ns.as_bytes()))
}

// ---------------------------------------------------------------------------
// Command
// ---------------------------------------------------------------------------

/// Scope token placed in [`Command::graph`] to target the nearest parent
/// graph rather than the current one.
pub const PARENT: &str = "__parent__";

/// A single destination for a [`Command::goto`] navigation.
///
/// Serialises as either a bare node name string or a `Send` object,
/// matching Python's `Send | N` union where `N` is the node name type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum GoTo {
    /// Jump to a node by name, passing the current state through unchanged.
    Node(String),
    /// Invoke a node with a specific argument.
    Send(Send),
}

impl From<Send> for GoTo {
    fn from(send: Send) -> Self {
        GoTo::Send(send)
    }
}

impl From<String> for GoTo {
    fn from(node: String) -> Self {
        GoTo::Node(node)
    }
}

impl From<&str> for GoTo {
    fn from(node: &str) -> Self {
        GoTo::Node(node.to_owned())
    }
}

/// One or more instructions a node returns to steer the graph.
///
/// * `update` mutates state — merged via the same reducer rules node return
///   values go through.
/// * `resume` supplies a value for the current [`Interrupt`].
/// * `goto` queues the next node(s) to run; an empty vec preserves the
///   default static edges.
/// * `graph = Some(PARENT)` targets the enclosing graph, used for bubbling
///   commands out of a subgraph.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, Builder)]
pub struct Command {
    /// Which graph this command targets: `None` = current, `Some(PARENT)` =
    /// the enclosing parent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[builder(into)]
    pub graph: Option<String>,

    /// State update to apply. The exact shape depends on the graph's state
    /// schema.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub update: Option<Value>,

    /// Resume value for the current interrupt. A map is matched by id; any
    /// other value is consumed by the next outstanding interrupt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resume: Option<Value>,

    /// Next destinations. An empty vec falls back to the graph's static
    /// edges.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[builder(default)]
    pub goto: Vec<GoTo>,
}

impl Command {
    /// Shorthand for a Command that only navigates.
    pub fn goto_node(name: impl Into<String>) -> Self {
        Self {
            goto: vec![GoTo::Node(name.into())],
            ..Self::default()
        }
    }

    /// Shorthand for a Command that only resumes.
    pub fn resume(value: Value) -> Self {
        Self {
            resume: Some(value),
            ..Self::default()
        }
    }

    /// Shorthand for a Command that only applies an update.
    pub fn update(value: Value) -> Self {
        Self {
            update: Some(value),
            ..Self::default()
        }
    }
}

// ---------------------------------------------------------------------------
// PregelTask / StateSnapshot
// ---------------------------------------------------------------------------

/// Breadcrumb describing how to navigate nested subgraphs to reach a task.
///
/// Python uses `tuple[str | int | tuple, ...]`; the Rust representation is
/// structurally identical with an untagged enum so wire compatibility is
/// preserved.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TaskPath {
    Str(String),
    Int(i64),
    Nested(Vec<TaskPath>),
}

impl From<&str> for TaskPath {
    fn from(value: &str) -> Self {
        TaskPath::Str(value.to_owned())
    }
}

impl From<String> for TaskPath {
    fn from(value: String) -> Self {
        TaskPath::Str(value)
    }
}

impl From<i64> for TaskPath {
    fn from(value: i64) -> Self {
        TaskPath::Int(value)
    }
}

/// Current state of a task as returned by `get_state`.
///
/// Python types this as `None | RunnableConfig | StateSnapshot`. The Rust
/// enum carries the same three cases, with `None` being
/// `Option::<TaskState>::None`.
///
/// `PartialEq` is not derived because [`RunnableConfig`] holds non-comparable
/// callback trait objects; test round-trips compare via `serde_json::Value`
/// instead.
///
/// Both payloads are boxed so the enum itself stays pointer-sized — callers
/// can embed it directly (e.g. `Option<TaskState>`) without paying for the
/// larger of the two variants on every task.
///
/// Variant order matters: `RunnableConfig` deserialises from any JSON object
/// (every field has a default), so `Snapshot` — which demands `values`,
/// `next`, and `config` — must be tried first for the untagged discrimination
/// to be unambiguous.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TaskState {
    /// Fully materialised snapshot of the subgraph's state.
    Snapshot(Box<StateSnapshot>),
    /// Subgraph task referenced by configuration alone; the full snapshot
    /// has not been fetched yet.
    Config(Box<RunnableConfig>),
}

/// Metadata for a task scheduled by the Pregel planner.
///
/// Companion to [`StateSnapshot`]: a snapshot lists every task that will run
/// in its super-step. The `error` field carries a display string rather
/// than a live [`crate::errors::Error`] so the value can survive a
/// checkpoint round-trip — the original type information is lost on
/// deserialisation regardless, so folding it into a string up front keeps
/// the field honest.
///
/// `PartialEq` is not derived because the optional `state` transitively
/// references [`RunnableConfig`]; see [`TaskState`] for details.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PregelTask {
    pub id: String,
    pub name: String,
    pub path: Vec<TaskPath>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub interrupts: Vec<Interrupt>,

    /// Subgraph state for this task, if any. [`TaskState`] carries its own
    /// indirection, so no outer [`Box`] is needed here.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<TaskState>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
}

/// Snapshot of the graph at the start of a super-step.
///
/// Returned by `get_state`, `get_state_history`, and (as an event payload)
/// by `stream(mode=checkpoints)`.
///
/// `PartialEq` is not derived because the snapshot holds a
/// [`RunnableConfig`], whose callback trait objects cannot be compared;
/// round-trip tests compare via `serde_json::Value`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSnapshot {
    /// Current channel values projected into the graph's output schema.
    pub values: Value,

    /// Names of the nodes that will execute in the next step.
    pub next: Vec<String>,

    /// Config used to fetch this snapshot.
    pub config: RunnableConfig,

    /// Checkpoint metadata, if the snapshot came from a checkpointer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<CheckpointMetadata>,

    /// ISO 8601 timestamp of snapshot creation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,

    /// Parent checkpoint's config, if any. Boxed to avoid bloating
    /// `StateSnapshot` with a second full [`RunnableConfig`] when the
    /// common case is `None`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_config: Option<Box<RunnableConfig>>,

    /// Tasks scheduled for the next step. May be empty for terminal
    /// snapshots.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tasks: Vec<PregelTask>,

    /// Interrupts raised in this step awaiting a resume.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub interrupts: Vec<Interrupt>,
}
