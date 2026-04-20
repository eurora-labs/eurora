//! Error hierarchy for agent-graph.
//!
//! Python's `GraphBubbleUp` / `GraphInterrupt` / `ParentCommand` are
//! intentionally **not** modeled as error variants: in Python they abuse the
//! exception machinery for control flow, whereas idiomatic Rust expresses
//! control flow with return types. Those signals will live on a
//! `TaskOutcome` enum introduced in Phase 4.

use std::fmt;

use thiserror::Error;

/// Convenience alias for results returned from agent-graph APIs.
pub type Result<T> = std::result::Result<T, Error>;

/// Structured error codes matching the identifiers in the LangGraph
/// troubleshooting docs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorCode {
    GraphRecursionLimit,
    InvalidConcurrentGraphUpdate,
    InvalidGraphNodeReturnValue,
    MultipleSubgraphs,
    InvalidChatHistory,
}

impl ErrorCode {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::GraphRecursionLimit => "GRAPH_RECURSION_LIMIT",
            Self::InvalidConcurrentGraphUpdate => "INVALID_CONCURRENT_GRAPH_UPDATE",
            Self::InvalidGraphNodeReturnValue => "INVALID_GRAPH_NODE_RETURN_VALUE",
            Self::MultipleSubgraphs => "MULTIPLE_SUBGRAPHS",
            Self::InvalidChatHistory => "INVALID_CHAT_HISTORY",
        }
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Format an error message with a troubleshooting URL suffix.
pub fn create_message(message: &str, code: ErrorCode) -> String {
    format!(
        "{message}\nFor troubleshooting, visit: \
         https://docs.langchain.com/oss/python/langgraph/errors/{code}"
    )
}

#[derive(Debug, Error)]
pub enum Error {
    /// Graph exhausted its maximum number of steps without completing.
    ///
    /// Corresponds to Python's `GraphRecursionError`. Raise the configured
    /// `recursion_limit` to allow more steps.
    #[error("Recursion limit of {limit} reached without completing the graph")]
    GraphRecursion { limit: usize },

    /// A channel was updated with an invalid set of updates.
    ///
    /// Corresponds to Python's `InvalidUpdateError`.
    #[error("{0}")]
    InvalidUpdate(String),

    /// A channel was read before any value was written to it.
    ///
    /// Corresponds to Python's `EmptyChannelError`.
    #[error("Channel `{0}` was read before it was written to")]
    EmptyChannel(String),

    /// Graph received an empty input.
    ///
    /// Corresponds to Python's `EmptyInputError`.
    #[error("Graph received empty input")]
    EmptyInput,

    /// Executor was unable to find a task (used in distributed mode).
    ///
    /// Corresponds to Python's `TaskNotFound`.
    #[error("Task `{0}` not found")]
    TaskNotFound(String),

    /// Catch-all for messages that do not yet have a dedicated variant.
    #[error("{0}")]
    Other(String),
}

#[cfg(test)]
mod tests {
    use super::{ErrorCode, create_message};

    #[test]
    fn error_code_as_str_matches_python() {
        assert_eq!(
            ErrorCode::GraphRecursionLimit.as_str(),
            "GRAPH_RECURSION_LIMIT"
        );
        assert_eq!(
            ErrorCode::InvalidConcurrentGraphUpdate.as_str(),
            "INVALID_CONCURRENT_GRAPH_UPDATE",
        );
    }

    #[test]
    fn create_message_includes_troubleshooting_url() {
        let msg = create_message("boom", ErrorCode::GraphRecursionLimit);
        assert!(msg.contains("boom"));
        assert!(msg.contains("GRAPH_RECURSION_LIMIT"));
        assert!(msg.contains("docs.langchain.com"));
    }
}
