//! Runnable that can fallback to other Runnables if it fails.
//!
//! This module provides `RunnableWithFallbacks`, a Runnable that tries a primary
//! runnable first and falls back to alternative runnables if the primary fails.
//! This mirrors `langchain_core.runnables.fallbacks`.

use std::fmt::Debug;
use std::sync::Arc;

use async_trait::async_trait;
use futures::StreamExt;
use futures::stream::BoxStream;

use crate::error::{Error, Result};

use super::base::{DynRunnable, Runnable};
use super::config::{
    ConfigOrList, RunnableConfig, ensure_config, get_callback_manager_for_config, get_config_list,
    patch_config,
};
use super::utils::{ConfigurableFieldSpec, get_unique_config_specs};

/// A `Runnable` that can fallback to other `Runnable`s if it fails.
///
/// External APIs (e.g., APIs for a language model) may at times experience
/// degraded performance or even downtime.
///
/// In these cases, it can be useful to have a fallback `Runnable` that can be
/// used in place of the original `Runnable` (e.g., fallback to another LLM provider).
///
/// Fallbacks can be defined at the level of a single `Runnable`, or at the level
/// of a chain of `Runnable`s. Fallbacks are tried in order until one succeeds or
/// all fail.
///
/// While you can instantiate a `RunnableWithFallbacks` directly, it is usually
/// more convenient to use the `with_fallbacks` method on a `Runnable`.
///
/// # Example
///
/// ```ignore
/// use agent_chain_core::runnables::{RunnableLambda, RunnableWithFallbacks};
///
/// // Create a primary runnable that might fail
/// let primary = RunnableLambda::new(|x: i32| {
///     if x > 5 { Err(Error::other("too large")) }
///     else { Ok(x * 2) }
/// });
///
/// // Create a fallback runnable
/// let fallback = RunnableLambda::new(|x: i32| Ok(x));
///
/// // Combine them with fallbacks
/// let with_fallbacks = RunnableWithFallbacks::new(primary, vec![fallback]);
///
/// // Will use primary for x <= 5, fallback for x > 5
/// assert_eq!(with_fallbacks.invoke(3, None).unwrap(), 6);
/// assert_eq!(with_fallbacks.invoke(10, None).unwrap(), 10);
/// ```
/// Predicate for determining whether a fallback should be attempted for a given error.
pub type FallbackErrorPredicate = Arc<dyn Fn(&Error) -> bool + Send + Sync>;

/// Type alias for a function that inserts an exception into an input.
///
/// When `exception_key` is set, this function is called to create a modified
/// input that includes the exception information under the specified key.
pub type ExceptionInserter<I> = Arc<dyn Fn(&I, &str, &Error) -> I + Send + Sync>;

pub struct RunnableWithFallbacks<I, O>
where
    I: Send + Sync + Clone + Debug + 'static,
    O: Send + Sync + Clone + Debug + 'static,
{
    /// The `Runnable` to run first.
    pub runnable: DynRunnable<I, O>,
    /// A sequence of fallbacks to try.
    pub fallbacks: Vec<DynRunnable<I, O>>,
    /// Predicate to determine which errors should trigger a fallback.
    /// If None, all errors trigger fallback (equivalent to Python's default `(Exception,)`).
    pub error_predicate: Option<FallbackErrorPredicate>,
    /// If set, handled exceptions will be passed to fallbacks as part of the input
    /// under the specified key. The input must be a dict-like type.
    pub exception_key: Option<String>,
    /// Function to insert an exception into the input when `exception_key` is set.
    exception_inserter: Option<ExceptionInserter<I>>,
    /// Optional name for this runnable.
    name: Option<String>,
}

impl<I, O> Debug for RunnableWithFallbacks<I, O>
where
    I: Send + Sync + Clone + Debug + 'static,
    O: Send + Sync + Clone + Debug + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RunnableWithFallbacks")
            .field("runnable", &"<runnable>")
            .field("fallbacks_count", &self.fallbacks.len())
            .field(
                "error_predicate",
                &self.error_predicate.as_ref().map(|_| "..."),
            )
            .field("exception_key", &self.exception_key)
            .field("name", &self.name)
            .finish()
    }
}

impl<I, O> RunnableWithFallbacks<I, O>
where
    I: Send + Sync + Clone + Debug + 'static,
    O: Send + Sync + Clone + Debug + 'static,
{
    /// Create a new RunnableWithFallbacks.
    ///
    /// # Arguments
    /// * `runnable` - The primary runnable to try first
    /// * `fallbacks` - A list of fallback runnables to try if the primary fails
    pub fn new<R>(runnable: R, fallbacks: Vec<DynRunnable<I, O>>) -> Self
    where
        R: Runnable<Input = I, Output = O> + Send + Sync + 'static,
    {
        Self {
            runnable: Arc::new(runnable),
            fallbacks,
            error_predicate: None,
            exception_key: None,
            exception_inserter: None,
            name: None,
        }
    }

    /// Create a new RunnableWithFallbacks from a DynRunnable.
    pub fn from_dyn(runnable: DynRunnable<I, O>, fallbacks: Vec<DynRunnable<I, O>>) -> Self {
        Self {
            runnable,
            fallbacks,
            error_predicate: None,
            exception_key: None,
            exception_inserter: None,
            name: None,
        }
    }

    /// Set a predicate to determine which errors should trigger fallback.
    ///
    /// If the predicate returns true for an error, fallback is attempted.
    /// If it returns false, the error is raised immediately.
    /// If no predicate is set, all errors trigger fallback.
    pub fn with_error_predicate(mut self, predicate: FallbackErrorPredicate) -> Self {
        self.error_predicate = Some(predicate);
        self
    }

    /// Set the exception key with an inserter function.
    ///
    /// When set, handled exceptions are passed to fallback runnables as part of
    /// the input under the specified key. The `inserter` function defines how
    /// to create a new input with the exception value inserted.
    pub fn with_exception_key(
        mut self,
        key: impl Into<String>,
        inserter: ExceptionInserter<I>,
    ) -> Self {
        self.exception_key = Some(key.into());
        self.exception_inserter = Some(inserter);
        self
    }

    /// Set the name of this runnable.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Get an iterator over all runnables (primary + fallbacks).
    pub fn runnables(&self) -> impl Iterator<Item = &DynRunnable<I, O>> {
        std::iter::once(&self.runnable).chain(self.fallbacks.iter())
    }

    /// Get the config specs from all runnables.
    pub fn config_specs(&self) -> Result<Vec<ConfigurableFieldSpec>> {
        let specs: Vec<ConfigurableFieldSpec> = self
            .runnables()
            .flat_map(|_r| {
                // In a full implementation, we would get config specs from each runnable
                // For now, return empty as the trait doesn't expose config_specs
                Vec::<ConfigurableFieldSpec>::new()
            })
            .collect();

        get_unique_config_specs(specs).map_err(Error::other)
    }

    /// Check if an error should trigger a fallback.
    fn should_fallback(&self, error: &Error) -> bool {
        match &self.error_predicate {
            Some(predicate) => predicate(error),
            None => true,
        }
    }
}

#[async_trait]
impl<I, O> Runnable for RunnableWithFallbacks<I, O>
where
    I: Send + Sync + Clone + Debug + 'static,
    O: Send + Sync + Clone + Debug + 'static,
{
    type Input = I;
    type Output = O;

    fn name(&self) -> Option<String> {
        self.name.clone()
    }

    fn get_input_schema(&self, config: Option<&RunnableConfig>) -> serde_json::Value {
        self.runnable.get_input_schema(config)
    }

    fn get_output_schema(&self, config: Option<&RunnableConfig>) -> serde_json::Value {
        self.runnable.get_output_schema(config)
    }

    fn invoke(&self, input: Self::Input, config: Option<RunnableConfig>) -> Result<Self::Output> {
        let config = ensure_config(config);
        let callback_manager = get_callback_manager_for_config(&config);

        // Start the root run
        let run_manager = callback_manager
            .on_chain_start()
            .serialized(&std::collections::HashMap::new())
            .inputs(&std::collections::HashMap::new())
            .maybe_run_id(config.run_id)
            .call();

        let mut first_error: Option<Error> = None;
        let mut last_error: Option<Error> = None;
        let mut current_input = input;

        for runnable in self.runnables() {
            // If exception_key is set, inject the last error into the input
            if let (Some(key), Some(inserter), Some(err)) =
                (&self.exception_key, &self.exception_inserter, &last_error)
            {
                current_input = inserter(&current_input, key, err);
            }

            let child_config = patch_config(
                Some(config.clone()),
                Some(run_manager.get_child(None)),
                None,
                None,
                None,
                None,
            );

            match runnable.invoke(current_input.clone(), Some(child_config)) {
                Ok(output) => {
                    run_manager.on_chain_end(&std::collections::HashMap::new());
                    return Ok(output);
                }
                Err(e) => {
                    if self.should_fallback(&e) {
                        if first_error.is_none() {
                            first_error = Some(Error::other(e.to_string()));
                        }
                        last_error = Some(e);
                    } else {
                        run_manager.on_chain_error(&e);
                        return Err(e);
                    }
                }
            }
        }

        let error =
            first_error.unwrap_or_else(|| Error::other("No error stored at end of fallbacks."));
        run_manager.on_chain_error(&error);
        Err(error)
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

        let mut first_error: Option<Error> = None;
        let mut last_error: Option<Error> = None;
        let mut current_input = input;

        for runnable in self.runnables() {
            // If exception_key is set, inject the last error into the input
            if let (Some(key), Some(inserter), Some(err)) =
                (&self.exception_key, &self.exception_inserter, &last_error)
            {
                current_input = inserter(&current_input, key, err);
            }

            match runnable
                .ainvoke(current_input.clone(), Some(config.clone()))
                .await
            {
                Ok(output) => {
                    return Ok(output);
                }
                Err(e) => {
                    if self.should_fallback(&e) {
                        if first_error.is_none() {
                            first_error = Some(Error::other(e.to_string()));
                        }
                        last_error = Some(e);
                    } else {
                        return Err(e);
                    }
                }
            }
        }

        Err(first_error.unwrap_or_else(|| Error::other("No error stored at end of fallbacks.")))
    }

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
        let n = inputs.len();

        // Track which inputs still need to be processed
        let mut to_return: Vec<Option<Result<Self::Output>>> = (0..n).map(|_| None).collect();
        let mut run_again: Vec<(usize, Self::Input)> = inputs.into_iter().enumerate().collect();
        let mut handled_exception_indices: Vec<usize> = Vec::new();
        let mut first_to_raise: Option<Error> = None;

        for runnable in self.runnables() {
            if run_again.is_empty() {
                break;
            }

            // Get inputs and configs for items that need to be run again
            let batch_inputs: Vec<Self::Input> =
                run_again.iter().map(|(_, inp)| inp.clone()).collect();
            let batch_configs: Vec<RunnableConfig> =
                run_again.iter().map(|(i, _)| configs[*i].clone()).collect();

            let outputs = runnable.batch(
                batch_inputs,
                Some(ConfigOrList::List(batch_configs)),
                true, // Always return exceptions to handle them ourselves
            );

            let mut next_run_again = Vec::new();

            for ((i, input), output) in run_again.iter().zip(outputs) {
                match output {
                    Ok(out) => {
                        to_return[*i] = Some(Ok(out));
                        handled_exception_indices.retain(|&idx| idx != *i);
                    }
                    Err(e) => {
                        if self.should_fallback(&e) {
                            if !handled_exception_indices.contains(i) {
                                handled_exception_indices.push(*i);
                            }
                            // If exception_key is set, inject exception into the input
                            let next_input = if let (Some(key), Some(inserter)) =
                                (&self.exception_key, &self.exception_inserter)
                            {
                                inserter(input, key, &e)
                            } else {
                                input.clone()
                            };
                            to_return[*i] = Some(Err(e));
                            next_run_again.push((*i, next_input));
                        } else if return_exceptions {
                            to_return[*i] = Some(Err(e));
                        } else if first_to_raise.is_none() {
                            first_to_raise = Some(e);
                        }
                    }
                }
            }

            if first_to_raise.is_some() {
                // Return early with the first non-fallback error
                let mut results = Vec::with_capacity(to_return.len());
                let mut error_consumed = false;
                for opt in to_return {
                    match opt {
                        Some(result) => results.push(result),
                        None => {
                            if !error_consumed {
                                results.push(Err(first_to_raise
                                    .take()
                                    .expect("first_to_raise set when errors exist")));
                                error_consumed = true;
                            } else {
                                results.push(Err(Error::other("Batch aborted due to error")));
                            }
                        }
                    }
                }
                return results;
            }

            run_again = next_run_again;
        }

        // All fallbacks exhausted - errors are already stored in to_return
        if !return_exceptions && !handled_exception_indices.is_empty() {
            // Return all results as-is, errors from the last fallback attempt are stored
        }

        // Return results, filling in errors for items that never had any result
        to_return
            .into_iter()
            .map(|opt| opt.unwrap_or_else(|| Err(Error::other("No result for index"))))
            .collect()
    }

    async fn abatch(
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
        let n = inputs.len();

        // Track which inputs still need to be processed
        let mut to_return: Vec<Option<Result<Self::Output>>> = (0..n).map(|_| None).collect();
        let mut run_again: Vec<(usize, Self::Input)> = inputs.into_iter().enumerate().collect();
        let mut handled_exception_indices: Vec<usize> = Vec::new();
        let mut first_to_raise: Option<Error> = None;

        for runnable in self.runnables() {
            if run_again.is_empty() {
                break;
            }

            // Get inputs and configs for items that need to be run again
            let batch_inputs: Vec<Self::Input> =
                run_again.iter().map(|(_, inp)| inp.clone()).collect();
            let batch_configs: Vec<RunnableConfig> =
                run_again.iter().map(|(i, _)| configs[*i].clone()).collect();

            let outputs = runnable
                .abatch(
                    batch_inputs,
                    Some(ConfigOrList::List(batch_configs)),
                    true, // Always return exceptions to handle them ourselves
                )
                .await;

            let mut next_run_again = Vec::new();

            for ((i, input), output) in run_again.iter().zip(outputs) {
                match output {
                    Ok(out) => {
                        to_return[*i] = Some(Ok(out));
                        handled_exception_indices.retain(|&idx| idx != *i);
                    }
                    Err(e) => {
                        if self.should_fallback(&e) {
                            if !handled_exception_indices.contains(i) {
                                handled_exception_indices.push(*i);
                            }
                            // If exception_key is set, inject exception into the input
                            let next_input = if let (Some(key), Some(inserter)) =
                                (&self.exception_key, &self.exception_inserter)
                            {
                                inserter(input, key, &e)
                            } else {
                                input.clone()
                            };
                            to_return[*i] = Some(Err(e));
                            next_run_again.push((*i, next_input));
                        } else if return_exceptions {
                            to_return[*i] = Some(Err(e));
                        } else if first_to_raise.is_none() {
                            first_to_raise = Some(e);
                        }
                    }
                }
            }

            if first_to_raise.is_some() {
                // Return early with the first non-fallback error
                let mut results = Vec::with_capacity(to_return.len());
                let mut error_consumed = false;
                for opt in to_return {
                    match opt {
                        Some(result) => results.push(result),
                        None => {
                            if !error_consumed {
                                results.push(Err(first_to_raise
                                    .take()
                                    .expect("first_to_raise set when errors exist")));
                                error_consumed = true;
                            } else {
                                results.push(Err(Error::other("Batch aborted due to error")));
                            }
                        }
                    }
                }
                return results;
            }

            run_again = next_run_again;
        }

        // All fallbacks exhausted - errors are already stored in to_return
        if !return_exceptions && !handled_exception_indices.is_empty() {
            // Return all results as-is, errors from the last fallback attempt are stored
        }

        // Return results, filling in errors for items that never had any result
        to_return
            .into_iter()
            .map(|opt| opt.unwrap_or_else(|| Err(Error::other("No result for index"))))
            .collect()
    }

    fn stream(
        &self,
        input: Self::Input,
        config: Option<RunnableConfig>,
    ) -> BoxStream<'_, Result<Self::Output>> {
        let config = ensure_config(config);

        Box::pin(async_stream::stream! {
            let mut first_error: Option<Error> = None;
            let mut last_error: Option<Error> = None;
            let mut current_input = input;

            for runnable in self.runnables() {
                // If exception_key is set, inject the last error into the input
                if let (Some(key), Some(inserter), Some(err)) =
                    (&self.exception_key, &self.exception_inserter, &last_error)
                {
                    current_input = inserter(&current_input, key, err);
                }

                // Try to get the first chunk from this runnable's stream
                let mut stream = runnable.stream(current_input.clone(), Some(config.clone()));

                match stream.next().await {
                    Some(Ok(chunk)) => {
                        // Success! Yield this chunk and continue streaming
                        yield Ok(chunk);

                        // Stream remaining chunks
                        while let Some(result) = stream.next().await {
                            yield result;
                        }
                        return;
                    }
                    Some(Err(e)) => {
                        if self.should_fallback(&e) {
                            if first_error.is_none() {
                                first_error = Some(Error::other(e.to_string()));
                            }
                            last_error = Some(e);
                        } else {
                            yield Err(e);
                            return;
                        }
                    }
                    None => {
                        // Empty stream, try next fallback
                        if first_error.is_none() {
                            first_error = Some(Error::other("Empty stream from runnable"));
                        }
                    }
                }
            }

            // All fallbacks exhausted
            yield Err(first_error.unwrap_or_else(|| Error::other("No error stored at end of fallbacks.")));
        })
    }

    fn astream(
        &self,
        input: Self::Input,
        config: Option<RunnableConfig>,
    ) -> BoxStream<'_, Result<Self::Output>>
    where
        Self: 'static,
    {
        let config = ensure_config(config);

        Box::pin(async_stream::stream! {
            let mut first_error: Option<Error> = None;
            let mut last_error: Option<Error> = None;
            let mut current_input = input;

            for runnable in self.runnables() {
                // If exception_key is set, inject the last error into the input
                if let (Some(key), Some(inserter), Some(err)) =
                    (&self.exception_key, &self.exception_inserter, &last_error)
                {
                    current_input = inserter(&current_input, key, err);
                }

                // Try to get the first chunk from this runnable's stream
                let mut stream = runnable.astream(current_input.clone(), Some(config.clone()));

                match stream.next().await {
                    Some(Ok(chunk)) => {
                        // Success! Yield this chunk and continue streaming
                        yield Ok(chunk);

                        // Stream remaining chunks
                        while let Some(result) = stream.next().await {
                            yield result;
                        }
                        return;
                    }
                    Some(Err(e)) => {
                        if self.should_fallback(&e) {
                            if first_error.is_none() {
                                first_error = Some(Error::other(e.to_string()));
                            }
                            last_error = Some(e);
                        } else {
                            yield Err(e);
                            return;
                        }
                    }
                    None => {
                        // Empty stream, try next fallback
                        if first_error.is_none() {
                            first_error = Some(Error::other("Empty stream from runnable"));
                        }
                    }
                }
            }

            // All fallbacks exhausted
            yield Err(first_error.unwrap_or_else(|| Error::other("No error stored at end of fallbacks.")));
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runnables::base::RunnableLambda;

    #[test]
    fn test_fallback_on_error() {
        let primary =
            RunnableLambda::new(|_x: i32| -> Result<i32> { Err(Error::other("primary failed")) });

        let fallback = RunnableLambda::new(|x: i32| -> Result<i32> { Ok(x * 2) });

        let with_fallbacks = RunnableWithFallbacks::new(primary, vec![Arc::new(fallback)]);

        let result = with_fallbacks.invoke(5, None).unwrap();
        assert_eq!(result, 10);
    }

    #[test]
    fn test_primary_succeeds() {
        let primary = RunnableLambda::new(|x: i32| -> Result<i32> { Ok(x + 1) });

        let fallback = RunnableLambda::new(|x: i32| -> Result<i32> { Ok(x * 2) });

        let with_fallbacks = RunnableWithFallbacks::new(primary, vec![Arc::new(fallback)]);

        let result = with_fallbacks.invoke(5, None).unwrap();
        assert_eq!(result, 6); // Primary succeeded, not fallback
    }

    #[test]
    fn test_all_fail() {
        let primary =
            RunnableLambda::new(|_x: i32| -> Result<i32> { Err(Error::other("primary failed")) });

        let fallback =
            RunnableLambda::new(|_x: i32| -> Result<i32> { Err(Error::other("fallback failed")) });

        let with_fallbacks = RunnableWithFallbacks::new(primary, vec![Arc::new(fallback)]);

        let result = with_fallbacks.invoke(5, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_multiple_fallbacks() {
        let primary =
            RunnableLambda::new(|_x: i32| -> Result<i32> { Err(Error::other("primary failed")) });

        let fallback1 =
            RunnableLambda::new(|_x: i32| -> Result<i32> { Err(Error::other("fallback1 failed")) });

        let fallback2 = RunnableLambda::new(|x: i32| -> Result<i32> { Ok(x * 3) });

        let with_fallbacks =
            RunnableWithFallbacks::new(primary, vec![Arc::new(fallback1), Arc::new(fallback2)]);

        let result = with_fallbacks.invoke(5, None).unwrap();
        assert_eq!(result, 15); // Second fallback succeeded
    }

    #[test]
    fn test_with_fallbacks_ext() {
        let primary =
            RunnableLambda::new(|_x: i32| -> Result<i32> { Err(Error::other("primary failed")) });

        let fallback = RunnableLambda::new(|x: i32| -> Result<i32> { Ok(x * 2) });

        let with_fallbacks = primary.with_fallbacks(vec![Arc::new(fallback)]);

        let result = with_fallbacks.invoke(5, None).unwrap();
        assert_eq!(result, 10);
    }

    #[tokio::test]
    async fn test_fallback_async() {
        let primary =
            RunnableLambda::new(|_x: i32| -> Result<i32> { Err(Error::other("primary failed")) });

        let fallback = RunnableLambda::new(|x: i32| -> Result<i32> { Ok(x * 2) });

        let with_fallbacks = RunnableWithFallbacks::new(primary, vec![Arc::new(fallback)]);

        let result = with_fallbacks.ainvoke(5, None).await.unwrap();
        assert_eq!(result, 10);
    }

    #[test]
    fn test_batch_fallback() {
        let primary = RunnableLambda::new(|x: i32| -> Result<i32> {
            if x > 5 {
                Err(Error::other("too large"))
            } else {
                Ok(x + 1)
            }
        });

        let fallback = RunnableLambda::new(|x: i32| -> Result<i32> { Ok(x * 2) });

        let with_fallbacks = RunnableWithFallbacks::new(primary, vec![Arc::new(fallback)]);

        let results = with_fallbacks.batch(vec![3, 10, 5], None, false);

        // 3 -> primary succeeds -> 4
        // 10 -> primary fails, fallback succeeds -> 20
        // 5 -> primary succeeds -> 6
        assert_eq!(results[0].as_ref().unwrap(), &4);
        assert_eq!(results[1].as_ref().unwrap(), &20);
        assert_eq!(results[2].as_ref().unwrap(), &6);
    }

    #[tokio::test]
    async fn test_stream_fallback() {
        use futures::StreamExt;

        let primary =
            RunnableLambda::new(|_x: i32| -> Result<i32> { Err(Error::other("primary failed")) });

        let fallback = RunnableLambda::new(|x: i32| -> Result<i32> { Ok(x * 2) });

        let with_fallbacks = RunnableWithFallbacks::new(primary, vec![Arc::new(fallback)]);

        let mut stream = with_fallbacks.stream(5, None);
        let result = stream.next().await.unwrap().unwrap();
        assert_eq!(result, 10);
    }

    #[test]
    fn test_runnables_iterator() {
        let primary = RunnableLambda::new(|x: i32| -> Result<i32> { Ok(x) });
        let fallback1 = RunnableLambda::new(|x: i32| -> Result<i32> { Ok(x) });
        let fallback2 = RunnableLambda::new(|x: i32| -> Result<i32> { Ok(x) });

        let with_fallbacks =
            RunnableWithFallbacks::new(primary, vec![Arc::new(fallback1), Arc::new(fallback2)]);

        let count = with_fallbacks.runnables().count();
        assert_eq!(count, 3); // primary + 2 fallbacks
    }
}
