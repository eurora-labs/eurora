//! Runnable that routes to a set of Runnables.
//!
//! This module provides `RouterRunnable` which routes to different Runnables
//! based on a key in the input, mirroring `langchain_core.runnables.router`.

use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;

use async_trait::async_trait;
use futures::StreamExt;
use futures::stream::BoxStream;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::{Error, Result};
use crate::load::{Serializable, Serialized, SerializedConstructorData};

use super::base::{DynRunnable, Runnable, RunnableSerializable};
use super::config::{ConfigOrList, RunnableConfig, get_config_list};
use super::utils::gather_with_concurrency;

/// Router input.
///
/// This struct represents the input to a RouterRunnable, containing
/// the key to route on and the actual input to pass to the selected Runnable.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouterInput<I> {
    /// The key to route on.
    pub key: String,
    /// The input to pass to the selected `Runnable`.
    pub input: I,
}

impl<I> RouterInput<I> {
    /// Create a new RouterInput.
    pub fn new(key: impl Into<String>, input: I) -> Self {
        Self {
            key: key.into(),
            input,
        }
    }
}

/// A `Runnable` that routes to a set of `Runnable` based on `Input['key']`.
///
/// Returns the output of the selected Runnable.
///
/// # Example
///
/// ```ignore
/// use agent_chain_core::runnables::{RouterRunnable, RunnableLambda, RouterInput};
///
/// let add = RunnableLambda::new(|x: i32| Ok(x + 1));
/// let square = RunnableLambda::new(|x: i32| Ok(x * x));
///
/// let router = RouterRunnable::new()
///     .add("add", add)
///     .add("square", square);
///
/// let result = router.invoke(RouterInput::new("square", 3), None)?;
/// assert_eq!(result, 9);
/// ```
pub struct RouterRunnable<I, O>
where
    I: Send + Sync + Clone + Debug + 'static,
    O: Send + Sync + Clone + Debug + 'static,
{
    /// The mapping of keys to Runnables.
    runnables: HashMap<String, DynRunnable<I, O>>,
    /// Optional name for this router.
    name: Option<String>,
}

impl<I, O> Debug for RouterRunnable<I, O>
where
    I: Send + Sync + Clone + Debug + 'static,
    O: Send + Sync + Clone + Debug + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RouterRunnable")
            .field("runnables", &self.runnables.keys().collect::<Vec<_>>())
            .field("name", &self.name)
            .finish()
    }
}

impl<I, O> RouterRunnable<I, O>
where
    I: Send + Sync + Clone + Debug + 'static,
    O: Send + Sync + Clone + Debug + 'static,
{
    /// Create a new empty RouterRunnable.
    pub fn new() -> Self {
        Self {
            runnables: HashMap::new(),
            name: None,
        }
    }

    /// Create a new RouterRunnable from a HashMap of runnables.
    pub fn from_runnables(runnables: HashMap<String, DynRunnable<I, O>>) -> Self {
        Self {
            runnables,
            name: None,
        }
    }

    /// Add a runnable to the router.
    pub fn add<R>(mut self, key: impl Into<String>, runnable: R) -> Self
    where
        R: Runnable<Input = I, Output = O> + Send + Sync + 'static,
    {
        self.runnables.insert(key.into(), Arc::new(runnable));
        self
    }

    /// Set the name of this router.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }
}

impl<I, O> Default for RouterRunnable<I, O>
where
    I: Send + Sync + Clone + Debug + 'static,
    O: Send + Sync + Clone + Debug + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl<I, O> Runnable for RouterRunnable<I, O>
where
    I: Send + Sync + Clone + Debug + 'static,
    O: Send + Sync + Clone + Debug + 'static,
{
    type Input = RouterInput<I>;
    type Output = O;

    fn name(&self) -> Option<String> {
        self.name.clone().or_else(|| {
            Some(format!(
                "RouterRunnable<{}>",
                self.runnables.keys().cloned().collect::<Vec<_>>().join(",")
            ))
        })
    }

    fn invoke(&self, input: Self::Input, config: Option<RunnableConfig>) -> Result<Self::Output> {
        let key = &input.key;
        let actual_input = input.input;

        let runnable = self
            .runnables
            .get(key)
            .ok_or_else(|| Error::Other(format!("No runnable associated with key '{}'", key)))?;

        runnable.invoke(actual_input, config)
    }

    async fn ainvoke(
        &self,
        input: Self::Input,
        config: Option<RunnableConfig>,
    ) -> Result<Self::Output>
    where
        Self: 'static,
    {
        let key = &input.key;
        let actual_input = input.input;

        let runnable = self
            .runnables
            .get(key)
            .ok_or_else(|| Error::Other(format!("No runnable associated with key '{}'", key)))?;

        runnable.ainvoke(actual_input, config).await
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

        let keys: Vec<_> = inputs.iter().map(|i| i.key.clone()).collect();
        let actual_inputs: Vec<_> = inputs.into_iter().map(|i| i.input).collect();

        // Check if all keys have corresponding runnables
        for key in &keys {
            if !self.runnables.contains_key(key) {
                return vec![Err(Error::Other(
                    "One or more keys do not have a corresponding runnable".to_string(),
                ))];
            }
        }

        let configs = get_config_list(config, keys.len());

        let _return_exceptions = return_exceptions; // Vec<Result<O>> already captures exceptions per-item
        let results: Vec<Result<O>> = keys
            .into_iter()
            .zip(actual_inputs)
            .zip(configs)
            .map(|((key, input), config)| {
                let runnable = self.runnables.get(&key).ok_or_else(|| {
                    Error::Other(format!("No runnable associated with key '{}'", key))
                })?;
                runnable.invoke(input, Some(config))
            })
            .collect();

        results
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

        let keys: Vec<_> = inputs.iter().map(|i| i.key.clone()).collect();
        let actual_inputs: Vec<_> = inputs.into_iter().map(|i| i.input).collect();

        // Check if all keys have corresponding runnables
        for key in &keys {
            if !self.runnables.contains_key(key) {
                return vec![Err(Error::Other(
                    "One or more keys do not have a corresponding runnable".to_string(),
                ))];
            }
        }

        let configs = get_config_list(config, keys.len());
        let max_concurrency = configs.first().and_then(|c| c.max_concurrency);

        let _return_exceptions = return_exceptions; // Vec<Result<O>> already captures exceptions per-item
        // Create futures for each invocation
        let futures: Vec<_> = keys
            .into_iter()
            .zip(actual_inputs)
            .zip(configs)
            .map(|((key, input), config)| {
                let runnable = self.runnables.get(&key).cloned().ok_or_else(|| {
                    Error::Other(format!("No runnable associated with key '{}'", key))
                });
                Box::pin(async move {
                    let runnable = runnable?;
                    runnable.ainvoke(input, Some(config)).await
                })
                    as std::pin::Pin<Box<dyn std::future::Future<Output = Result<O>> + Send>>
            })
            .collect();

        gather_with_concurrency(max_concurrency, futures).await
    }

    fn stream(
        &self,
        input: Self::Input,
        config: Option<RunnableConfig>,
    ) -> BoxStream<'_, Result<Self::Output>> {
        let key = input.key.clone();
        let actual_input = input.input;

        Box::pin(async_stream::stream! {
            let runnable = match self.runnables.get(&key) {
                Some(r) => r,
                None => {
                    yield Err(Error::Other(format!("No runnable associated with key '{}'", key)));
                    return;
                }
            };

            let mut stream = runnable.stream(actual_input, config);
            while let Some(output) = stream.next().await {
                yield output;
            }
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
        let key = input.key.clone();
        let actual_input = input.input;

        Box::pin(async_stream::stream! {
            let runnable = match self.runnables.get(&key) {
                Some(r) => r,
                None => {
                    yield Err(Error::Other(format!("No runnable associated with key '{}'", key)));
                    return;
                }
            };

            let mut stream = runnable.astream(actual_input, config);
            while let Some(output) = stream.next().await {
                yield output;
            }
        })
    }
}

impl<I, O> Serializable for RouterRunnable<I, O>
where
    I: Send + Sync + Clone + Debug + Serialize + 'static,
    O: Send + Sync + Clone + Debug + 'static,
{
    fn is_lc_serializable() -> bool {
        true
    }

    fn get_lc_namespace() -> Vec<String> {
        vec![
            "langchain".to_string(),
            "schema".to_string(),
            "runnable".to_string(),
        ]
    }

    fn to_json(&self) -> Serialized {
        let mut kwargs = std::collections::HashMap::new();
        kwargs.insert(
            "runnables".to_string(),
            serde_json::json!(self.runnables.keys().collect::<Vec<_>>()),
        );

        Serialized::Constructor(SerializedConstructorData {
            lc: 1,
            id: Self::get_lc_namespace(),
            kwargs,
            name: None,
            graph: None,
        })
    }
}

impl<I, O> RunnableSerializable for RouterRunnable<I, O>
where
    I: Send + Sync + Clone + Debug + Serialize + 'static,
    O: Send + Sync + Clone + Debug + Serialize + 'static,
{
}

/// Type alias for a RouterRunnable with Value input and output.
///
/// This is useful when the types are not known at compile time.
pub type DynRouterRunnable = RouterRunnable<Value, Value>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runnables::RunnableLambda;

    #[test]
    fn test_router_input() {
        let input = RouterInput::new("add", 5);
        assert_eq!(input.key, "add");
        assert_eq!(input.input, 5);
    }

    #[test]
    fn test_router_runnable_invoke() {
        let add = RunnableLambda::new(|x: i32| Ok(x + 1));
        let square = RunnableLambda::new(|x: i32| Ok(x * x));

        let router = RouterRunnable::new().add("add", add).add("square", square);

        let result = router.invoke(RouterInput::new("add", 5), None).unwrap();
        assert_eq!(result, 6);

        let result = router.invoke(RouterInput::new("square", 4), None).unwrap();
        assert_eq!(result, 16);
    }

    #[test]
    fn test_router_runnable_missing_key() {
        let add = RunnableLambda::new(|x: i32| Ok(x + 1));
        let router = RouterRunnable::new().add("add", add);

        let result = router.invoke(RouterInput::new("multiply", 5), None);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("No runnable associated with key")
        );
    }

    #[test]
    fn test_router_runnable_batch() {
        let add = RunnableLambda::new(|x: i32| Ok(x + 1));
        let square = RunnableLambda::new(|x: i32| Ok(x * x));

        let router = RouterRunnable::new().add("add", add).add("square", square);

        let inputs = vec![
            RouterInput::new("add", 5),
            RouterInput::new("square", 4),
            RouterInput::new("add", 10),
        ];

        let results = router.batch(inputs, None, false);
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].as_ref().unwrap(), &6);
        assert_eq!(results[1].as_ref().unwrap(), &16);
        assert_eq!(results[2].as_ref().unwrap(), &11);
    }

    #[test]
    fn test_router_runnable_name() {
        let add = RunnableLambda::new(|x: i32| Ok(x + 1));

        let router = RouterRunnable::new().add("add", add).with_name("my_router");

        assert_eq!(router.name(), Some("my_router".to_string()));
    }

    #[test]
    fn test_router_runnable_default_name() {
        let add = RunnableLambda::new(|x: i32| Ok(x + 1));
        let square = RunnableLambda::new(|x: i32| Ok(x * x));

        let router = RouterRunnable::new().add("add", add).add("square", square);

        let name = router.name().unwrap();
        assert!(name.starts_with("RouterRunnable<"));
        assert!(name.contains("add") || name.contains("square"));
    }

    #[tokio::test]
    async fn test_router_runnable_ainvoke() {
        let add = RunnableLambda::new(|x: i32| Ok(x + 1));
        let square = RunnableLambda::new(|x: i32| Ok(x * x));

        let router = RouterRunnable::new().add("add", add).add("square", square);

        let result = router
            .ainvoke(RouterInput::new("add", 5), None)
            .await
            .unwrap();
        assert_eq!(result, 6);

        let result = router
            .ainvoke(RouterInput::new("square", 4), None)
            .await
            .unwrap();
        assert_eq!(result, 16);
    }

    #[tokio::test]
    async fn test_router_runnable_abatch() {
        let add = RunnableLambda::new(|x: i32| Ok(x + 1));
        let square = RunnableLambda::new(|x: i32| Ok(x * x));

        let router = RouterRunnable::new().add("add", add).add("square", square);

        let inputs = vec![RouterInput::new("add", 5), RouterInput::new("square", 4)];

        let results = router.abatch(inputs, None, false).await;
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].as_ref().unwrap(), &6);
        assert_eq!(results[1].as_ref().unwrap(), &16);
    }

    #[tokio::test]
    async fn test_router_runnable_stream() {
        let add = RunnableLambda::new(|x: i32| Ok(x + 1));

        let router = RouterRunnable::new().add("add", add);

        let mut stream = router.stream(RouterInput::new("add", 5), None);
        let result = stream.next().await.unwrap().unwrap();
        assert_eq!(result, 6);
    }
}
