//! Implementation of the RunnablePassthrough and related types.
//!
//! This module provides:
//! - `RunnablePassthrough`: A runnable that passes through inputs unchanged or with additional keys
//! - `RunnableAssign`: A runnable that assigns key-value pairs to dict inputs
//! - `RunnablePick`: A runnable that picks keys from dict inputs

use std::collections::HashMap;
use std::fmt::Debug;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// Type alias for the synchronous callback function in RunnablePassthrough.
type PassthroughFunc<I> = Arc<dyn Fn(&I, &RunnableConfig) + Send + Sync>;

/// Type alias for the asynchronous callback function in RunnablePassthrough.
type PassthroughAfunc<I> =
    Arc<dyn Fn(&I, &RunnableConfig) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;

use async_trait::async_trait;
use futures::StreamExt;
use futures::stream::BoxStream;
use serde_json::Value;

use crate::error::{Error, Result};

use super::base::{Runnable, RunnableParallel};
use super::config::{RunnableConfig, ensure_config, get_callback_manager_for_config, patch_config};

/// A Runnable that passes through its input unchanged or with additional keys.
///
/// This Runnable behaves almost like the identity function, except that it
/// can be configured to add additional keys to the output, if the input is a dict.
/// It can also optionally run a callback function with the input.
///
/// # Example
///
/// ```ignore
/// use agent_chain_core::runnables::{RunnablePassthrough, RunnableParallel};
///
/// // Simple passthrough
/// let passthrough: RunnablePassthrough<i32> = RunnablePassthrough::new();
/// let result = passthrough.invoke(42, None).unwrap();
/// assert_eq!(result, 42);
///
/// // With assign
/// let runnable = RunnablePassthrough::assign()
///     .add("extra_key", RunnableLambda::new(|x| Ok(x["value"].clone())));
/// ```
pub struct RunnablePassthrough<I>
where
    I: Send + Sync + Clone + Debug + 'static,
{
    name: Option<String>,
    func: Option<PassthroughFunc<I>>,
    afunc: Option<PassthroughAfunc<I>>,
    _phantom: std::marker::PhantomData<I>,
}

impl<I> Debug for RunnablePassthrough<I>
where
    I: Send + Sync + Clone + Debug + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RunnablePassthrough")
            .field("name", &self.name)
            .field("has_func", &self.func.is_some())
            .field("has_afunc", &self.afunc.is_some())
            .finish()
    }
}

impl<I> Clone for RunnablePassthrough<I>
where
    I: Send + Sync + Clone + Debug + 'static,
{
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            func: self.func.clone(),
            afunc: self.afunc.clone(),
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<I> Default for RunnablePassthrough<I>
where
    I: Send + Sync + Clone + Debug + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<I> RunnablePassthrough<I>
where
    I: Send + Sync + Clone + Debug + 'static,
{
    /// Create a new RunnablePassthrough.
    pub fn new() -> Self {
        Self {
            name: None,
            func: None,
            afunc: None,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Create a new RunnablePassthrough with a callback function.
    ///
    /// The function will be called with the input before passing it through.
    pub fn with_func<F>(func: F) -> Self
    where
        F: Fn(&I, &RunnableConfig) + Send + Sync + 'static,
    {
        Self {
            name: None,
            func: Some(Arc::new(func)),
            afunc: None,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Create a new RunnablePassthrough with an async callback function.
    ///
    /// The function will be called with the input before passing it through.
    pub fn with_afunc<F, Fut>(afunc: F) -> Self
    where
        F: Fn(&I, &RunnableConfig) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        Self {
            name: None,
            func: None,
            afunc: Some(Arc::new(move |input, config| {
                Box::pin(afunc(input, config)) as Pin<Box<dyn Future<Output = ()> + Send>>
            })),
            _phantom: std::marker::PhantomData,
        }
    }

    /// Set the name of this RunnablePassthrough.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Merge the dict input with the output produced by the mapping argument.
    ///
    /// Returns a `RunnableAssign` that will run the given mapper in parallel
    /// and merge its output with the input.
    pub fn assign() -> RunnableAssignBuilder {
        RunnableAssignBuilder::new()
    }
}

#[async_trait]
impl<I> Runnable for RunnablePassthrough<I>
where
    I: Send + Sync + Clone + Debug + 'static,
{
    type Input = I;
    type Output = I;

    fn name(&self) -> Option<String> {
        self.name.clone()
    }

    fn get_input_schema(&self, _config: Option<&RunnableConfig>) -> serde_json::Value {
        serde_json::json!({
            "title": self.get_name(Some("Input"), None),
            "type": "object"
        })
    }

    fn get_output_schema(&self, _config: Option<&RunnableConfig>) -> serde_json::Value {
        serde_json::json!({
            "title": self.get_name(Some("Output"), None),
            "type": "object"
        })
    }

    fn invoke(&self, input: Self::Input, config: Option<RunnableConfig>) -> Result<Self::Output> {
        let config = ensure_config(config);

        if let Some(ref func) = self.func {
            func(&input, &config);
        }

        Ok(input)
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

        if let Some(ref afunc) = self.afunc {
            afunc(&input, &config).await;
        } else if let Some(ref func) = self.func {
            func(&input, &config);
        }

        Ok(input)
    }

    fn stream(
        &self,
        input: Self::Input,
        config: Option<RunnableConfig>,
    ) -> BoxStream<'_, Result<Self::Output>> {
        let config = ensure_config(config);

        Box::pin(async_stream::stream! {
            if let Some(ref func) = self.func {
                func(&input, &config);
            }
            yield Ok(input);
        })
    }

    fn transform<'a>(
        &'a self,
        input: BoxStream<'a, Self::Input>,
        config: Option<RunnableConfig>,
    ) -> BoxStream<'a, Result<Self::Output>> {
        let config = ensure_config(config);

        if self.func.is_none() {
            Box::pin(input.map(Ok))
        } else {
            let func = self.func.clone();
            Box::pin(async_stream::stream! {
                let mut final_input: Option<Self::Input> = None;
                let mut first_chunk = true;
                let mut input = input;

                while let Some(chunk) = input.next().await {
                    yield Ok(chunk.clone());

                    if first_chunk {
                        final_input = Some(chunk);
                        first_chunk = false;
                    } else if let Some(ref mut current) = final_input {
                        *current = chunk;
                    }
                }

                if let (Some(func), Some(final_val)) = (func, final_input) {
                    func(&final_val, &config);
                }
            })
        }
    }

    fn atransform<'a>(
        &'a self,
        input: BoxStream<'a, Self::Input>,
        config: Option<RunnableConfig>,
    ) -> BoxStream<'a, Result<Self::Output>>
    where
        Self: 'static,
    {
        let config = ensure_config(config);

        if self.func.is_none() && self.afunc.is_none() {
            Box::pin(input.map(Ok))
        } else {
            let func = self.func.clone();
            let afunc = self.afunc.clone();
            Box::pin(async_stream::stream! {
                let mut final_input: Option<Self::Input> = None;
                let mut first_chunk = true;
                let mut input = input;

                while let Some(chunk) = input.next().await {
                    yield Ok(chunk.clone());

                    if first_chunk {
                        final_input = Some(chunk);
                        first_chunk = false;
                    } else if let Some(ref mut current) = final_input {
                        *current = chunk;
                    }
                }

                if let Some(final_val) = final_input {
                    if let Some(ref afunc) = afunc {
                        afunc(&final_val, &config).await;
                    } else if let Some(ref func) = func {
                        func(&final_val, &config);
                    }
                }
            })
        }
    }
}

/// Builder for creating a RunnableAssign.
///
/// Use `RunnablePassthrough::assign()` to create a new builder.
pub struct RunnableAssignBuilder {
    mapper: RunnableParallel<HashMap<String, Value>>,
}

impl RunnableAssignBuilder {
    fn new() -> Self {
        Self {
            mapper: RunnableParallel::new(),
        }
    }

    /// Add a step to the assign operation.
    pub fn add<R>(mut self, key: impl Into<String>, runnable: R) -> Self
    where
        R: Runnable<Input = HashMap<String, Value>, Output = Value> + Send + Sync + 'static,
    {
        self.mapper = self.mapper.add(key, runnable);
        self
    }

    /// Build the RunnableAssign.
    pub fn build(self) -> RunnableAssign {
        RunnableAssign::new(self.mapper)
    }
}

/// Runnable that assigns key-value pairs to dict inputs.
///
/// The `RunnableAssign` class takes input dictionaries and, through a
/// `RunnableParallel` instance, applies transformations, then combines
/// these with the original data, introducing new key-value pairs based
/// on the mapper's logic.
///
/// # Example
///
/// ```ignore
/// use agent_chain_core::runnables::{RunnableAssign, RunnableParallel, RunnableLambda};
/// use std::collections::HashMap;
///
/// let mapper = RunnableParallel::new()
///     .add("extra", RunnableLambda::new(|input: HashMap<String, Value>| {
///         Ok(serde_json::json!(input.get("value").cloned().unwrap_or_default()))
///     }));
///
/// let assign = RunnableAssign::new(mapper);
/// let mut input = HashMap::new();
/// input.insert("value".to_string(), serde_json::json!(42));
/// let result = assign.invoke(input, None).unwrap();
/// // result contains {"value": 42, "extra": 42}
/// ```
pub struct RunnableAssign {
    mapper: RunnableParallel<HashMap<String, Value>>,
    name: Option<String>,
}

impl Debug for RunnableAssign {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RunnableAssign")
            .field("mapper", &self.mapper)
            .field("name", &self.name)
            .finish()
    }
}

impl RunnableAssign {
    /// Create a new RunnableAssign with the given mapper.
    pub fn new(mapper: RunnableParallel<HashMap<String, Value>>) -> Self {
        Self { mapper, name: None }
    }

    /// Set the name of this RunnableAssign.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Get a reference to the underlying mapper.
    pub fn mapper(&self) -> &RunnableParallel<HashMap<String, Value>> {
        &self.mapper
    }
}

#[async_trait]
impl Runnable for RunnableAssign {
    type Input = HashMap<String, Value>;
    type Output = HashMap<String, Value>;

    fn name(&self) -> Option<String> {
        self.name
            .clone()
            .or_else(|| Some("RunnableAssign".to_string()))
    }

    fn get_input_schema(&self, config: Option<&RunnableConfig>) -> serde_json::Value {
        self.mapper.get_input_schema(config)
    }

    fn get_output_schema(&self, config: Option<&RunnableConfig>) -> serde_json::Value {
        let input_schema = self.mapper.get_input_schema(config);
        let output_schema = self.mapper.get_output_schema(config);
        // Merge input and output properties into a combined schema
        let mut properties = serde_json::Map::new();
        if let Some(props) = input_schema.get("properties").and_then(|v| v.as_object()) {
            for (k, v) in props {
                properties.insert(k.clone(), v.clone());
            }
        }
        if let Some(props) = output_schema.get("properties").and_then(|v| v.as_object()) {
            for (k, v) in props {
                properties.insert(k.clone(), v.clone());
            }
        }
        serde_json::json!({
            "title": "RunnableAssignOutput",
            "type": "object",
            "properties": properties
        })
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

        let child_config = patch_config(
            Some(config),
            Some(run_manager.get_child(None)),
            None,
            None,
            None,
            None,
        );

        let mapper_output = match self.mapper.invoke(input.clone(), Some(child_config)) {
            Ok(output) => output,
            Err(e) => {
                run_manager.on_chain_error(&e);
                return Err(e);
            }
        };

        let mut result = input;
        for (key, value) in mapper_output {
            result.insert(key, value);
        }

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

        let mapper_output = match self.mapper.ainvoke(input.clone(), Some(child_config)).await {
            Ok(output) => output,
            Err(e) => {
                run_manager.on_chain_error(&e);
                return Err(e);
            }
        };

        let mut result = input;
        for (key, value) in mapper_output {
            result.insert(key, value);
        }

        run_manager.on_chain_end(&HashMap::new());
        Ok(result)
    }

    fn stream(
        &self,
        input: Self::Input,
        config: Option<RunnableConfig>,
    ) -> BoxStream<'_, Result<Self::Output>> {
        Box::pin(async_stream::stream! {
            let config = ensure_config(config);

            let mapper_keys: std::collections::HashSet<String> = std::collections::HashSet::new();

            let filtered: HashMap<String, Value> = input
                .iter()
                .filter(|(k, _)| !mapper_keys.contains(*k))
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();

            if !filtered.is_empty() {
                yield Ok(filtered);
            }

            match self.mapper.invoke(input, Some(config)) {
                Ok(mapper_output) => {
                    yield Ok(mapper_output);
                }
                Err(e) => {
                    yield Err(e);
                }
            }
        })
    }

    fn transform<'a>(
        &'a self,
        input: BoxStream<'a, Self::Input>,
        config: Option<RunnableConfig>,
    ) -> BoxStream<'a, Result<Self::Output>> {
        let config = ensure_config(config);

        Box::pin(async_stream::stream! {
            let mut collected_input: Option<HashMap<String, Value>> = None;
            let mut input = input;

            while let Some(chunk) = input.next().await {
                if let Some(ref mut current) = collected_input {
                    for (key, value) in chunk {
                        current.insert(key, value);
                    }
                } else {
                    collected_input = Some(chunk);
                }
            }

            if let Some(final_input) = collected_input {
                match self.invoke(final_input, Some(config)) {
                    Ok(result) => yield Ok(result),
                    Err(e) => yield Err(e),
                }
            }
        })
    }

    fn atransform<'a>(
        &'a self,
        input: BoxStream<'a, Self::Input>,
        config: Option<RunnableConfig>,
    ) -> BoxStream<'a, Result<Self::Output>>
    where
        Self: 'static,
    {
        let config = ensure_config(config);

        Box::pin(async_stream::stream! {
            let mut collected_input: Option<HashMap<String, Value>> = None;
            let mut input = input;

            while let Some(chunk) = input.next().await {
                if let Some(ref mut current) = collected_input {
                    for (key, value) in chunk {
                        current.insert(key, value);
                    }
                } else {
                    collected_input = Some(chunk);
                }
            }

            if let Some(final_input) = collected_input {
                match self.ainvoke(final_input, Some(config)).await {
                    Ok(result) => yield Ok(result),
                    Err(e) => yield Err(e),
                }
            }
        })
    }
}

/// Runnable that picks keys from dict inputs.
///
/// `RunnablePick` selects specific keys from a dictionary input.
/// The return type depends on whether a single key or multiple keys are specified:
/// - Single key: Returns the value directly
/// - Multiple keys: Returns a dictionary with only the selected keys
///
/// # Example
///
/// ```ignore
/// use agent_chain_core::runnables::RunnablePick;
/// use std::collections::HashMap;
///
/// let mut input = HashMap::new();
/// input.insert("name".to_string(), serde_json::json!("John"));
/// input.insert("age".to_string(), serde_json::json!(30));
/// input.insert("city".to_string(), serde_json::json!("NYC"));
///
/// // Single key - returns the value directly
/// let pick_single = RunnablePick::new_single("name");
/// let result = pick_single.invoke(input.clone(), None).unwrap();
/// // result is "John"
///
/// // Multiple keys - returns a dict
/// let pick_multi = RunnablePick::new_multi(vec!["name", "age"]);
/// let result = pick_multi.invoke(input, None).unwrap();
/// // result is {"name": "John", "age": 30}
/// ```
pub struct RunnablePick {
    keys: PickKeys,
    name: Option<String>,
}

/// Keys to pick - either a single key or multiple keys.
#[derive(Debug, Clone)]
pub enum PickKeys {
    Single(String),
    Multiple(Vec<String>),
}

impl Debug for RunnablePick {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RunnablePick")
            .field("keys", &self.keys)
            .field("name", &self.name)
            .finish()
    }
}

impl RunnablePick {
    /// Create a new RunnablePick that picks a single key.
    pub fn new_single(key: impl Into<String>) -> Self {
        Self {
            keys: PickKeys::Single(key.into()),
            name: None,
        }
    }

    /// Create a new RunnablePick that picks multiple keys.
    pub fn new_multi(keys: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            keys: PickKeys::Multiple(keys.into_iter().map(Into::into).collect()),
            name: None,
        }
    }

    /// Set the name of this RunnablePick.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Pick the specified keys from the input.
    fn pick(&self, input: &HashMap<String, Value>) -> Option<Value> {
        match &self.keys {
            PickKeys::Single(key) => input.get(key).cloned(),
            PickKeys::Multiple(keys) => {
                let picked: HashMap<String, Value> = keys
                    .iter()
                    .filter_map(|k| input.get(k).map(|v| (k.clone(), v.clone())))
                    .collect();

                if picked.is_empty() {
                    None
                } else {
                    Some(serde_json::to_value(picked).unwrap_or(Value::Null))
                }
            }
        }
    }
}

#[async_trait]
impl Runnable for RunnablePick {
    type Input = HashMap<String, Value>;
    type Output = Value;

    fn name(&self) -> Option<String> {
        self.name.clone().or_else(|| {
            let keys_str = match &self.keys {
                PickKeys::Single(k) => k.clone(),
                PickKeys::Multiple(keys) => keys.join(","),
            };
            Some(format!("RunnablePick<{}>", keys_str))
        })
    }

    fn get_output_schema(&self, _config: Option<&RunnableConfig>) -> serde_json::Value {
        match &self.keys {
            PickKeys::Single(key) => serde_json::json!({
                "title": format!("RunnablePick<{}>Output", key),
                "type": "object",
                "properties": {
                    key: { "title": key }
                }
            }),
            PickKeys::Multiple(keys) => {
                let mut properties = serde_json::Map::new();
                for key in keys {
                    properties.insert(key.clone(), serde_json::json!({ "title": key }));
                }
                serde_json::json!({
                    "title": self.get_name(Some("Output"), None),
                    "type": "object",
                    "properties": properties
                })
            }
        }
    }
    fn invoke(&self, input: Self::Input, _config: Option<RunnableConfig>) -> Result<Self::Output> {
        self.pick(&input)
            .ok_or_else(|| Error::Other("No matching keys found in input".to_string()))
    }

    async fn ainvoke(
        &self,
        input: Self::Input,
        _config: Option<RunnableConfig>,
    ) -> Result<Self::Output>
    where
        Self: 'static,
    {
        self.pick(&input)
            .ok_or_else(|| Error::Other("No matching keys found in input".to_string()))
    }

    fn stream(
        &self,
        input: Self::Input,
        _config: Option<RunnableConfig>,
    ) -> BoxStream<'_, Result<Self::Output>> {
        let result = self
            .pick(&input)
            .ok_or_else(|| Error::Other("No matching keys found in input".to_string()));
        Box::pin(futures::stream::once(async move { result }))
    }

    fn transform<'a>(
        &'a self,
        input: BoxStream<'a, Self::Input>,
        _config: Option<RunnableConfig>,
    ) -> BoxStream<'a, Result<Self::Output>> {
        Box::pin(async_stream::stream! {
            let mut input = input;
            while let Some(chunk) = input.next().await {
                if let Some(picked) = self.pick(&chunk) {
                    yield Ok(picked);
                }
            }
        })
    }

    fn atransform<'a>(
        &'a self,
        input: BoxStream<'a, Self::Input>,
        _config: Option<RunnableConfig>,
    ) -> BoxStream<'a, Result<Self::Output>>
    where
        Self: 'static,
    {
        Box::pin(async_stream::stream! {
            let mut input = input;
            while let Some(chunk) = input.next().await {
                if let Some(picked) = self.pick(&chunk) {
                    yield Ok(picked);
                }
            }
        })
    }
}

/// A global passthrough runnable used for graph operations.
pub fn graph_passthrough<I>() -> RunnablePassthrough<I>
where
    I: Send + Sync + Clone + Debug + 'static,
{
    RunnablePassthrough::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runnable_passthrough() {
        let passthrough: RunnablePassthrough<i32> = RunnablePassthrough::new();
        let result = passthrough.invoke(42, None).unwrap();
        assert_eq!(result, 42);
    }

    #[test]
    fn test_runnable_passthrough_with_name() {
        let passthrough: RunnablePassthrough<i32> =
            RunnablePassthrough::new().with_name("test_passthrough");
        assert_eq!(passthrough.name(), Some("test_passthrough".to_string()));
    }

    #[test]
    fn test_runnable_passthrough_with_func() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};

        let called = Arc::new(AtomicBool::new(false));
        let called_clone = called.clone();

        let passthrough: RunnablePassthrough<i32> =
            RunnablePassthrough::with_func(move |_input, _config| {
                called_clone.store(true, Ordering::SeqCst);
            });

        let result = passthrough.invoke(42, None).unwrap();
        assert_eq!(result, 42);
        assert!(called.load(Ordering::SeqCst));
    }

    #[test]
    fn test_runnable_pick_single() {
        let pick = RunnablePick::new_single("name");

        let mut input = HashMap::new();
        input.insert("name".to_string(), serde_json::json!("John"));
        input.insert("age".to_string(), serde_json::json!(30));

        let result = pick.invoke(input, None).unwrap();
        assert_eq!(result, serde_json::json!("John"));
    }

    #[test]
    fn test_runnable_pick_multiple() {
        let pick = RunnablePick::new_multi(vec!["name", "age"]);

        let mut input = HashMap::new();
        input.insert("name".to_string(), serde_json::json!("John"));
        input.insert("age".to_string(), serde_json::json!(30));
        input.insert("city".to_string(), serde_json::json!("NYC"));

        let result = pick.invoke(input, None).unwrap();
        let result_map: HashMap<String, Value> = serde_json::from_value(result).unwrap();
        assert_eq!(result_map.len(), 2);
        assert_eq!(result_map.get("name"), Some(&serde_json::json!("John")));
        assert_eq!(result_map.get("age"), Some(&serde_json::json!(30)));
    }

    #[test]
    fn test_runnable_pick_name() {
        let pick_single = RunnablePick::new_single("name");
        assert_eq!(pick_single.name(), Some("RunnablePick<name>".to_string()));

        let pick_multi = RunnablePick::new_multi(vec!["name", "age"]);
        assert_eq!(
            pick_multi.name(),
            Some("RunnablePick<name,age>".to_string())
        );
    }

    #[tokio::test]
    async fn test_runnable_passthrough_async() {
        let passthrough: RunnablePassthrough<i32> = RunnablePassthrough::new();
        let result = passthrough.ainvoke(42, None).await.unwrap();
        assert_eq!(result, 42);
    }

    #[tokio::test]
    async fn test_runnable_pick_async() {
        let pick = RunnablePick::new_single("name");

        let mut input = HashMap::new();
        input.insert("name".to_string(), serde_json::json!("John"));

        let result = pick.ainvoke(input, None).await.unwrap();
        assert_eq!(result, serde_json::json!("John"));
    }
}
