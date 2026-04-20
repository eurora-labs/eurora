//! Channel primitives for the Pregel engine.
//!
//! Channels are the communication medium between Pregel actors. Each channel
//! holds a piece of state, accepts a sequence of updates per super-step, and
//! produces a value that downstream tasks can read. Implementations mirror
//! `langgraph/channels/` in the Python sources.
//!
//! Values flowing through channels are represented as [`serde_json::Value`]
//! — channels are held by the Pregel loop in a `HashMap<String, Box<dyn
//! BaseChannel>>` and must therefore be object-safe. The static type safety
//! that Python's `Generic[Value, Update, Checkpoint]` expresses will be
//! re-introduced at the `StateGraph` DSL layer (Phase 6) via the
//! `#[derive(State)]` macro.

use std::fmt::Debug;

use crate::errors::Result;

pub mod any_value;
pub mod binop;
pub mod ephemeral_value;
pub mod last_value;
pub mod named_barrier_value;
pub mod topic;
pub mod untracked_value;

pub use any_value::AnyValue;
pub use binop::BinaryOperatorAggregate;
pub use ephemeral_value::EphemeralValue;
pub use last_value::{LastValue, LastValueAfterFinish};
pub use named_barrier_value::{NamedBarrierValue, NamedBarrierValueAfterFinish};
pub use topic::Topic;
pub use untracked_value::UntrackedValue;

/// The dynamic value type carried by channels.
pub type Value = serde_json::Value;

/// Contract implemented by every Pregel channel.
///
/// The Python base class is parameterised over `(Value, Update, Checkpoint)`;
/// we erase all three to [`Value`] so implementations are object-safe. See
/// the module-level docs for the rationale.
pub trait BaseChannel: Debug + Send + Sync {
    /// The channel's name within the Pregel graph.
    fn key(&self) -> &str;

    /// Associate this channel with a name in the graph.
    ///
    /// Called once during graph compilation. Channels are constructed without
    /// a key and receive one when they are registered.
    fn set_key(&mut self, key: String);

    /// Apply a batch of updates collected during a single super-step.
    ///
    /// Returns `true` if the channel's state changed (the Pregel planner uses
    /// this signal to bump the channel's version). The order of `values` is
    /// unspecified. An empty slice is a legal call and some channels use it
    /// to transition state (for example, [`EphemeralValue`] clears itself).
    fn update(&mut self, values: Vec<Value>) -> Result<bool>;

    /// Read the channel's current value.
    ///
    /// Returns [`crate::errors::Error::EmptyChannel`] if the channel has not
    /// yet received a value (or has cleared itself since).
    fn get(&self) -> Result<Value>;

    /// Cheap availability check. The default falls back to [`Self::get`];
    /// override when a channel can answer without materialising the value.
    fn is_available(&self) -> bool {
        self.get().is_ok()
    }

    /// Serialise the channel's state for persistence.
    ///
    /// `None` means the channel has nothing worth persisting — either it has
    /// never received a value, or (as with [`UntrackedValue`]) it opts out of
    /// checkpointing entirely. Returning `Some` implies the value will be
    /// passed back verbatim to [`Self::from_checkpoint`] on restore.
    fn checkpoint(&self) -> Option<Value>;

    /// Reconstruct a channel of the same shape, optionally seeded from a
    /// previously captured checkpoint.
    ///
    /// Must preserve configuration (e.g. `guard`, `accumulate`, `names`) but
    /// reset any derived state not present in the checkpoint. Takes `&self`
    /// because the caller needs a template holding the immutable
    /// configuration — matching Python's `BaseChannel.from_checkpoint`
    /// semantics.
    #[allow(clippy::wrong_self_convention)]
    fn from_checkpoint(&self, checkpoint: Option<Value>) -> Result<Box<dyn BaseChannel>>;

    /// Notify the channel that a subscribed task consumed its value.
    ///
    /// Default is a no-op. Channels that hand out values only once (for
    /// example the `*AfterFinish` variants) override this to clear state.
    /// Returns `true` if the channel mutated itself.
    fn consume(&mut self) -> bool {
        false
    }

    /// Notify the channel that the Pregel run is finishing.
    ///
    /// Default is a no-op. Returns `true` if the channel mutated itself.
    fn finish(&mut self) -> bool {
        false
    }

    /// Produce an owned clone behind a trait object.
    ///
    /// Needed because `Clone` is not object-safe but the Pregel loop holds
    /// channels as `Box<dyn BaseChannel>` and occasionally needs to fork
    /// state (for example, when running subgraphs or during debug streaming).
    fn clone_channel(&self) -> Box<dyn BaseChannel>;
}
