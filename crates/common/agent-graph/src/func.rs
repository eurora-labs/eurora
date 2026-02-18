//! Functional API module for LangGraph workflows.
//!
//! This module provides the `task` and `entrypoint` builders
//! for defining LangGraph workflows using a functional approach.
//!
//! # Overview
//!
//! The functional API allows you to define workflows using simple async functions
//! decorated with `#[task]` and `#[entrypoint]`. Tasks are the building blocks
//! that can be called from within an entrypoint, and they return futures that
//! can be awaited or collected.
//!
//! # Example
//!
//! ```ignore
//! use agent_graph::func::{Task, Entrypoint};
//! use agent_graph::types::RetryPolicy;
//!
//! // Define a task
//! async fn process_data(input: String) -> String {
//!     input.to_uppercase()
//! }
//!
//! // Wrap it as a Task
//! let task = Task::new(process_data);
//!
//! // Define an entrypoint
//! let workflow = Entrypoint::new(|input: String| async move {
//!     process_data(input).await
//! });
//!
//! // Execute
//! let result = workflow.invoke("hello".to_string()).await;
//! ```

use std::collections::HashMap;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::Arc;

/// Type alias for a boxed future that is Send.
pub type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;

/// Type alias for an async function that takes Args and returns a BoxFuture<T>.
pub type AsyncFn<Args, T> = dyn Fn(Args) -> BoxFuture<T> + Send + Sync;

use futures::stream::{self, Stream};
use tokio::sync::oneshot;

use crate::checkpoint::InMemorySaver;
use crate::stream::{StreamChunk, StreamMode};
use crate::types::{CachePolicy, RetryPolicy};

pub use agent_graph_macros::{entrypoint, task};

/// A future that can be awaited or have its result retrieved synchronously.
///
/// This is similar to Python's `SyncAsyncFuture` - it wraps an async computation
/// and provides both sync and async ways to get the result.
pub struct TaskFuture<T> {
    receiver: oneshot::Receiver<T>,
}

impl<T> TaskFuture<T> {
    /// Create a new TaskFuture from a receiver.
    pub fn new(receiver: oneshot::Receiver<T>) -> Self {
        Self { receiver }
    }

    /// Block and wait for the result (for use in sync contexts).
    ///
    /// Note: This will block the current thread. Prefer using `.await` in async contexts.
    pub fn result(self) -> Result<T, TaskError> {
        self.receiver
            .blocking_recv()
            .map_err(|_| TaskError::Cancelled)
    }
}

impl<T> Future for TaskFuture<T> {
    type Output = Result<T, TaskError>;

    fn poll(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        Pin::new(&mut self.receiver)
            .poll(cx)
            .map(|r| r.map_err(|_| TaskError::Cancelled))
    }
}

/// Errors that can occur during task execution.
#[derive(Debug, Clone, thiserror::Error)]
pub enum TaskError {
    /// The task was cancelled.
    #[error("Task was cancelled")]
    Cancelled,
    /// The task failed after all retry attempts.
    #[error("Task failed after {attempts} attempts: {message}")]
    Failed { attempts: u32, message: String },
    /// An error occurred during execution.
    #[error("Task execution error: {0}")]
    Execution(String),
}

/// A wrapper for async functions that can be called as tasks.
///
/// Tasks are the building blocks of LangGraph workflows. They represent
/// individual units of work that can be executed and whose results can
/// be awaited.
///
/// # Example
///
/// ```ignore
/// use agent_graph::func::Task;
/// use agent_graph::types::RetryPolicy;
///
/// async fn my_task(input: i32) -> i32 {
///     input * 2
/// }
///
/// let task = Task::new(my_task)
///     .with_retry_policy(RetryPolicy::default())
///     .with_name("double");
///
/// // Call the task
/// let future = task.call(5);
/// let result = future.await?; // 10
/// ```
pub struct Task<F, Args, T>
where
    F: Fn(Args) -> Pin<Box<dyn Future<Output = T> + Send>> + Send + Sync + 'static,
    T: Send + 'static,
{
    func: Arc<F>,
    name: Option<String>,
    retry_policy: Option<RetryPolicy>,
    cache_policy: Option<CachePolicy>,
    _phantom: PhantomData<(Args, T)>,
}

impl<F, Args, T> Clone for Task<F, Args, T>
where
    F: Fn(Args) -> Pin<Box<dyn Future<Output = T> + Send>> + Send + Sync + 'static,
    T: Send + 'static,
{
    fn clone(&self) -> Self {
        Self {
            func: self.func.clone(),
            name: self.name.clone(),
            retry_policy: self.retry_policy.clone(),
            cache_policy: self.cache_policy.clone(),
            _phantom: PhantomData,
        }
    }
}

impl<F, Args, T> Task<F, Args, T>
where
    F: Fn(Args) -> Pin<Box<dyn Future<Output = T> + Send>> + Send + Sync + 'static,
    Args: Send + 'static,
    T: Send + 'static,
{
    /// Create a new task from an async function.
    pub fn new(func: F) -> Self {
        Self {
            func: Arc::new(func),
            name: None,
            retry_policy: None,
            cache_policy: None,
            _phantom: PhantomData,
        }
    }

    /// Set the name for this task.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the retry policy for this task.
    pub fn with_retry_policy(mut self, policy: RetryPolicy) -> Self {
        self.retry_policy = Some(policy);
        self
    }

    /// Set the cache policy for this task.
    pub fn with_cache_policy(mut self, policy: CachePolicy) -> Self {
        self.cache_policy = Some(policy);
        self
    }

    /// Get the name of this task.
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Call the task with the given arguments.
    ///
    /// Returns a `TaskFuture` that can be awaited or have its result retrieved.
    pub fn call(&self, args: Args) -> TaskFuture<T> {
        let func = self.func.clone();
        let _retry_policy = self.retry_policy.clone();

        let (sender, receiver) = oneshot::channel();

        tokio::spawn(async move {
            let result = (func)(args).await;
            let _ = sender.send(result);
        });

        TaskFuture::new(receiver)
    }
}

/// Type alias for a Task with a boxed async function.
pub type BoxedTask<Args, T> = Task<Box<AsyncFn<Args, T>>, Args, T>;

/// Create a task from an async function.
///
/// This is a convenience function that creates a `Task` wrapper around an async function.
///
/// # Example
///
/// ```ignore
/// use agent_graph::func::create_task;
///
/// let task = create_task("my_task", |x: i32| async move { x * 2 });
/// let result = task.call(5).await?;
/// ```
pub fn create_task<F, Fut, Args, T>(name: impl Into<String>, func: F) -> BoxedTask<Args, T>
where
    F: Fn(Args) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = T> + Send + 'static,
    Args: Send + 'static,
    T: Send + 'static,
{
    let func = Arc::new(func);
    let wrapper: Box<AsyncFn<Args, T>> = Box::new(move |args: Args| {
        let func = func.clone();
        Box::pin(async move { func(args).await }) as BoxFuture<T>
    });
    Task::new(wrapper).with_name(name)
}

/// A value returned by an entrypoint that decouples the return value from the saved state.
///
/// This is similar to Python's `entrypoint.final` - it allows returning one value
/// to the caller while saving a different value to the checkpoint.
///
/// # Example
///
/// ```ignore
/// use agent_graph::func::Final;
///
/// // Return 0 to caller, but save 6 to checkpoint for next invocation
/// Final::new(0, 6)
/// ```
#[derive(Debug, Clone)]
pub struct Final<R, S = R> {
    /// Value to return to the caller.
    pub value: R,
    /// Value to save to the checkpoint for the next invocation.
    pub save: S,
}

impl<R, S> Final<R, S> {
    /// Create a new Final with separate return and save values.
    pub fn new(value: R, save: S) -> Self {
        Self { value, save }
    }
}

impl<R> Final<R, R>
where
    R: Clone,
{
    /// Create a Final where the return and save values are the same.
    pub fn same(value: R) -> Self {
        Self {
            value: value.clone(),
            save: value,
        }
    }
}

/// Configuration for an entrypoint.
#[derive(Clone)]
pub struct EntrypointConfig<C = ()> {
    /// Optional checkpointer for state persistence.
    pub checkpointer: Option<InMemorySaver>,
    /// Optional context schema type.
    pub context: Option<C>,
    /// Optional cache policy.
    pub cache_policy: Option<CachePolicy>,
    /// Optional retry policy.
    pub retry_policy: Option<RetryPolicy>,
}

impl<C> Default for EntrypointConfig<C> {
    fn default() -> Self {
        Self {
            checkpointer: None,
            context: None,
            cache_policy: None,
            retry_policy: None,
        }
    }
}

/// An entrypoint for a LangGraph workflow.
///
/// The entrypoint wraps an async function and provides `invoke` and `stream` methods
/// for executing the workflow.
///
/// # Example
///
/// ```ignore
/// use agent_graph::func::Entrypoint;
/// use agent_graph::checkpoint::InMemorySaver;
///
/// let workflow = Entrypoint::builder()
///     .with_checkpointer(InMemorySaver::new())
///     .build(|input: String| async move {
///         input.to_uppercase()
///     });
///
/// let result = workflow.invoke("hello".to_string()).await;
/// assert_eq!(result, "HELLO");
/// ```
pub struct Entrypoint<F, I, O, C = ()>
where
    F: Fn(I) -> Pin<Box<dyn Future<Output = O> + Send>> + Send + Sync + 'static,
    I: Send + 'static,
    O: Send + Clone + 'static,
{
    func: Arc<F>,
    name: String,
    config: EntrypointConfig<C>,
    _phantom: PhantomData<(I, O)>,
}

impl<F, I, O, C> Entrypoint<F, I, O, C>
where
    F: Fn(I) -> Pin<Box<dyn Future<Output = O> + Send>> + Send + Sync + 'static,
    I: Send + 'static,
    O: Send + Clone + 'static,
    C: Send + Sync + 'static,
{
    /// Create a new entrypoint with the given function.
    pub fn new(name: impl Into<String>, func: F) -> Self {
        Self {
            func: Arc::new(func),
            name: name.into(),
            config: EntrypointConfig::default(),
            _phantom: PhantomData,
        }
    }

    /// Create a builder for configuring an entrypoint.
    pub fn builder() -> EntrypointBuilder<C> {
        EntrypointBuilder::new()
    }

    /// Set the checkpointer for this entrypoint.
    pub fn with_checkpointer(mut self, checkpointer: InMemorySaver) -> Self {
        self.config.checkpointer = Some(checkpointer);
        self
    }

    /// Set the cache policy for this entrypoint.
    pub fn with_cache_policy(mut self, policy: CachePolicy) -> Self {
        self.config.cache_policy = Some(policy);
        self
    }

    /// Set the retry policy for this entrypoint.
    pub fn with_retry_policy(mut self, policy: RetryPolicy) -> Self {
        self.config.retry_policy = Some(policy);
        self
    }

    /// Get the name of this entrypoint.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Invoke the workflow with the given input.
    ///
    /// Returns the final output of the workflow.
    pub async fn invoke(&self, input: I) -> O {
        (self.func)(input).await
    }

    /// Invoke the workflow with the given input and configuration.
    ///
    /// The config can contain a thread_id for checkpointing.
    ///
    /// Note: Full checkpointing requires the output type to implement Serialize/Deserialize.
    /// Use `invoke_with_config_serde` for full checkpoint support.
    pub async fn invoke_with_config(&self, input: I, _config: RunConfig) -> O {
        (self.func)(input).await
    }

    /// Stream the workflow execution.
    ///
    /// Returns a stream of `StreamChunk` values containing intermediate results.
    pub fn stream(
        &self,
        input: I,
        _mode: StreamMode,
    ) -> Pin<Box<dyn Stream<Item = StreamChunk<O>> + Send + '_>>
    where
        O: 'static,
    {
        let func = self.func.clone();
        let name = self.name.clone();

        Box::pin(stream::once(async move {
            let result = (func)(input).await;
            StreamChunk::new(name, result)
        }))
    }

    /// Stream the workflow with configuration.
    pub fn stream_with_config(
        &self,
        input: I,
        mode: StreamMode,
        _config: RunConfig,
    ) -> Pin<Box<dyn Stream<Item = StreamChunk<O>> + Send + '_>>
    where
        O: 'static,
    {
        self.stream(input, mode)
    }
}

/// Configuration for a workflow run.
#[derive(Debug, Clone, Default)]
pub struct RunConfig {
    /// Thread ID for checkpointing.
    pub thread_id: Option<String>,
    /// Additional metadata.
    pub metadata: HashMap<String, String>,
}

impl RunConfig {
    /// Create a new run configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the thread ID.
    pub fn with_thread_id(mut self, thread_id: impl Into<String>) -> Self {
        self.thread_id = Some(thread_id.into());
        self
    }

    /// Add metadata.
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// Builder for creating entrypoints.
pub struct EntrypointBuilder<C = ()> {
    checkpointer: Option<InMemorySaver>,
    context: Option<C>,
    cache_policy: Option<CachePolicy>,
    retry_policy: Option<RetryPolicy>,
    name: Option<String>,
}

impl<C> Default for EntrypointBuilder<C> {
    fn default() -> Self {
        Self::new()
    }
}

impl<C> EntrypointBuilder<C> {
    /// Create a new entrypoint builder.
    pub fn new() -> Self {
        Self {
            checkpointer: None,
            context: None,
            cache_policy: None,
            retry_policy: None,
            name: None,
        }
    }

    /// Set the checkpointer.
    pub fn with_checkpointer(mut self, checkpointer: InMemorySaver) -> Self {
        self.checkpointer = Some(checkpointer);
        self
    }

    /// Set the context.
    pub fn with_context(mut self, context: C) -> Self {
        self.context = Some(context);
        self
    }

    /// Set the cache policy.
    pub fn with_cache_policy(mut self, policy: CachePolicy) -> Self {
        self.cache_policy = Some(policy);
        self
    }

    /// Set the retry policy.
    pub fn with_retry_policy(mut self, policy: RetryPolicy) -> Self {
        self.retry_policy = Some(policy);
        self
    }

    /// Set the name.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Build the entrypoint with the given function.
    pub fn build<F, Fut, I, O>(self, func: F) -> BoxedEntrypoint<I, O, C>
    where
        F: Fn(I) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = O> + Send + 'static,
        I: Send + 'static,
        O: Send + Clone + 'static,
        C: Send + Sync + 'static,
    {
        let name = self.name.unwrap_or_else(|| "entrypoint".to_string());
        let func = Arc::new(func);

        let wrapper: Box<AsyncFn<I, O>> = Box::new(move |input: I| {
            let func = func.clone();
            Box::pin(async move { func(input).await }) as BoxFuture<O>
        });

        let mut entrypoint = Entrypoint::new(name, wrapper);
        entrypoint.config = EntrypointConfig {
            checkpointer: self.checkpointer,
            context: self.context,
            cache_policy: self.cache_policy,
            retry_policy: self.retry_policy,
        };
        entrypoint
    }
}

/// Type alias for an Entrypoint with a boxed async function and no context.
pub type BoxedEntrypoint<I, O, C = ()> = Entrypoint<Box<AsyncFn<I, O>>, I, O, C>;

/// Create an entrypoint from an async function.
///
/// This is a convenience function for quickly creating an entrypoint.
///
/// # Example
///
/// ```ignore
/// use agent_graph::func::create_entrypoint;
///
/// let workflow = create_entrypoint("my_workflow", |x: i32| async move { x * 2 });
/// let result = workflow.invoke(5).await;
/// ```
pub fn create_entrypoint<F, Fut, I, O>(name: impl Into<String>, func: F) -> BoxedEntrypoint<I, O>
where
    F: Fn(I) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = O> + Send + 'static,
    I: Send + 'static,
    O: Send + Clone + 'static,
{
    EntrypointBuilder::new().with_name(name).build(func)
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::stream::StreamExt;

    #[tokio::test]
    async fn test_task_future() {
        let (sender, receiver) = oneshot::channel();
        let future = TaskFuture::new(receiver);

        sender.send(42).unwrap();
        let result = future.await.unwrap();
        assert_eq!(result, 42);
    }

    #[tokio::test]
    async fn test_final() {
        let final_value = Final::new(10, 20);
        assert_eq!(final_value.value, 10);
        assert_eq!(final_value.save, 20);

        let same = Final::same(30);
        assert_eq!(same.value, 30);
        assert_eq!(same.save, 30);
    }

    #[tokio::test]
    async fn test_create_entrypoint() {
        let workflow = create_entrypoint("double", |x: i32| async move { x * 2 });

        let result = workflow.invoke(5).await;
        assert_eq!(result, 10);
    }

    #[tokio::test]
    async fn test_entrypoint_builder() {
        let workflow = EntrypointBuilder::<()>::new()
            .with_name("triple")
            .build(|x: i32| async move { x * 3 });

        let result = workflow.invoke(5).await;
        assert_eq!(result, 15);
    }

    #[tokio::test]
    async fn test_entrypoint_stream() {
        let workflow = create_entrypoint("uppercase", |s: String| async move { s.to_uppercase() });

        let chunks: Vec<_> = workflow
            .stream("hello".to_string(), StreamMode::Updates)
            .collect()
            .await;

        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].node, "uppercase");
        assert_eq!(chunks[0].data, "HELLO");
    }

    #[tokio::test]
    async fn test_run_config() {
        let config = RunConfig::new()
            .with_thread_id("thread-1")
            .with_metadata("user_id", "123");

        assert_eq!(config.thread_id, Some("thread-1".to_string()));
        assert_eq!(config.metadata.get("user_id"), Some(&"123".to_string()));
    }
}
