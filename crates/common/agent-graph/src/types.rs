//! Types for LangGraph workflows.
//!
//! This module provides configuration types similar to Python's langgraph.types.

use std::time::Duration;

/// Configuration for retrying nodes.
///
/// # Example
///
/// ```
/// use agent_graph::types::RetryPolicy;
/// use std::time::Duration;
///
/// let policy = RetryPolicy::default()
///     .with_max_attempts(5)
///     .with_initial_interval(Duration::from_millis(500));
/// ```
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// Amount of time that must elapse before the first retry occurs.
    pub initial_interval: Duration,
    /// Multiplier by which the interval increases after each retry.
    pub backoff_factor: f64,
    /// Maximum amount of time that may elapse between retries.
    pub max_interval: Duration,
    /// Maximum number of attempts to make before giving up, including the first.
    pub max_attempts: u32,
    /// Whether to add random jitter to the interval between retries.
    pub jitter: bool,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            initial_interval: Duration::from_millis(500),
            backoff_factor: 2.0,
            max_interval: Duration::from_secs(128),
            max_attempts: 3,
            jitter: true,
        }
    }
}

impl RetryPolicy {
    /// Create a new retry policy with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the initial interval.
    pub fn with_initial_interval(mut self, interval: Duration) -> Self {
        self.initial_interval = interval;
        self
    }

    /// Set the backoff factor.
    pub fn with_backoff_factor(mut self, factor: f64) -> Self {
        self.backoff_factor = factor;
        self
    }

    /// Set the maximum interval.
    pub fn with_max_interval(mut self, interval: Duration) -> Self {
        self.max_interval = interval;
        self
    }

    /// Set the maximum number of attempts.
    pub fn with_max_attempts(mut self, attempts: u32) -> Self {
        self.max_attempts = attempts;
        self
    }

    /// Set whether to use jitter.
    pub fn with_jitter(mut self, jitter: bool) -> Self {
        self.jitter = jitter;
        self
    }

    /// Calculate the delay for a given attempt number.
    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        if attempt == 0 {
            return Duration::ZERO;
        }

        let base_delay =
            self.initial_interval.as_secs_f64() * self.backoff_factor.powi((attempt - 1) as i32);
        let delay = base_delay.min(self.max_interval.as_secs_f64());

        let final_delay = if self.jitter {
            let jitter_factor = 0.5 + rand_jitter() * 0.5;
            delay * jitter_factor
        } else {
            delay
        };

        Duration::from_secs_f64(final_delay)
    }
}

/// Simple pseudo-random jitter (0.0 to 1.0).
fn rand_jitter() -> f64 {
    use std::time::SystemTime;
    let nanos = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    (nanos % 1000) as f64 / 1000.0
}

/// Configuration for caching nodes.
///
/// # Example
///
/// ```
/// use agent_graph::types::CachePolicy;
/// use std::time::Duration;
///
/// let policy = CachePolicy::new()
///     .with_ttl(Duration::from_secs(3600));
/// ```
#[derive(Debug, Clone, Default)]
pub struct CachePolicy {
    /// Time to live for the cache entry. If `None`, the entry never expires.
    pub ttl: Option<Duration>,
}

impl CachePolicy {
    /// Create a new cache policy with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the time to live.
    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.ttl = Some(ttl);
        self
    }

    /// Set the TTL to never expire.
    pub fn without_ttl(mut self) -> Self {
        self.ttl = None;
        self
    }
}

/// A message or packet to send to a specific node in the graph.
///
/// The `Send` type is used within a `StateGraph`'s conditional edges to
/// dynamically invoke a node with a custom state at the next step.
///
/// # Example
///
/// ```ignore
/// use agent_graph::types::Send;
///
/// let sends = vec![
///     Send::new("generate_joke", "cats"),
///     Send::new("generate_joke", "dogs"),
/// ];
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Send<T> {
    /// The name of the target node to send the message to.
    pub node: String,
    /// The state or message to send to the target node.
    pub arg: T,
}

impl<T> Send<T> {
    /// Create a new Send message.
    pub fn new(node: impl Into<String>, arg: T) -> Self {
        Self {
            node: node.into(),
            arg,
        }
    }
}

/// One or more commands to update the graph's state and send messages to nodes.
///
/// # Example
///
/// ```ignore
/// use agent_graph::types::Command;
///
/// let cmd = Command::new()
///     .with_update(my_update)
///     .with_goto("next_node");
/// ```
#[derive(Debug, Clone)]
pub struct Command<U, G = String> {
    /// Update to apply to the graph's state.
    pub update: Option<U>,
    /// Value to resume execution with.
    pub resume: Option<serde_json::Value>,
    /// Node(s) to navigate to next.
    pub goto: Vec<G>,
}

impl<U, G> Default for Command<U, G> {
    fn default() -> Self {
        Self {
            update: None,
            resume: None,
            goto: Vec::new(),
        }
    }
}

impl<U, G> Command<U, G> {
    /// Create a new empty command.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the update for the command.
    pub fn with_update(mut self, update: U) -> Self {
        self.update = Some(update);
        self
    }

    /// Set the resume value for the command.
    pub fn with_resume(mut self, resume: serde_json::Value) -> Self {
        self.resume = Some(resume);
        self
    }

    /// Add a goto target.
    pub fn with_goto(mut self, target: G) -> Self {
        self.goto.push(target);
        self
    }

    /// Add multiple goto targets.
    pub fn with_gotos(mut self, targets: impl IntoIterator<Item = G>) -> Self {
        self.goto.extend(targets);
        self
    }
}

/// Information about an interrupt that occurred in a node.
#[derive(Debug, Clone)]
pub struct Interrupt {
    /// The value associated with the interrupt.
    pub value: serde_json::Value,
    /// The ID of the interrupt.
    pub id: String,
}

impl Interrupt {
    /// Create a new interrupt.
    pub fn new(value: serde_json::Value, id: impl Into<String>) -> Self {
        Self {
            value,
            id: id.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retry_policy_default() {
        let policy = RetryPolicy::default();
        assert_eq!(policy.initial_interval, Duration::from_millis(500));
        assert_eq!(policy.backoff_factor, 2.0);
        assert_eq!(policy.max_interval, Duration::from_secs(128));
        assert_eq!(policy.max_attempts, 3);
        assert!(policy.jitter);
    }

    #[test]
    fn test_retry_policy_builder() {
        let policy = RetryPolicy::new()
            .with_initial_interval(Duration::from_secs(1))
            .with_backoff_factor(3.0)
            .with_max_attempts(5)
            .with_jitter(false);

        assert_eq!(policy.initial_interval, Duration::from_secs(1));
        assert_eq!(policy.backoff_factor, 3.0);
        assert_eq!(policy.max_attempts, 5);
        assert!(!policy.jitter);
    }

    #[test]
    fn test_cache_policy() {
        let policy = CachePolicy::new().with_ttl(Duration::from_secs(3600));
        assert_eq!(policy.ttl, Some(Duration::from_secs(3600)));

        let policy2 = CachePolicy::default();
        assert_eq!(policy2.ttl, None);
    }

    #[test]
    fn test_send() {
        let send = Send::new("my_node", 42);
        assert_eq!(send.node, "my_node");
        assert_eq!(send.arg, 42);
    }

    #[test]
    fn test_command() {
        let cmd: Command<i32, String> = Command::new()
            .with_update(42)
            .with_goto("node_a".to_string())
            .with_goto("node_b".to_string());

        assert_eq!(cmd.update, Some(42));
        assert_eq!(cmd.goto, vec!["node_a", "node_b"]);
    }
}
