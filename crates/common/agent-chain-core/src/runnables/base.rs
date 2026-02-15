//! Base classes and utilities for Runnables.
//!
//! This module provides the core `Runnable` trait and implementations,
//! mirroring `langchain_core.runnables.base`.

use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use async_trait::async_trait;
use futures::StreamExt;
use futures::stream::BoxStream;
use serde::Serialize;
use serde_json::Value;
use tokio::sync::Semaphore;

use crate::error::{Error, Result};
use crate::load::{Serializable, Serialized};

use super::config::{
    AsyncVariableArgsFn, ConfigOrList, RunnableConfig, VariableArgsFn,
    acall_func_with_variable_args, call_func_with_variable_args, ensure_config,
    get_async_callback_manager_for_config, get_callback_manager_for_config, get_config_list,
    merge_configs, patch_config,
};

/// Number of generic type arguments for Runnable (Input and Output).
#[allow(dead_code)]
const RUNNABLE_GENERIC_NUM_ARGS: usize = 2;

/// Minimum number of steps in a RunnableSequence.
#[allow(dead_code)]
const RUNNABLE_SEQUENCE_MIN_STEPS: usize = 2;

/// A unit of work that can be invoked, batched, streamed, transformed and composed.
///
/// Key Methods:
/// - `invoke`/`ainvoke`: Transforms a single input into an output.
/// - `batch`/`abatch`: Efficiently transforms multiple inputs into outputs.
/// - `stream`/`astream`: Streams output from a single input as it's produced.
///
/// Built-in optimizations:
/// - **Batch**: By default, batch runs invoke() in parallel using threads.
///   Override to optimize batching.
/// - **Async**: Methods with `'a'` prefix are asynchronous. By default, they execute
///   the sync counterpart using async runtime. Override for native async.
///
/// All methods accept an optional config argument, which can be used to configure
/// execution, add tags and metadata for tracing and debugging.
#[async_trait]
pub trait Runnable: Send + Sync + Debug {
    /// The input type for this Runnable.
    type Input: Send + Sync + Clone + Debug + 'static;
    /// The output type for this Runnable.
    type Output: Send + Sync + Clone + Debug + 'static;

    /// Get the name of this Runnable.
    fn get_name(&self, suffix: Option<&str>, name: Option<&str>) -> String {
        let name_ = name
            .map(|s| s.to_string())
            .or_else(|| self.name())
            .unwrap_or_else(|| self.type_name().to_string());

        match suffix {
            Some(s) if !name_.is_empty() && name_.chars().next().unwrap().is_uppercase() => {
                format!("{}{}", name_, to_title_case(s))
            }
            Some(s) => format!("{}_{}", name_, s.to_lowercase()),
            None => name_,
        }
    }

    /// Get the optional name of this Runnable.
    fn name(&self) -> Option<String> {
        None
    }

    /// Get the type name of this Runnable.
    fn type_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    /// Get a JSON schema describing the input type of this Runnable.
    ///
    /// Mirrors `Runnable.get_input_schema()` from
    /// `langchain_core.runnables.base`.
    ///
    /// The default implementation returns a generic object schema derived
    /// from the Runnable's name. Wrapper runnables (retry, fallbacks, etc.)
    /// override this to delegate to the wrapped runnable's schema.
    fn get_input_schema(&self, _config: Option<&RunnableConfig>) -> Value {
        serde_json::json!({
            "title": self.get_name(Some("Input"), None),
            "type": "object"
        })
    }

    /// Get a JSON schema describing the output type of this Runnable.
    ///
    /// Mirrors `Runnable.get_output_schema()` from
    /// `langchain_core.runnables.base`.
    fn get_output_schema(&self, _config: Option<&RunnableConfig>) -> Value {
        serde_json::json!({
            "title": self.get_name(Some("Output"), None),
            "type": "object"
        })
    }

    /// Transform a single input into an output.
    ///
    /// # Arguments
    ///
    /// * `input` - The input to the Runnable.
    /// * `config` - Optional config to use when invoking the Runnable.
    ///
    /// # Returns
    ///
    /// The output of the Runnable.
    fn invoke(&self, input: Self::Input, config: Option<RunnableConfig>) -> Result<Self::Output>;

    /// Transform a single input into an output asynchronously.
    ///
    /// Default implementation runs invoke() in a blocking task.
    async fn ainvoke(
        &self,
        input: Self::Input,
        config: Option<RunnableConfig>,
    ) -> Result<Self::Output>
    where
        Self: 'static,
    {
        // Default implementation: run invoke in a blocking context
        self.invoke(input, config)
    }

    /// Transform multiple inputs into outputs in parallel.
    ///
    /// Default implementation runs invoke() in parallel using scoped threads,
    /// respecting the `max_concurrency` setting from config.
    fn batch(
        &self,
        inputs: Vec<Self::Input>,
        config: Option<ConfigOrList>,
        return_exceptions: bool,
    ) -> Vec<Result<Self::Output>>
    where
        Self: 'static,
    {
        if inputs.is_empty() {
            return Vec::new();
        }

        let configs = get_config_list(config, inputs.len());

        // For single input, just invoke directly
        if inputs.len() == 1 {
            let input = inputs.into_iter().next().unwrap();
            let config = configs.into_iter().next().unwrap();
            let result = self.invoke(input, Some(config));
            if return_exceptions {
                return vec![result];
            }
            return vec![result];
        }

        let max_concurrency = configs[0].max_concurrency;
        let len = inputs.len();
        let mut results: Vec<Option<Result<Self::Output>>> = (0..len).map(|_| None).collect();

        std::thread::scope(|scope| {
            let active_count = Arc::new(AtomicUsize::new(0));
            let semaphore_like = max_concurrency;
            let mut handles = Vec::with_capacity(len);

            for (i, (input, config)) in inputs.into_iter().zip(configs).enumerate() {
                // Simple concurrency limiting: wait until a slot is available
                if let Some(max) = semaphore_like {
                    while active_count.load(Ordering::SeqCst) >= max {
                        std::thread::sleep(std::time::Duration::from_millis(1));
                    }
                }
                let active = active_count.clone();
                active.fetch_add(1, Ordering::SeqCst);

                let handle = scope.spawn(move || {
                    // TODO: when return_exceptions is true, catch panics and return them as errors
                    let _ = return_exceptions;
                    let result = self.invoke(input, Some(config));
                    active.fetch_sub(1, Ordering::SeqCst);
                    (i, result)
                });
                handles.push(handle);
            }

            for handle in handles {
                let (i, result) = handle.join().expect("thread should not panic");
                results[i] = Some(result);
            }
        });

        results.into_iter().map(|r| r.unwrap()).collect()
    }

    /// Transform multiple inputs into outputs asynchronously.
    ///
    /// Default implementation runs ainvoke() concurrently, respecting the
    /// `max_concurrency` setting from config using a semaphore.
    async fn abatch(
        &self,
        inputs: Vec<Self::Input>,
        config: Option<ConfigOrList>,
        _return_exceptions: bool,
    ) -> Vec<Result<Self::Output>>
    where
        Self: 'static,
    {
        if inputs.is_empty() {
            return Vec::new();
        }

        let configs = get_config_list(config, inputs.len());
        let max_concurrency = configs[0].max_concurrency;

        match max_concurrency {
            Some(limit) if limit > 0 => {
                let semaphore = Arc::new(Semaphore::new(limit));
                let futures: Vec<_> = inputs
                    .into_iter()
                    .zip(configs)
                    .map(|(input, config)| {
                        let sem = semaphore.clone();
                        async move {
                            let _permit =
                                sem.acquire().await.expect("semaphore should not be closed");
                            self.ainvoke(input, Some(config)).await
                        }
                    })
                    .collect();
                futures::future::join_all(futures).await
            }
            _ => {
                let futures: Vec<_> = inputs
                    .into_iter()
                    .zip(configs)
                    .map(|(input, config)| self.ainvoke(input, Some(config)))
                    .collect();
                futures::future::join_all(futures).await
            }
        }
    }

    /// Run invoke in parallel on a list of inputs, yielding results as they
    /// complete.
    ///
    /// Default implementation uses scoped threads with concurrency limiting.
    fn batch_as_completed(
        &self,
        inputs: Vec<Self::Input>,
        config: Option<ConfigOrList>,
        _return_exceptions: bool,
    ) -> Vec<(usize, Result<Self::Output>)>
    where
        Self: 'static,
    {
        if inputs.is_empty() {
            return Vec::new();
        }

        let configs = get_config_list(config, inputs.len());

        if inputs.len() == 1 {
            let input = inputs.into_iter().next().unwrap();
            let config = configs.into_iter().next().unwrap();
            return vec![(0, self.invoke(input, Some(config)))];
        }

        let max_concurrency = configs[0].max_concurrency;
        let (sender, receiver) = std::sync::mpsc::channel();

        std::thread::scope(|scope| {
            let active_count = Arc::new(AtomicUsize::new(0));

            for (i, (input, config)) in inputs.into_iter().zip(configs).enumerate() {
                if let Some(max) = max_concurrency {
                    while active_count.load(Ordering::SeqCst) >= max {
                        std::thread::sleep(std::time::Duration::from_millis(1));
                    }
                }
                let active = active_count.clone();
                active.fetch_add(1, Ordering::SeqCst);
                let tx = sender.clone();

                scope.spawn(move || {
                    let result = self.invoke(input, Some(config));
                    active.fetch_sub(1, Ordering::SeqCst);
                    tx.send((i, result))
                        .expect("receiver should not be dropped");
                });
            }

            // Drop the original sender so the receiver will terminate
            drop(sender);
        });

        receiver.into_iter().collect()
    }

    /// Run ainvoke in parallel on a list of inputs, yielding results as they
    /// complete.
    ///
    /// Default implementation uses FuturesUnordered with semaphore-based
    /// concurrency limiting. Returns a stream of (index, result) tuples.
    fn abatch_as_completed(
        &self,
        inputs: Vec<Self::Input>,
        config: Option<ConfigOrList>,
        _return_exceptions: bool,
    ) -> BoxStream<'_, (usize, Result<Self::Output>)>
    where
        Self: 'static,
    {
        if inputs.is_empty() {
            return Box::pin(futures::stream::empty());
        }

        let configs = get_config_list(config, inputs.len());
        let max_concurrency = configs[0].max_concurrency;
        let semaphore = max_concurrency.map(|n| Arc::new(Semaphore::new(n)));

        let futures_unordered = futures::stream::FuturesUnordered::new();

        for (i, (input, config)) in inputs.into_iter().zip(configs).enumerate() {
            let sem = semaphore.clone();
            futures_unordered.push(async move {
                let _permit = match sem {
                    Some(ref s) => Some(s.acquire().await.expect("semaphore should not be closed")),
                    None => None,
                };
                let result = self.ainvoke(input, Some(config)).await;
                (i, result)
            });
        }

        Box::pin(futures_unordered)
    }

    /// Stream output from a single input.
    ///
    /// Default implementation calls invoke() and yields the result.
    fn stream(
        &self,
        input: Self::Input,
        config: Option<RunnableConfig>,
    ) -> BoxStream<'_, Result<Self::Output>> {
        let result = self.invoke(input, config);
        Box::pin(futures::stream::once(async move { result }))
    }

    /// Stream output from a single input asynchronously.
    ///
    /// Default implementation calls ainvoke() and yields the result.
    fn astream(
        &self,
        input: Self::Input,
        config: Option<RunnableConfig>,
    ) -> BoxStream<'_, Result<Self::Output>>
    where
        Self: 'static,
    {
        Box::pin(futures::stream::once(async move {
            self.ainvoke(input, config).await
        }))
    }

    /// Transform an input stream into an output stream.
    ///
    /// Default implementation buffers input and calls stream().
    fn transform<'a>(
        &'a self,
        input: BoxStream<'a, Self::Input>,
        config: Option<RunnableConfig>,
    ) -> BoxStream<'a, Result<Self::Output>> {
        Box::pin(async_stream::stream! {
            let mut final_input: Option<Self::Input> = None;
            let mut input = input;

            while let Some(ichunk) = input.next().await {
                if let Some(ref mut current) = final_input {
                    // Try to combine inputs if possible
                    // For now, just take the last one
                    *current = ichunk;
                } else {
                    final_input = Some(ichunk);
                }
            }

            if let Some(input) = final_input {
                let mut stream = self.stream(input, config);
                while let Some(output) = stream.next().await {
                    yield output;
                }
            }
        })
    }

    /// Transform an input stream into an output stream asynchronously.
    ///
    /// Default implementation buffers input and calls astream().
    fn atransform<'a>(
        &'a self,
        input: BoxStream<'a, Self::Input>,
        config: Option<RunnableConfig>,
    ) -> BoxStream<'a, Result<Self::Output>>
    where
        Self: 'static,
    {
        Box::pin(async_stream::stream! {
            let mut final_input: Option<Self::Input> = None;
            let mut input = input;

            while let Some(ichunk) = input.next().await {
                if let Some(ref mut current) = final_input {
                    *current = ichunk;
                } else {
                    final_input = Some(ichunk);
                }
            }

            if let Some(input) = final_input {
                let mut stream = self.astream(input, config);
                while let Some(output) = stream.next().await {
                    yield output;
                }
            }
        })
    }

    /// Bind arguments to this Runnable, returning a new Runnable.
    fn bind(self, kwargs: HashMap<String, Value>) -> RunnableBinding<Self>
    where
        Self: Sized,
    {
        RunnableBinding::new(self, kwargs, None)
    }

    /// Bind config to this Runnable, returning a new Runnable.
    fn with_config(self, config: RunnableConfig) -> RunnableBinding<Self>
    where
        Self: Sized,
    {
        RunnableBinding::new(self, HashMap::new(), Some(config))
    }

    /// Create a new Runnable that retries on failure.
    ///
    /// This is a convenience method that uses the `RunnableRetryExt` trait.
    /// For more control over retry behavior, use `RunnableRetryExt::with_retry`
    /// with a `RunnableRetryConfig`.
    fn with_retry(
        self,
        max_attempts: usize,
        wait_exponential_jitter: bool,
    ) -> super::retry::RunnableRetry<Self>
    where
        Self: Sized,
    {
        super::retry::RunnableRetry::with_simple(self, max_attempts, wait_exponential_jitter)
    }

    /// Return a new Runnable that maps a list of inputs to a list of outputs.
    fn map(self) -> RunnableEach<Self>
    where
        Self: Sized,
    {
        RunnableEach::new(self)
    }
}

/// Convert a string to title case.
fn to_title_case(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().chain(chars).collect(),
    }
}

/// A Runnable that can be serialized to JSON.
pub trait RunnableSerializable: Runnable + Serializable {
    /// Serialize this Runnable to JSON.
    fn to_json_runnable(&self) -> Serialized
    where
        Self: Sized + Serialize,
    {
        <Self as Serializable>::to_json(self)
    }
}

// =============================================================================
// RunnableLambda
// =============================================================================

/// A Runnable that wraps a function.
///
/// `RunnableLambda` converts a callable into a `Runnable`.
/// Wrapping a callable in a `RunnableLambda` makes the callable usable
/// within either a sync or async context.
pub struct RunnableLambda<F, I, O>
where
    F: Fn(I) -> Result<O> + Send + Sync,
    I: Send + Sync + Clone + Debug + 'static,
    O: Send + Sync + Clone + Debug + 'static,
{
    func: F,
    name: Option<String>,
    _phantom: std::marker::PhantomData<(I, O)>,
}

impl<F, I, O> Debug for RunnableLambda<F, I, O>
where
    F: Fn(I) -> Result<O> + Send + Sync,
    I: Send + Sync + Clone + Debug + 'static,
    O: Send + Sync + Clone + Debug + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RunnableLambda")
            .field("name", &self.name)
            .finish()
    }
}

impl<F, I, O> RunnableLambda<F, I, O>
where
    F: Fn(I) -> Result<O> + Send + Sync,
    I: Send + Sync + Clone + Debug + 'static,
    O: Send + Sync + Clone + Debug + 'static,
{
    /// Create a new RunnableLambda from a function.
    pub fn new(func: F) -> Self {
        Self {
            func,
            name: None,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Create a new RunnableLambda with a name.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }
}

#[async_trait]
impl<F, I, O> Runnable for RunnableLambda<F, I, O>
where
    F: Fn(I) -> Result<O> + Send + Sync,
    I: Send + Sync + Clone + Debug + 'static,
    O: Send + Sync + Clone + Debug + 'static,
{
    type Input = I;
    type Output = O;

    fn name(&self) -> Option<String> {
        self.name.clone()
    }

    fn invoke(&self, input: Self::Input, config: Option<RunnableConfig>) -> Result<Self::Output> {
        let config = ensure_config(config);
        let callback_manager = get_callback_manager_for_config(&config);
        let run_manager = callback_manager
            .on_chain_start()
            .serialized(&HashMap::new())
            .inputs(&HashMap::new())
            .maybe_run_id(config.run_id)
            .call();

        match (self.func)(input) {
            Ok(output) => {
                run_manager.on_chain_end(&HashMap::new());
                Ok(output)
            }
            Err(e) => {
                run_manager.on_chain_error(&e);
                Err(e)
            }
        }
    }
}

/// Create a RunnableLambda from a function.
pub fn runnable_lambda<F, I, O>(func: F) -> RunnableLambda<F, I, O>
where
    F: Fn(I) -> Result<O> + Send + Sync,
    I: Send + Sync + Clone + Debug + 'static,
    O: Send + Sync + Clone + Debug + 'static,
{
    RunnableLambda::new(func)
}

// =============================================================================
// RunnableLambdaWithConfig
// =============================================================================

/// A config-aware version of `RunnableLambda` that supports functions which
/// receive `RunnableConfig`, as well as async functions.
///
/// Mirrors Python's `RunnableLambda` support for functions with optional
/// `config` parameter. Uses `VariableArgsFn` / `AsyncVariableArgsFn` enums
/// to dispatch to the correct function signature at runtime.
///
/// # Examples
///
///
pub struct RunnableLambdaWithConfig<I, O>
where
    I: Send + Sync + Clone + Debug + 'static,
    O: Send + Sync + Clone + Debug + 'static,
{
    func: Option<VariableArgsFn<I, Result<O>>>,
    afunc: Option<AsyncVariableArgsFn<I, Result<O>>>,
    name: Option<String>,
}

impl<I, O> Debug for RunnableLambdaWithConfig<I, O>
where
    I: Send + Sync + Clone + Debug + 'static,
    O: Send + Sync + Clone + Debug + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RunnableLambdaWithConfig")
            .field("name", &self.name)
            .field("has_func", &self.func.is_some())
            .field("has_afunc", &self.afunc.is_some())
            .finish()
    }
}

impl<I, O> RunnableLambdaWithConfig<I, O>
where
    I: Send + Sync + Clone + Debug + 'static,
    O: Send + Sync + Clone + Debug + 'static,
{
    /// Create from a sync function that only takes input.
    pub fn new(func: impl Fn(I) -> Result<O> + Send + Sync + 'static) -> Self {
        Self {
            func: Some(VariableArgsFn::InputOnly(Box::new(func))),
            afunc: None,
            name: None,
        }
    }

    /// Create from a sync function that takes input and config.
    pub fn new_with_config(
        func: impl Fn(I, &RunnableConfig) -> Result<O> + Send + Sync + 'static,
    ) -> Self {
        Self {
            func: Some(VariableArgsFn::WithConfig(Box::new(func))),
            afunc: None,
            name: None,
        }
    }

    /// Create from an async function that only takes input.
    pub fn new_async<F, Fut>(afunc: F) -> Self
    where
        F: Fn(I) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<O>> + Send + 'static,
    {
        Self {
            func: None,
            afunc: Some(AsyncVariableArgsFn::InputOnly(Box::new(move |input| {
                Box::pin(afunc(input))
            }))),
            name: None,
        }
    }

    /// Create from an async function that takes input and config.
    pub fn new_async_with_config<F, Fut>(afunc: F) -> Self
    where
        F: Fn(I, RunnableConfig) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<O>> + Send + 'static,
    {
        Self {
            func: None,
            afunc: Some(AsyncVariableArgsFn::WithConfig(Box::new(
                move |input, config| Box::pin(afunc(input, config)),
            ))),
            name: None,
        }
    }

    /// Set a name for this runnable.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Add an async function to this runnable (for use with ainvoke).
    pub fn with_afunc<F, Fut>(mut self, afunc: F) -> Self
    where
        F: Fn(I) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<O>> + Send + 'static,
    {
        self.afunc = Some(AsyncVariableArgsFn::InputOnly(Box::new(move |input| {
            Box::pin(afunc(input))
        })));
        self
    }

    /// Add a config-aware async function to this runnable.
    pub fn with_afunc_config<F, Fut>(mut self, afunc: F) -> Self
    where
        F: Fn(I, RunnableConfig) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<O>> + Send + 'static,
    {
        self.afunc = Some(AsyncVariableArgsFn::WithConfig(Box::new(
            move |input, config| Box::pin(afunc(input, config)),
        )));
        self
    }
}

#[async_trait]
impl<I, O> Runnable for RunnableLambdaWithConfig<I, O>
where
    I: Send + Sync + Clone + Debug + 'static,
    O: Send + Sync + Clone + Debug + 'static,
{
    type Input = I;
    type Output = O;

    fn name(&self) -> Option<String> {
        self.name.clone()
    }

    fn invoke(&self, input: Self::Input, config: Option<RunnableConfig>) -> Result<Self::Output> {
        let func = self.func.as_ref().ok_or_else(|| {
            Error::other("Cannot invoke a coroutine function synchronously. Use ainvoke instead.")
        })?;

        let config = ensure_config(config);
        let callback_manager = get_callback_manager_for_config(&config);
        let run_manager = callback_manager
            .on_chain_start()
            .serialized(&HashMap::new())
            .inputs(&HashMap::new())
            .maybe_run_id(config.run_id)
            .call();

        let child_config = patch_config(
            Some(config),
            Some(run_manager.get_child(None)),
            None,
            None,
            None,
            None,
        );

        match call_func_with_variable_args(func, input, &child_config) {
            Ok(output) => {
                run_manager.on_chain_end(&HashMap::new());
                Ok(output)
            }
            Err(e) => {
                run_manager.on_chain_error(&e);
                Err(e)
            }
        }
    }

    async fn ainvoke(
        &self,
        input: Self::Input,
        config: Option<RunnableConfig>,
    ) -> Result<Self::Output>
    where
        Self: 'static,
    {
        let config = ensure_config(config);
        let async_callback_manager = get_async_callback_manager_for_config(&config);
        let run_manager = async_callback_manager
            .on_chain_start(&HashMap::new(), &HashMap::new(), config.run_id, None)
            .await;

        let child_config = patch_config(
            Some(config),
            Some(run_manager.get_child(None).to_callback_manager()),
            None,
            None,
            None,
            None,
        );

        let result = if let Some(afunc) = &self.afunc {
            acall_func_with_variable_args(afunc, input, &child_config).await
        } else if let Some(func) = &self.func {
            call_func_with_variable_args(func, input, &child_config)
        } else {
            Err(Error::other(
                "RunnableLambdaWithConfig has no func or afunc",
            ))
        };

        match result {
            Ok(output) => {
                run_manager.get_sync().on_chain_end(&HashMap::new());
                Ok(output)
            }
            Err(e) => {
                run_manager.get_sync().on_chain_error(&e);
                Err(e)
            }
        }
    }
}

// =============================================================================
// RunnableSequence
// =============================================================================

/// A sequence of Runnables that are executed one after another.
///
/// The output of one Runnable is the input to the next.
/// This is the most common composition pattern in LangChain.
pub struct RunnableSequence<R1, R2>
where
    R1: Runnable,
    R2: Runnable<Input = R1::Output>,
{
    first: R1,
    last: R2,
    name: Option<String>,
}

impl<R1, R2> Debug for RunnableSequence<R1, R2>
where
    R1: Runnable,
    R2: Runnable<Input = R1::Output>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RunnableSequence")
            .field("first", &self.first)
            .field("last", &self.last)
            .field("name", &self.name)
            .finish()
    }
}

impl<R1, R2> RunnableSequence<R1, R2>
where
    R1: Runnable,
    R2: Runnable<Input = R1::Output>,
{
    /// Create a new RunnableSequence.
    pub fn new(first: R1, last: R2) -> Self {
        Self {
            first,
            last,
            name: None,
        }
    }

    /// Create a new RunnableSequence with a name.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }
}

#[async_trait]
impl<R1, R2> Runnable for RunnableSequence<R1, R2>
where
    R1: Runnable + 'static,
    R2: Runnable<Input = R1::Output> + 'static,
{
    type Input = R1::Input;
    type Output = R2::Output;

    fn name(&self) -> Option<String> {
        self.name.clone()
    }

    fn invoke(&self, input: Self::Input, config: Option<RunnableConfig>) -> Result<Self::Output> {
        let config = ensure_config(config);
        let callback_manager = get_callback_manager_for_config(&config);

        // Start the chain run
        let run_manager = callback_manager
            .on_chain_start()
            .serialized(&HashMap::new())
            .inputs(&HashMap::new())
            .maybe_run_id(config.run_id)
            .call();

        // Invoke first step
        let first_config = patch_config(
            Some(config.clone()),
            Some(run_manager.get_child(Some("seq:step:1"))),
            None,
            None,
            None,
            None,
        );
        let intermediate = match self.first.invoke(input, Some(first_config)) {
            Ok(output) => output,
            Err(e) => {
                run_manager.on_chain_error(&e);
                return Err(e);
            }
        };

        // Invoke second step
        let last_config = patch_config(
            Some(config),
            Some(run_manager.get_child(Some("seq:step:2"))),
            None,
            None,
            None,
            None,
        );
        let result = match self.last.invoke(intermediate, Some(last_config)) {
            Ok(output) => output,
            Err(e) => {
                run_manager.on_chain_error(&e);
                return Err(e);
            }
        };

        run_manager.on_chain_end(&HashMap::new());
        Ok(result)
    }

    async fn ainvoke(
        &self,
        input: Self::Input,
        config: Option<RunnableConfig>,
    ) -> Result<Self::Output>
    where
        Self: 'static,
    {
        let config = ensure_config(config);
        let async_callback_manager = get_async_callback_manager_for_config(&config);
        let run_manager = async_callback_manager
            .on_chain_start(&HashMap::new(), &HashMap::new(), config.run_id, None)
            .await;

        // Invoke first step
        let first_config = patch_config(
            Some(config.clone()),
            Some(
                run_manager
                    .get_child(Some("seq:step:1"))
                    .to_callback_manager(),
            ),
            None,
            None,
            None,
            None,
        );
        let intermediate = match self.first.ainvoke(input, Some(first_config)).await {
            Ok(output) => output,
            Err(e) => {
                run_manager.get_sync().on_chain_error(&e);
                return Err(e);
            }
        };

        // Invoke second step
        let last_config = patch_config(
            Some(config),
            Some(
                run_manager
                    .get_child(Some("seq:step:2"))
                    .to_callback_manager(),
            ),
            None,
            None,
            None,
            None,
        );
        let result = match self.last.ainvoke(intermediate, Some(last_config)).await {
            Ok(output) => output,
            Err(e) => {
                run_manager.get_sync().on_chain_error(&e);
                return Err(e);
            }
        };

        run_manager.get_sync().on_chain_end(&HashMap::new());
        Ok(result)
    }

    fn stream(
        &self,
        input: Self::Input,
        config: Option<RunnableConfig>,
    ) -> BoxStream<'_, Result<Self::Output>> {
        Box::pin(async_stream::stream! {
            let config = ensure_config(config);

            // Invoke first step
            let intermediate = match self.first.invoke(input, Some(config.clone())) {
                Ok(output) => output,
                Err(e) => {
                    yield Err(e);
                    return;
                }
            };

            // Stream from second step
            let mut stream = self.last.stream(intermediate, Some(config));
            while let Some(output) = stream.next().await {
                yield output;
            }
        })
    }
}

/// Create a RunnableSequence by piping two Runnables together.
pub fn pipe<R1, R2>(first: R1, second: R2) -> RunnableSequence<R1, R2>
where
    R1: Runnable,
    R2: Runnable<Input = R1::Output>,
{
    RunnableSequence::new(first, second)
}

// =============================================================================
// RunnableParallel
// =============================================================================

/// A Runnable that runs multiple Runnables in parallel.
///
/// Returns a HashMap with the results keyed by the step names.
pub struct RunnableParallel<I>
where
    I: Send + Sync + Clone + Debug + 'static,
{
    steps: HashMap<String, Arc<dyn Runnable<Input = I, Output = Value> + Send + Sync>>,
    name: Option<String>,
}

impl<I> Debug for RunnableParallel<I>
where
    I: Send + Sync + Clone + Debug + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RunnableParallel")
            .field("steps", &self.steps.keys().collect::<Vec<_>>())
            .field("name", &self.name)
            .finish()
    }
}

impl<I> RunnableParallel<I>
where
    I: Send + Sync + Clone + Debug + 'static,
{
    /// Create a new empty RunnableParallel.
    pub fn new() -> Self {
        Self {
            steps: HashMap::new(),
            name: None,
        }
    }

    /// Add a step to the RunnableParallel.
    pub fn add<R>(mut self, key: impl Into<String>, runnable: R) -> Self
    where
        R: Runnable<Input = I, Output = Value> + Send + Sync + 'static,
    {
        self.steps.insert(key.into(), Arc::new(runnable));
        self
    }

    /// Set the name of this RunnableParallel.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }
}

impl<I> Default for RunnableParallel<I>
where
    I: Send + Sync + Clone + Debug + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl<I> Runnable for RunnableParallel<I>
where
    I: Send + Sync + Clone + Debug + 'static,
{
    type Input = I;
    type Output = HashMap<String, Value>;

    fn name(&self) -> Option<String> {
        self.name.clone().or_else(|| {
            Some(format!(
                "RunnableParallel<{}>",
                self.steps.keys().cloned().collect::<Vec<_>>().join(",")
            ))
        })
    }

    fn invoke(&self, input: Self::Input, config: Option<RunnableConfig>) -> Result<Self::Output> {
        let config = ensure_config(config);

        let step_entries: Vec<_> = self.steps.iter().collect();
        let mut results = HashMap::new();

        std::thread::scope(|scope| {
            let handles: Vec<_> = step_entries
                .iter()
                .map(|(key, step)| {
                    let input = input.clone();
                    let config = config.clone();
                    let key = (*key).clone();
                    scope.spawn(move || {
                        let result = step.invoke(input, Some(config));
                        (key, result)
                    })
                })
                .collect();

            for handle in handles {
                let (key, result) = handle.join().expect("thread should not panic");
                results.insert(key, result?);
            }

            Ok(results)
        })
    }

    async fn ainvoke(
        &self,
        input: Self::Input,
        config: Option<RunnableConfig>,
    ) -> Result<Self::Output>
    where
        Self: 'static,
    {
        let config = ensure_config(config);

        let futures: Vec<_> = self
            .steps
            .iter()
            .map(|(key, step)| {
                let input = input.clone();
                let config = config.clone();
                let key = key.clone();
                async move {
                    let result = step.ainvoke(input, Some(config)).await;
                    (key, result)
                }
            })
            .collect();

        let completed = futures::future::join_all(futures).await;

        let mut results = HashMap::new();
        for (key, result) in completed {
            results.insert(key, result?);
        }
        Ok(results)
    }
}

// =============================================================================
// RunnableBinding
// =============================================================================

/// A Runnable that binds arguments or config to another Runnable.
pub struct RunnableBinding<R>
where
    R: Runnable,
{
    bound: R,
    kwargs: HashMap<String, Value>,
    config: Option<RunnableConfig>,
}

impl<R> Debug for RunnableBinding<R>
where
    R: Runnable,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RunnableBinding")
            .field("bound", &self.bound)
            .field("kwargs", &self.kwargs)
            .field("config", &self.config)
            .finish()
    }
}

impl<R> RunnableBinding<R>
where
    R: Runnable,
{
    /// Create a new RunnableBinding.
    pub fn new(bound: R, kwargs: HashMap<String, Value>, config: Option<RunnableConfig>) -> Self {
        Self {
            bound,
            kwargs,
            config,
        }
    }

    /// Merge configs for the binding.
    fn merge_configs(&self, config: Option<RunnableConfig>) -> RunnableConfig {
        merge_configs(vec![self.config.clone(), config])
    }
}

#[async_trait]
impl<R> Runnable for RunnableBinding<R>
where
    R: Runnable + 'static,
{
    type Input = R::Input;
    type Output = R::Output;

    fn name(&self) -> Option<String> {
        self.bound.name()
    }

    fn get_input_schema(&self, config: Option<&RunnableConfig>) -> Value {
        self.bound.get_input_schema(config)
    }

    fn get_output_schema(&self, config: Option<&RunnableConfig>) -> Value {
        self.bound.get_output_schema(config)
    }

    fn invoke(&self, input: Self::Input, config: Option<RunnableConfig>) -> Result<Self::Output> {
        self.bound.invoke(input, Some(self.merge_configs(config)))
    }

    async fn ainvoke(
        &self,
        input: Self::Input,
        config: Option<RunnableConfig>,
    ) -> Result<Self::Output>
    where
        Self: 'static,
    {
        self.bound
            .ainvoke(input, Some(self.merge_configs(config)))
            .await
    }

    fn stream(
        &self,
        input: Self::Input,
        config: Option<RunnableConfig>,
    ) -> BoxStream<'_, Result<Self::Output>> {
        self.bound.stream(input, Some(self.merge_configs(config)))
    }
}

// =============================================================================
// RunnableEach
// =============================================================================

/// A Runnable that maps over a list of inputs.
pub struct RunnableEach<R>
where
    R: Runnable,
{
    bound: R,
}

impl<R> Debug for RunnableEach<R>
where
    R: Runnable,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RunnableEach")
            .field("bound", &self.bound)
            .finish()
    }
}

impl<R> RunnableEach<R>
where
    R: Runnable,
{
    /// Create a new RunnableEach.
    pub fn new(bound: R) -> Self {
        Self { bound }
    }
}

#[async_trait]
impl<R> Runnable for RunnableEach<R>
where
    R: Runnable + 'static,
{
    type Input = Vec<R::Input>;
    type Output = Vec<R::Output>;

    fn name(&self) -> Option<String> {
        self.bound.name().map(|n| format!("RunnableEach<{}>", n))
    }

    fn invoke(&self, inputs: Self::Input, config: Option<RunnableConfig>) -> Result<Self::Output> {
        let config = ensure_config(config);
        let configs =
            super::config::ConfigOrList::List(inputs.iter().map(|_| config.clone()).collect());

        let results = self.bound.batch(inputs, Some(configs), false);

        // Collect results, returning error if any failed
        results.into_iter().collect()
    }

    async fn ainvoke(
        &self,
        inputs: Self::Input,
        config: Option<RunnableConfig>,
    ) -> Result<Self::Output>
    where
        Self: 'static,
    {
        let config = ensure_config(config);
        let configs =
            super::config::ConfigOrList::List(inputs.iter().map(|_| config.clone()).collect());

        let results = self.bound.abatch(inputs, Some(configs), false).await;

        results.into_iter().collect()
    }
}

// =============================================================================
// DynRunnable - Type-erased Runnable
// =============================================================================

/// A type-erased Runnable that can be stored in collections.
pub type DynRunnable<I, O> = Arc<dyn Runnable<Input = I, Output = O> + Send + Sync>;

/// Convert any Runnable into a DynRunnable.
pub fn to_dyn<R>(runnable: R) -> DynRunnable<R::Input, R::Output>
where
    R: Runnable + Send + Sync + 'static,
{
    Arc::new(runnable)
}

// =============================================================================
// Helper functions
// =============================================================================

/// Coerce a function into a Runnable.
pub fn coerce_to_runnable<F, I, O>(func: F) -> RunnableLambda<F, I, O>
where
    F: Fn(I) -> Result<O> + Send + Sync,
    I: Send + Sync + Clone + Debug + 'static,
    O: Send + Sync + Clone + Debug + 'static,
{
    RunnableLambda::new(func)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runnables::passthrough::RunnablePassthrough;
    use crate::runnables::utils::AddableDict;

    #[test]
    fn test_runnable_lambda() {
        let runnable = RunnableLambda::new(|x: i32| Ok(x + 1));
        let result = runnable.invoke(1, None).unwrap();
        assert_eq!(result, 2);
    }

    #[test]
    fn test_runnable_lambda_with_name() {
        let runnable = RunnableLambda::new(|x: i32| Ok(x + 1)).with_name("add_one");
        assert_eq!(runnable.name(), Some("add_one".to_string()));
    }

    #[test]
    fn test_runnable_sequence() {
        let first = RunnableLambda::new(|x: i32| Ok(x + 1));
        let second = RunnableLambda::new(|x: i32| Ok(x * 2));
        let sequence = RunnableSequence::new(first, second);

        let result = sequence.invoke(1, None).unwrap();
        assert_eq!(result, 4); // (1 + 1) * 2 = 4
    }

    #[test]
    fn test_runnable_each() {
        let runnable = RunnableLambda::new(|x: i32| Ok(x * 2));
        let each = RunnableEach::new(runnable);

        let result = each.invoke(vec![1, 2, 3], None).unwrap();
        assert_eq!(result, vec![2, 4, 6]);
    }

    #[test]
    fn test_runnable_binding() {
        let runnable = RunnableLambda::new(|x: i32| Ok(x + 1));
        let config = RunnableConfig::new().with_tags(vec!["test".to_string()]);
        let bound = RunnableBinding::new(runnable, HashMap::new(), Some(config));

        let result = bound.invoke(1, None).unwrap();
        assert_eq!(result, 2);
    }

    #[test]
    fn test_runnable_passthrough() {
        let runnable: RunnablePassthrough<i32> = RunnablePassthrough::new();
        let result = runnable.invoke(42, None).unwrap();
        assert_eq!(result, 42);
    }

    #[test]
    fn test_runnable_retry() {
        // Test that retry works with a successful function
        use crate::runnables::retry::RunnableRetry;

        let runnable = RunnableLambda::new(|x: i32| Ok(x + 1));
        let retry = RunnableRetry::with_simple(runnable, 3, false);

        let result = retry.invoke(1, None).unwrap();
        assert_eq!(result, 2);
    }

    #[test]
    fn test_addable_dict() {
        let mut dict1 = AddableDict::new();
        dict1.0.insert("a".to_string(), serde_json::json!(1));

        let mut dict2 = AddableDict::new();
        dict2.0.insert("b".to_string(), serde_json::json!(2));

        let combined = dict1 + dict2;
        assert_eq!(combined.0.get("a"), Some(&serde_json::json!(1)));
        assert_eq!(combined.0.get("b"), Some(&serde_json::json!(2)));
    }

    #[test]
    fn test_pipe() {
        let first = RunnableLambda::new(|x: i32| Ok(x + 1));
        let second = RunnableLambda::new(|x: i32| Ok(x * 2));
        let sequence = pipe(first, second);

        let result = sequence.invoke(1, None).unwrap();
        assert_eq!(result, 4);
    }

    #[tokio::test]
    async fn test_runnable_lambda_async() {
        let runnable = RunnableLambda::new(|x: i32| Ok(x + 1));
        let result = runnable.ainvoke(1, None).await.unwrap();
        assert_eq!(result, 2);
    }

    #[tokio::test]
    async fn test_runnable_sequence_async() {
        let first = RunnableLambda::new(|x: i32| Ok(x + 1));
        let second = RunnableLambda::new(|x: i32| Ok(x * 2));
        let sequence = RunnableSequence::new(first, second);

        let result = sequence.ainvoke(1, None).await.unwrap();
        assert_eq!(result, 4);
    }
}
