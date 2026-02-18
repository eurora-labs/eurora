//! Runnable that retries a Runnable if it fails.
//!
//! This module provides `RunnableRetry`, a Runnable that wraps another Runnable
//! and retries it on failure with configurable retry logic.
//! This mirrors `langchain_core.runnables.retry`.

use std::fmt::Debug;
use std::time::Duration;

use async_trait::async_trait;
use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::callbacks::CallbackManagerForChainRun;
use crate::error::{Error, Result};

use super::base::Runnable;
use super::config::{
    ConfigOrList, RunnableConfig, ensure_config, get_callback_manager_for_config, get_config_list,
    patch_config,
};

/// Parameters for exponential backoff with jitter.
///
/// These parameters control the wait time between retry attempts.
/// The wait time is calculated as:
/// `min(max, initial * exp_base^attempt) + random(0, jitter)`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExponentialJitterParams {
    /// Initial wait time in seconds. Default: 1.0
    #[serde(default = "default_initial")]
    pub initial: f64,

    /// Maximum wait time in seconds. Default: 60.0
    #[serde(default = "default_max")]
    pub max: f64,

    /// Base for exponential backoff. Default: 2.0
    #[serde(default = "default_exp_base")]
    pub exp_base: f64,

    /// Maximum jitter in seconds to add. Default: 1.0
    #[serde(default = "default_jitter")]
    pub jitter: f64,
}

fn default_initial() -> f64 {
    1.0
}

fn default_max() -> f64 {
    60.0
}

fn default_exp_base() -> f64 {
    2.0
}

fn default_jitter() -> f64 {
    1.0
}

impl Default for ExponentialJitterParams {
    fn default() -> Self {
        Self {
            initial: 1.0,
            max: 60.0,
            exp_base: 2.0,
            jitter: 1.0,
        }
    }
}

impl ExponentialJitterParams {
    /// Create new exponential jitter parameters with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the initial wait time.
    pub fn with_initial(mut self, initial: f64) -> Self {
        self.initial = initial;
        self
    }

    /// Set the maximum wait time.
    pub fn with_max(mut self, max: f64) -> Self {
        self.max = max;
        self
    }

    /// Set the exponential base.
    pub fn with_exp_base(mut self, exp_base: f64) -> Self {
        self.exp_base = exp_base;
        self
    }

    /// Set the jitter value.
    pub fn with_jitter(mut self, jitter: f64) -> Self {
        self.jitter = jitter;
        self
    }

    /// Calculate the wait time for a given attempt number (1-indexed).
    pub fn calculate_wait(&self, attempt: usize) -> Duration {
        let exp_wait = self.initial * self.exp_base.powi(attempt.saturating_sub(1) as i32);
        let capped_wait = exp_wait.min(self.max);
        let jitter_amount = if self.jitter > 0.0 {
            let mut rng = rand::rng();
            rng.random_range(0.0..self.jitter)
        } else {
            0.0
        };
        let total_seconds = capped_wait + jitter_amount;
        Duration::from_secs_f64(total_seconds)
    }
}

/// State of a retry attempt.
#[derive(Debug, Clone)]
pub struct RetryCallState {
    /// The current attempt number (1-indexed).
    pub attempt_number: usize,
    /// Whether the attempt succeeded.
    pub succeeded: bool,
}

impl RetryCallState {
    fn new(attempt_number: usize) -> Self {
        Self {
            attempt_number,
            succeeded: false,
        }
    }
}

/// Error type predicate for retry logic.
///
/// This enum allows specifying which error types should trigger a retry.
#[derive(Debug, Clone, Default)]
pub enum RetryErrorPredicate {
    /// Retry on all errors (default).
    #[default]
    All,
    /// Retry only on HTTP/API errors.
    HttpErrors,
    /// Retry only on specific error variants using a custom predicate.
    Custom(fn(&Error) -> bool),
}

impl RetryErrorPredicate {
    /// Check if the given error should trigger a retry.
    pub fn should_retry(&self, error: &Error) -> bool {
        match self {
            RetryErrorPredicate::All => true,
            RetryErrorPredicate::HttpErrors => matches!(error, Error::Http(_) | Error::Api { .. }),
            RetryErrorPredicate::Custom(predicate) => predicate(error),
        }
    }
}

/// Configuration for creating a RunnableRetry.
#[derive(Debug, Clone)]
pub struct RunnableRetryConfig {
    /// The exception types to retry on. By default all exceptions are retried.
    pub retry_predicate: RetryErrorPredicate,

    /// Whether to add jitter to the exponential backoff.
    pub wait_exponential_jitter: bool,

    /// Parameters for exponential backoff with jitter.
    pub exponential_jitter_params: Option<ExponentialJitterParams>,

    /// The maximum number of attempts to retry the Runnable.
    pub max_attempt_number: usize,
}

impl Default for RunnableRetryConfig {
    fn default() -> Self {
        Self {
            retry_predicate: RetryErrorPredicate::All,
            wait_exponential_jitter: true,
            exponential_jitter_params: None,
            max_attempt_number: 3,
        }
    }
}

impl RunnableRetryConfig {
    /// Create a new retry configuration with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the retry predicate.
    pub fn with_retry_predicate(mut self, predicate: RetryErrorPredicate) -> Self {
        self.retry_predicate = predicate;
        self
    }

    /// Set whether to use exponential jitter.
    pub fn with_wait_exponential_jitter(mut self, wait: bool) -> Self {
        self.wait_exponential_jitter = wait;
        self
    }

    /// Set the exponential jitter parameters.
    pub fn with_exponential_jitter_params(mut self, params: ExponentialJitterParams) -> Self {
        self.exponential_jitter_params = Some(params);
        self
    }

    /// Set the maximum number of attempts.
    pub fn with_max_attempt_number(mut self, max: usize) -> Self {
        self.max_attempt_number = max;
        self
    }
}

/// A Runnable that retries on failure.
///
/// `RunnableRetry` wraps another `Runnable` and retries it if it fails.
/// This is particularly useful for network calls that may fail due to transient errors.
///
/// # Example
///
/// ```ignore
/// use agent_chain_core::runnables::{RunnableLambda, RunnableRetry, RunnableRetryConfig};
///
/// // Create a runnable that might fail
/// let runnable = RunnableLambda::new(|x: i32| {
///     // Simulated unreliable operation
///     if x > 0 { Ok(x * 2) }
///     else { Err(Error::other("negative input")) }
/// });
///
/// // Wrap it with retry logic
/// let config = RunnableRetryConfig::new()
///     .with_max_attempt_number(3)
///     .with_wait_exponential_jitter(true);
///
/// let with_retry = RunnableRetry::new(runnable, config);
/// ```
pub struct RunnableRetry<R>
where
    R: Runnable,
{
    /// The wrapped runnable.
    bound: R,

    /// Retry configuration.
    config: RunnableRetryConfig,
}

impl<R> Debug for RunnableRetry<R>
where
    R: Runnable,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RunnableRetry")
            .field("bound", &self.bound)
            .field("max_attempt_number", &self.config.max_attempt_number)
            .field(
                "wait_exponential_jitter",
                &self.config.wait_exponential_jitter,
            )
            .finish()
    }
}

impl<R> RunnableRetry<R>
where
    R: Runnable,
{
    /// Create a new RunnableRetry with the given configuration.
    pub fn new(bound: R, config: RunnableRetryConfig) -> Self {
        Self { bound, config }
    }

    /// Create a new RunnableRetry with simple parameters.
    pub fn with_simple(bound: R, max_attempts: usize, wait_exponential_jitter: bool) -> Self {
        Self {
            bound,
            config: RunnableRetryConfig {
                max_attempt_number: max_attempts,
                wait_exponential_jitter,
                ..Default::default()
            },
        }
    }

    /// Get the exponential jitter parameters, using defaults if not set.
    fn get_jitter_params(&self) -> ExponentialJitterParams {
        self.config
            .exponential_jitter_params
            .clone()
            .unwrap_or_default()
    }

    /// Check if the error should trigger a retry.
    fn should_retry(&self, error: &Error) -> bool {
        self.config.retry_predicate.should_retry(error)
    }

    /// Calculate the wait time for a given attempt.
    fn calculate_wait(&self, attempt: usize) -> Duration {
        if self.config.wait_exponential_jitter {
            self.get_jitter_params().calculate_wait(attempt)
        } else {
            Duration::ZERO
        }
    }

    /// Patch the config for a retry attempt.
    fn patch_config_for_retry(
        config: &RunnableConfig,
        run_manager: &CallbackManagerForChainRun,
        retry_state: &RetryCallState,
    ) -> RunnableConfig {
        let tag = if retry_state.attempt_number > 1 {
            Some(format!("retry:attempt:{}", retry_state.attempt_number))
        } else {
            None
        };

        patch_config(
            Some(config.clone()),
            Some(run_manager.get_child(tag.as_deref())),
            None,
            None,
            None,
            None,
        )
    }

    /// Patch configs for batch retry.
    fn patch_config_list_for_retry(
        configs: &[RunnableConfig],
        run_managers: &[CallbackManagerForChainRun],
        retry_state: &RetryCallState,
    ) -> Vec<RunnableConfig> {
        configs
            .iter()
            .zip(run_managers.iter())
            .map(|(config, run_manager)| {
                Self::patch_config_for_retry(config, run_manager, retry_state)
            })
            .collect()
    }
}

#[async_trait]
impl<R> Runnable for RunnableRetry<R>
where
    R: Runnable + 'static,
{
    type Input = R::Input;
    type Output = R::Output;

    fn name(&self) -> Option<String> {
        self.bound.name()
    }

    fn get_input_schema(&self, config: Option<&RunnableConfig>) -> serde_json::Value {
        self.bound.get_input_schema(config)
    }

    fn get_output_schema(&self, config: Option<&RunnableConfig>) -> serde_json::Value {
        self.bound.get_output_schema(config)
    }

    fn invoke(&self, input: Self::Input, config: Option<RunnableConfig>) -> Result<Self::Output> {
        let config = ensure_config(config);
        let callback_manager = get_callback_manager_for_config(&config);

        let run_manager = callback_manager
            .on_chain_start()
            .serialized(&std::collections::HashMap::new())
            .inputs(&std::collections::HashMap::new())
            .maybe_run_id(config.run_id)
            .call();

        let mut last_error = None;

        for attempt in 1..=self.config.max_attempt_number {
            let retry_state = RetryCallState::new(attempt);
            let patched_config = Self::patch_config_for_retry(&config, &run_manager, &retry_state);

            match self.bound.invoke(input.clone(), Some(patched_config)) {
                Ok(output) => {
                    run_manager.on_chain_end(&std::collections::HashMap::new());
                    return Ok(output);
                }
                Err(e) => {
                    if !self.should_retry(&e) || attempt == self.config.max_attempt_number {
                        run_manager.on_chain_error(&e);
                        return Err(e);
                    }
                    last_error = Some(e);

                    if self.config.wait_exponential_jitter
                        && attempt < self.config.max_attempt_number
                    {
                        let wait = self.calculate_wait(attempt);
                        std::thread::sleep(wait);
                    }
                }
            }
        }

        let error = last_error.unwrap_or_else(|| Error::other("Max retries exceeded"));
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
        let callback_manager = get_callback_manager_for_config(&config);

        let run_manager = callback_manager
            .on_chain_start()
            .serialized(&std::collections::HashMap::new())
            .inputs(&std::collections::HashMap::new())
            .maybe_run_id(config.run_id)
            .call();

        let mut last_error = None;

        for attempt in 1..=self.config.max_attempt_number {
            let retry_state = RetryCallState::new(attempt);
            let patched_config = Self::patch_config_for_retry(&config, &run_manager, &retry_state);

            match self
                .bound
                .ainvoke(input.clone(), Some(patched_config))
                .await
            {
                Ok(output) => {
                    run_manager.on_chain_end(&std::collections::HashMap::new());
                    return Ok(output);
                }
                Err(e) => {
                    if !self.should_retry(&e) || attempt == self.config.max_attempt_number {
                        run_manager.on_chain_error(&e);
                        return Err(e);
                    }
                    last_error = Some(e);

                    if self.config.wait_exponential_jitter
                        && attempt < self.config.max_attempt_number
                    {
                        let wait = self.calculate_wait(attempt);
                        tokio::time::sleep(wait).await;
                    }
                }
            }
        }

        let error = last_error.unwrap_or_else(|| Error::other("Max retries exceeded"));
        run_manager.on_chain_error(&error);
        Err(error)
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

        let run_managers: Vec<CallbackManagerForChainRun> = configs
            .iter()
            .map(|config| {
                let callback_manager = get_callback_manager_for_config(config);
                callback_manager
                    .on_chain_start()
                    .serialized(&std::collections::HashMap::new())
                    .inputs(&std::collections::HashMap::new())
                    .maybe_run_id(config.run_id)
                    .call()
            })
            .collect();

        let mut results: Vec<Option<Result<Self::Output>>> = (0..n).map(|_| None).collect();

        let mut remaining: Vec<usize> = (0..n).collect();

        for attempt in 1..=self.config.max_attempt_number {
            if remaining.is_empty() {
                break;
            }

            let retry_state = RetryCallState::new(attempt);

            let pending_inputs: Vec<Self::Input> =
                remaining.iter().map(|&i| inputs[i].clone()).collect();
            let pending_configs: Vec<RunnableConfig> =
                remaining.iter().map(|&i| configs[i].clone()).collect();
            let pending_managers: Vec<CallbackManagerForChainRun> =
                remaining.iter().map(|&i| run_managers[i].clone()).collect();

            let patched_configs = Self::patch_config_list_for_retry(
                &pending_configs,
                &pending_managers,
                &retry_state,
            );

            let batch_results = self.bound.batch(
                pending_inputs,
                Some(ConfigOrList::List(patched_configs)),
                true, // Always return exceptions to handle ourselves
            );

            let mut next_remaining = Vec::new();
            let mut first_non_retryable_error: Option<Error> = None;

            for (offset, result) in batch_results.into_iter().enumerate() {
                let orig_idx = remaining[offset];

                match result {
                    Ok(output) => {
                        results[orig_idx] = Some(Ok(output));
                    }
                    Err(e) => {
                        if self.should_retry(&e) && attempt < self.config.max_attempt_number {
                            results[orig_idx] = Some(Err(e));
                            next_remaining.push(orig_idx);
                        } else if !self.should_retry(&e) && !return_exceptions {
                            if first_non_retryable_error.is_none() {
                                first_non_retryable_error = Some(e);
                            }
                            results[orig_idx] = Some(Err(Error::other("Batch aborted")));
                        } else {
                            results[orig_idx] = Some(Err(e));
                        }
                    }
                }
            }

            if first_non_retryable_error.is_some() && !return_exceptions {
                for result in results.iter_mut().take(n) {
                    if result.is_none() {
                        *result = Some(Err(Error::other("Batch aborted due to error")));
                    }
                }
                break;
            }

            remaining = next_remaining;

            if !remaining.is_empty()
                && self.config.wait_exponential_jitter
                && attempt < self.config.max_attempt_number
            {
                let wait = self.calculate_wait(attempt);
                std::thread::sleep(wait);
            }
        }

        results
            .into_iter()
            .map(|opt| opt.unwrap_or_else(|| Err(Error::other("No result"))))
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

        let run_managers: Vec<CallbackManagerForChainRun> = configs
            .iter()
            .map(|config| {
                let callback_manager = get_callback_manager_for_config(config);
                callback_manager
                    .on_chain_start()
                    .serialized(&std::collections::HashMap::new())
                    .inputs(&std::collections::HashMap::new())
                    .maybe_run_id(config.run_id)
                    .call()
            })
            .collect();

        let mut results: Vec<Option<Result<Self::Output>>> = (0..n).map(|_| None).collect();

        let mut remaining: Vec<usize> = (0..n).collect();

        for attempt in 1..=self.config.max_attempt_number {
            if remaining.is_empty() {
                break;
            }

            let retry_state = RetryCallState::new(attempt);

            let pending_inputs: Vec<Self::Input> =
                remaining.iter().map(|&i| inputs[i].clone()).collect();
            let pending_configs: Vec<RunnableConfig> =
                remaining.iter().map(|&i| configs[i].clone()).collect();
            let pending_managers: Vec<CallbackManagerForChainRun> =
                remaining.iter().map(|&i| run_managers[i].clone()).collect();

            let patched_configs = Self::patch_config_list_for_retry(
                &pending_configs,
                &pending_managers,
                &retry_state,
            );

            let batch_results = self
                .bound
                .abatch(
                    pending_inputs,
                    Some(ConfigOrList::List(patched_configs)),
                    true, // Always return exceptions to handle ourselves
                )
                .await;

            let mut next_remaining = Vec::new();
            let mut first_non_retryable_error: Option<Error> = None;

            for (offset, result) in batch_results.into_iter().enumerate() {
                let orig_idx = remaining[offset];

                match result {
                    Ok(output) => {
                        results[orig_idx] = Some(Ok(output));
                    }
                    Err(e) => {
                        if self.should_retry(&e) && attempt < self.config.max_attempt_number {
                            results[orig_idx] = Some(Err(e));
                            next_remaining.push(orig_idx);
                        } else if !self.should_retry(&e) && !return_exceptions {
                            if first_non_retryable_error.is_none() {
                                first_non_retryable_error = Some(e);
                            }
                            results[orig_idx] = Some(Err(Error::other("Batch aborted")));
                        } else {
                            results[orig_idx] = Some(Err(e));
                        }
                    }
                }
            }

            if first_non_retryable_error.is_some() && !return_exceptions {
                for result in results.iter_mut().take(n) {
                    if result.is_none() {
                        *result = Some(Err(Error::other("Batch aborted due to error")));
                    }
                }
                break;
            }

            remaining = next_remaining;

            if !remaining.is_empty()
                && self.config.wait_exponential_jitter
                && attempt < self.config.max_attempt_number
            {
                let wait = self.calculate_wait(attempt);
                tokio::time::sleep(wait).await;
            }
        }

        results
            .into_iter()
            .map(|opt| opt.unwrap_or_else(|| Err(Error::other("No result"))))
            .collect()
    }
}

/// Extension trait to add retry configuration method to any Runnable.
pub trait RunnableRetryExt: Runnable {
    /// Create a new Runnable that retries this runnable on failure with full config.
    ///
    /// # Arguments
    /// * `config` - Retry configuration
    ///
    /// # Returns
    /// A new `RunnableRetry` instance
    fn with_retry_config(self, config: RunnableRetryConfig) -> RunnableRetry<Self>
    where
        Self: Sized,
    {
        RunnableRetry::new(self, config)
    }
}

impl<R: Runnable> RunnableRetryExt for R {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runnables::base::RunnableLambda;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn test_retry_succeeds_first_attempt() {
        let runnable = RunnableLambda::new(|x: i32| Ok(x + 1));
        let config = RunnableRetryConfig::new()
            .with_max_attempt_number(3)
            .with_wait_exponential_jitter(false);
        let retry = RunnableRetry::new(runnable, config);

        let result = retry.invoke(1, None).unwrap();
        assert_eq!(result, 2);
    }

    #[test]
    fn test_retry_succeeds_after_failures() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let runnable = RunnableLambda::new(move |x: i32| {
            let count = counter_clone.fetch_add(1, Ordering::SeqCst);
            if count < 2 {
                Err(Error::other("transient failure"))
            } else {
                Ok(x * 2)
            }
        });

        let config = RunnableRetryConfig::new()
            .with_max_attempt_number(5)
            .with_wait_exponential_jitter(false);
        let retry = RunnableRetry::new(runnable, config);

        let result = retry.invoke(5, None).unwrap();
        assert_eq!(result, 10);
        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }

    #[test]
    fn test_retry_exhausted() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let runnable = RunnableLambda::new(move |_x: i32| {
            counter_clone.fetch_add(1, Ordering::SeqCst);
            Err::<i32, _>(Error::other("always fails"))
        });

        let config = RunnableRetryConfig::new()
            .with_max_attempt_number(3)
            .with_wait_exponential_jitter(false);
        let retry = RunnableRetry::new(runnable, config);

        let result = retry.invoke(1, None);
        assert!(result.is_err());
        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }

    #[test]
    fn test_retry_predicate_http_errors() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let runnable = RunnableLambda::new(move |_x: i32| {
            counter_clone.fetch_add(1, Ordering::SeqCst);
            Err::<i32, _>(Error::other("not an HTTP error"))
        });

        let config = RunnableRetryConfig::new()
            .with_max_attempt_number(3)
            .with_retry_predicate(RetryErrorPredicate::HttpErrors)
            .with_wait_exponential_jitter(false);
        let retry = RunnableRetry::new(runnable, config);

        let result = retry.invoke(1, None);
        assert!(result.is_err());
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_exponential_jitter_params() {
        let params = ExponentialJitterParams::new()
            .with_initial(0.1)
            .with_max(1.0)
            .with_exp_base(2.0)
            .with_jitter(0.0);

        let wait1 = params.calculate_wait(1);
        assert!(wait1.as_secs_f64() >= 0.1 && wait1.as_secs_f64() < 0.2);

        let wait2 = params.calculate_wait(2);
        assert!(wait2.as_secs_f64() >= 0.2 && wait2.as_secs_f64() < 0.3);

        let wait3 = params.calculate_wait(3);
        assert!(wait3.as_secs_f64() >= 0.4 && wait3.as_secs_f64() < 0.5);
    }

    #[test]
    fn test_exponential_jitter_max_cap() {
        let params = ExponentialJitterParams::new()
            .with_initial(1.0)
            .with_max(2.0)
            .with_exp_base(10.0)
            .with_jitter(0.0);

        let wait = params.calculate_wait(10);
        assert!(wait.as_secs_f64() >= 2.0 && wait.as_secs_f64() < 2.1);
    }

    #[test]
    fn test_retry_ext_trait() {
        let runnable = RunnableLambda::new(|x: i32| Ok(x + 1));
        let config = RunnableRetryConfig::new().with_max_attempt_number(3);
        let retry = runnable.with_retry_config(config);

        let result = retry.invoke(1, None).unwrap();
        assert_eq!(result, 2);
    }

    #[test]
    fn test_retry_with_simple() {
        let runnable = RunnableLambda::new(|x: i32| Ok(x + 1));
        let retry = runnable.with_retry(3, false);

        let result = retry.invoke(1, None).unwrap();
        assert_eq!(result, 2);
    }

    #[test]
    fn test_batch_retry_partial_failures() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let runnable = RunnableLambda::new(move |x: i32| {
            let count = counter_clone.fetch_add(1, Ordering::SeqCst);
            if x < 0 && count < 4 {
                Err(Error::other("negative input"))
            } else {
                Ok(x * 2)
            }
        });

        let config = RunnableRetryConfig::new()
            .with_max_attempt_number(3)
            .with_wait_exponential_jitter(false);
        let retry = RunnableRetry::new(runnable, config);

        let results = retry.batch(vec![1, -1, 2], None, true);

        assert!(results[0].is_ok());
        assert!(results[2].is_ok());
    }

    #[tokio::test]
    async fn test_async_retry() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let runnable = RunnableLambda::new(move |x: i32| {
            let count = counter_clone.fetch_add(1, Ordering::SeqCst);
            if count < 1 {
                Err(Error::other("transient failure"))
            } else {
                Ok(x * 2)
            }
        });

        let config = RunnableRetryConfig::new()
            .with_max_attempt_number(3)
            .with_wait_exponential_jitter(false);
        let retry = RunnableRetry::new(runnable, config);

        let result = retry.ainvoke(5, None).await.unwrap();
        assert_eq!(result, 10);
        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }
}
