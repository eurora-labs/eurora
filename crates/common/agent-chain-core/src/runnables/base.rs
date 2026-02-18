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
    merge_configs, patch_config, set_config_context,
};
use super::utils::{Addable, ConfigurableFieldSpec, get_unique_config_specs};

/// Type alias for config factory functions used by `RunnableBinding`.
///
/// Config factories are lazily evaluated functions that produce config overrides.
/// They receive the current merged config and return additional config to merge.
/// This mirrors Python's `RunnableBinding.config_factories`.
pub type ConfigFactory = Arc<dyn Fn(&RunnableConfig) -> RunnableConfig + Send + Sync>;

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
            .unwrap_or_else(|| short_type_name(self.type_name()));

        match suffix {
            Some(s) if name_.chars().next().is_some_and(|c| c.is_uppercase()) => {
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

    /// Helper method to transform an input value to an output value, with
    /// callbacks. Use this method to implement `invoke` in subclasses.
    ///
    /// Mirrors Python's `Runnable._call_with_config`.
    fn call_with_config(
        &self,
        func: &dyn Fn(Self::Input, &RunnableConfig) -> Result<Self::Output>,
        input: Self::Input,
        config: Option<RunnableConfig>,
    ) -> Result<Self::Output> {
        let config = ensure_config(config);
        let callback_manager = get_callback_manager_for_config(&config);
        let run_manager = callback_manager
            .on_chain_start()
            .serialized(&HashMap::new())
            .inputs(&HashMap::new())
            .maybe_run_id(config.run_id)
            .maybe_name(config.run_name.as_deref())
            .call();

        let child_config = patch_config(
            Some(config),
            Some(run_manager.get_child(None)),
            None,
            None,
            None,
            None,
        );

        let _context_guard = set_config_context(child_config.clone());

        match func(input, &child_config) {
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

    /// Async helper method to transform an input value to an output value,
    /// with callbacks. Use this method to implement `ainvoke` in subclasses.
    ///
    /// Mirrors Python's `Runnable._acall_with_config`.
    #[allow(async_fn_in_trait)]
    async fn acall_with_config(
        &self,
        func: &(
             dyn Fn(
            Self::Input,
            RunnableConfig,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<Self::Output>> + Send>,
        > + Send
                 + Sync
         ),
        input: Self::Input,
        config: Option<RunnableConfig>,
    ) -> Result<Self::Output>
    where
        Self: 'static,
    {
        let config = ensure_config(config);
        let async_callback_manager = get_async_callback_manager_for_config(&config);
        let run_manager = async_callback_manager
            .on_chain_start(
                &HashMap::new(),
                &HashMap::new(),
                config.run_id,
                config.run_name.as_deref(),
            )
            .await;

        let child_config = patch_config(
            Some(config),
            Some(run_manager.get_child(None).to_callback_manager()),
            None,
            None,
            None,
            None,
        );

        let _context_guard = set_config_context(child_config.clone());

        match func(input, child_config).await {
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

    /// Helper method to transform a list of inputs to a list of outputs,
    /// with per-item callbacks.
    ///
    /// Mirrors Python's `Runnable._batch_with_config`.
    fn batch_with_config(
        &self,
        func: &dyn Fn(Vec<Self::Input>, Vec<RunnableConfig>) -> Vec<Result<Self::Output>>,
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

        let run_managers: Vec<_> = configs
            .iter()
            .map(|config| {
                let callback_manager = get_callback_manager_for_config(config);
                callback_manager
                    .on_chain_start()
                    .serialized(&HashMap::new())
                    .inputs(&HashMap::new())
                    .maybe_run_id(config.run_id)
                    .maybe_name(config.run_name.as_deref())
                    .call()
            })
            .collect();

        let child_configs: Vec<_> = configs
            .into_iter()
            .zip(run_managers.iter())
            .map(|(config, run_manager)| {
                patch_config(
                    Some(config),
                    Some(run_manager.get_child(None)),
                    None,
                    None,
                    None,
                    None,
                )
            })
            .collect();

        let outputs = func(inputs, child_configs);

        let mut first_exception: Option<usize> = None;
        for (i, (run_manager, output)) in run_managers.iter().zip(outputs.iter()).enumerate() {
            match output {
                Ok(_) => run_manager.on_chain_end(&HashMap::new()),
                Err(e) => {
                    if first_exception.is_none() {
                        first_exception = Some(i);
                    }
                    run_manager.on_chain_error(e as &dyn std::error::Error);
                }
            }
        }

        if return_exceptions {
            outputs
        } else if let Some(idx) = first_exception {
            vec![Err(outputs
                .into_iter()
                .nth(idx)
                .expect("idx within bounds")
                .unwrap_err())]
        } else {
            outputs
        }
    }

    /// Helper method to transform a stream of inputs to a stream of outputs,
    /// with callbacks. Use this to implement `stream` or `transform` in
    /// subclasses.
    ///
    /// Mirrors Python's `Runnable._transform_stream_with_config`.
    fn transform_stream_with_config<'a>(
        &'a self,
        input: BoxStream<'a, Self::Input>,
        transformer: Box<
            dyn FnOnce(
                    BoxStream<'a, Self::Input>,
                    &RunnableConfig,
                ) -> BoxStream<'a, Result<Self::Output>>
                + Send
                + 'a,
        >,
        config: Option<RunnableConfig>,
    ) -> BoxStream<'a, Result<Self::Output>> {
        let config = ensure_config(config);
        let callback_manager = get_callback_manager_for_config(&config);
        let run_manager = callback_manager
            .on_chain_start()
            .serialized(&HashMap::new())
            .inputs(&HashMap::new())
            .maybe_run_id(config.run_id)
            .maybe_name(config.run_name.as_deref())
            .call();

        let child_config = patch_config(
            Some(config),
            Some(run_manager.get_child(None)),
            None,
            None,
            None,
            None,
        );

        let output_stream = transformer(input, &child_config);

        Box::pin(async_stream::stream! {
            let mut stream = output_stream;
            let mut had_error = false;

            while let Some(item) = stream.next().await {
                match &item {
                    Ok(_) => {}
                    Err(e) => {
                        if !had_error {
                            run_manager.on_chain_error(e as &dyn std::error::Error);
                            had_error = true;
                        }
                    }
                }
                yield item;
            }

            if !had_error {
                run_manager.on_chain_end(&HashMap::new());
            }
        })
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

    /// Get a JSON schema that represents the input to the Runnable.
    ///
    /// Mirrors `Runnable.get_input_jsonschema()` from
    /// `langchain_core.runnables.base`.
    fn get_input_jsonschema(&self, config: Option<&RunnableConfig>) -> Value {
        self.get_input_schema(config)
    }

    /// Get a JSON schema that represents the output of the Runnable.
    ///
    /// Mirrors `Runnable.get_output_jsonschema()` from
    /// `langchain_core.runnables.base`.
    fn get_output_jsonschema(&self, config: Option<&RunnableConfig>) -> Value {
        self.get_output_schema(config)
    }

    /// Get a JSON schema that represents the config of the Runnable.
    ///
    /// Mirrors `Runnable.get_config_jsonschema()` from
    /// `langchain_core.runnables.base`.
    fn get_config_jsonschema(&self, include: Option<&[&str]>) -> Result<Value> {
        let specs = self.config_specs()?;
        let include = include.unwrap_or(&[]);

        let mut properties = serde_json::Map::new();

        if !specs.is_empty() {
            let mut config_props = serde_json::Map::new();
            for spec in &specs {
                let mut prop = serde_json::Map::new();
                if let Some(ref name) = spec.name {
                    prop.insert("title".into(), Value::String(name.clone()));
                }
                if let Some(ref desc) = spec.description {
                    prop.insert("description".into(), Value::String(desc.clone()));
                }
                if let Some(ref default) = spec.default {
                    prop.insert("default".into(), default.clone());
                }
                prop.insert("type".into(), Value::String(spec.annotation.clone()));
                config_props.insert(spec.id.clone(), Value::Object(prop));
            }
            properties.insert(
                "configurable".into(),
                serde_json::json!({
                    "title": "Configurable",
                    "type": "object",
                    "properties": Value::Object(config_props),
                }),
            );
        }

        for &field in include {
            if field != "configurable" {
                properties.insert(field.into(), serde_json::json!({}));
            }
        }

        Ok(serde_json::json!({
            "title": format!("{}Config", self.get_name(None, None)),
            "type": "object",
            "properties": Value::Object(properties),
        }))
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

        if inputs.len() == 1 {
            let input = inputs.into_iter().next().expect("checked len == 1");
            let config = configs.into_iter().next().expect("checked len == 1");
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
                if let Some(max) = semaphore_like {
                    while active_count.load(Ordering::SeqCst) >= max {
                        std::thread::sleep(std::time::Duration::from_millis(1));
                    }
                }
                let active = active_count.clone();
                active.fetch_add(1, Ordering::SeqCst);

                let handle = scope.spawn(move || {
                    let result = if return_exceptions {
                        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            self.invoke(input, Some(config))
                        })) {
                            Ok(r) => r,
                            Err(panic_info) => {
                                let msg = panic_info
                                    .downcast_ref::<String>()
                                    .cloned()
                                    .or_else(|| {
                                        panic_info.downcast_ref::<&str>().map(|s| s.to_string())
                                    })
                                    .unwrap_or_else(|| "unknown panic".to_string());
                                Err(Error::other(format!("Panic in batch item: {msg}")))
                            }
                        }
                    } else {
                        self.invoke(input, Some(config))
                    };
                    active.fetch_sub(1, Ordering::SeqCst);
                    (i, result)
                });
                handles.push(handle);
            }

            for handle in handles {
                match handle.join() {
                    Ok((i, result)) => {
                        results[i] = Some(result);
                    }
                    Err(panic_info) => {
                        if !return_exceptions {
                            std::panic::resume_unwind(panic_info);
                        }
                    }
                }
            }
        });

        let collected: Vec<Result<Self::Output>> = results
            .into_iter()
            .map(|r| r.expect("all results populated by thread::scope"))
            .collect();

        if return_exceptions {
            collected
        } else {
            if let Some(first_err_idx) = collected.iter().position(|r| r.is_err()) {
                return collected
                    .into_iter()
                    .nth(first_err_idx)
                    .into_iter()
                    .collect();
            }
            collected
        }
    }

    /// Transform multiple inputs into outputs asynchronously.
    ///
    /// Default implementation runs ainvoke() concurrently, respecting the
    /// `max_concurrency` setting from config using a semaphore.
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
        let max_concurrency = configs[0].max_concurrency;

        let results = match max_concurrency {
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
        };

        if return_exceptions {
            results
        } else {
            if let Some(first_err_idx) = results.iter().position(|r| r.is_err()) {
                return results.into_iter().nth(first_err_idx).into_iter().collect();
            }
            results
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
        return_exceptions: bool,
    ) -> Vec<(usize, Result<Self::Output>)>
    where
        Self: 'static,
    {
        if inputs.is_empty() {
            return Vec::new();
        }

        let configs = get_config_list(config, inputs.len());

        if inputs.len() == 1 {
            let input = inputs.into_iter().next().expect("checked len == 1");
            let config = configs.into_iter().next().expect("checked len == 1");
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
                    let result = if return_exceptions {
                        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            self.invoke(input, Some(config))
                        })) {
                            Ok(r) => r,
                            Err(panic_info) => {
                                let msg = panic_info
                                    .downcast_ref::<String>()
                                    .cloned()
                                    .or_else(|| {
                                        panic_info.downcast_ref::<&str>().map(|s| s.to_string())
                                    })
                                    .unwrap_or_else(|| "unknown panic".to_string());
                                Err(Error::other(format!("Panic in batch item: {msg}")))
                            }
                        }
                    } else {
                        self.invoke(input, Some(config))
                    };
                    active.fetch_sub(1, Ordering::SeqCst);
                    tx.send((i, result))
                        .expect("receiver should not be dropped");
                });
            }

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

    /// Generate a stream of events.
    ///
    /// Use to create a stream of `StreamEvent` that provide real-time
    /// information about the progress of the Runnable, including events
    /// from intermediate results.
    ///
    /// Mirrors Python's `Runnable.astream_events()` (V2 implementation).
    fn astream_events<'a>(
        &'a self,
        input: Self::Input,
        config: Option<RunnableConfig>,
        include_names: Option<Vec<String>>,
        include_types: Option<Vec<String>>,
        include_tags: Option<Vec<String>>,
        exclude_names: Option<Vec<String>>,
        exclude_types: Option<Vec<String>>,
        exclude_tags: Option<Vec<String>>,
    ) -> BoxStream<'a, crate::runnables::schema::StreamEvent>
    where
        Self: 'static,
        Self::Output: serde::Serialize,
        Self: Sized,
    {
        crate::tracers::event_stream::astream_events_implementation(
            self,
            input,
            config,
            include_names,
            include_types,
            include_tags,
            exclude_names,
            exclude_types,
            exclude_tags,
        )
    }

    /// Generate a stream of log patches.
    ///
    /// Use to create a stream of `RunLogPatch` that provide real-time
    /// information about the progress of the Runnable.
    ///
    /// Mirrors Python's `Runnable.astream_log()`.
    fn astream_log<'a>(
        &'a self,
        input: Self::Input,
        config: Option<RunnableConfig>,
        diff: bool,
        with_streamed_output_list: bool,
        include_names: Option<Vec<String>>,
        include_types: Option<Vec<String>>,
        include_tags: Option<Vec<String>>,
        exclude_names: Option<Vec<String>>,
        exclude_types: Option<Vec<String>>,
        exclude_tags: Option<Vec<String>>,
    ) -> BoxStream<'a, crate::tracers::log_stream::RunLogPatch>
    where
        Self: 'static,
        Self::Output: serde::Serialize,
        Self: Sized,
    {
        crate::tracers::log_stream::astream_log_implementation(
            self,
            input,
            config,
            diff,
            with_streamed_output_list,
            include_names,
            include_types,
            include_tags,
            exclude_names,
            exclude_types,
            exclude_tags,
        )
    }

    /// Return a graph representation of this Runnable.
    ///
    /// The default implementation creates a simple 3-node graph:
    /// Input → Runnable → Output.
    ///
    /// Mirrors Python's `Runnable.get_graph()`.
    fn get_graph(&self, config: Option<&RunnableConfig>) -> Result<super::graph::Graph> {
        use super::graph::NodeData;
        let mut graph = super::graph::Graph::new();

        let input_node = graph.add_node(
            Some(NodeData::Schema {
                name: self.get_name(Some("Input"), None),
            }),
            None,
            None,
        );

        let metadata = config
            .map(|c| &c.metadata)
            .filter(|m| !m.is_empty())
            .cloned();
        let runnable_node = graph.add_node(
            Some(NodeData::Runnable {
                name: self.get_name(None, None),
            }),
            None,
            metadata,
        );

        let output_node = graph.add_node(
            Some(NodeData::Schema {
                name: self.get_name(Some("Output"), None),
            }),
            None,
            None,
        );

        graph.add_edge(&input_node, &runnable_node, None, false);
        graph.add_edge(&runnable_node, &output_node, None, false);

        Ok(graph)
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
    /// Compose this Runnable with another, returning a RunnableSequence.
    ///
    /// Mirrors Python's `Runnable.pipe()`.
    fn pipe<R2>(self, other: R2) -> RunnableSequence<Self, R2>
    where
        Self: Sized,
        R2: Runnable<Input = Self::Output>,
    {
        RunnableSequence::new(self, other)
    }

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

    /// Select keys from the output of this runnable.
    ///
    /// Returns a `RunnableSequence` that pipes this runnable's output through
    /// a `RunnablePick` to select specific keys from a dict output.
    ///
    /// Mirrors Python's `Runnable.pick()`.
    fn pick(
        self,
        keys: super::passthrough::PickKeys,
    ) -> RunnableSequence<Self, super::passthrough::RunnablePick>
    where
        Self: Sized + Runnable<Output = HashMap<String, Value>>,
    {
        pipe(self, super::passthrough::RunnablePick::from(keys))
    }

    /// Assign key-value pairs to dict outputs from this runnable.
    ///
    /// Returns a `RunnableSequence` that pipes this runnable's output through
    /// a `RunnableAssign` to merge additional computed fields into the output.
    ///
    /// Mirrors Python's `Runnable.assign()`.
    fn assign(
        self,
        mapper: super::passthrough::RunnableAssign,
    ) -> RunnableSequence<Self, super::passthrough::RunnableAssign>
    where
        Self: Sized + Runnable<Output = HashMap<String, Value>>,
    {
        pipe(self, mapper)
    }

    /// Add fallback runnables that are invoked if this runnable fails.
    ///
    /// Returns a `RunnableWithFallbacks` that tries this runnable first,
    /// then falls back to the provided alternatives on failure.
    ///
    /// Mirrors Python's `Runnable.with_fallbacks()`.
    fn with_fallbacks(
        self,
        fallbacks: Vec<DynRunnable<Self::Input, Self::Output>>,
    ) -> super::fallbacks::RunnableWithFallbacks<Self::Input, Self::Output>
    where
        Self: Sized + Send + Sync + 'static,
    {
        super::fallbacks::RunnableWithFallbacks::new(self, fallbacks)
    }

    /// Bind lifecycle listeners to this runnable.
    ///
    /// Creates a `RunnableBinding` with a config factory that adds a
    /// `RootListenersTracer` as a callback. The tracer invokes the provided
    /// listener functions on the root run's start, end, and error events.
    ///
    /// Mirrors Python's `Runnable.with_listeners()`.
    fn with_listeners(
        self,
        on_start: Option<crate::tracers::root_listeners::Listener>,
        on_end: Option<crate::tracers::root_listeners::Listener>,
        on_error: Option<crate::tracers::root_listeners::Listener>,
    ) -> RunnableBinding<Self>
    where
        Self: Sized,
    {
        let on_start: Option<
            Arc<
                dyn Fn(&crate::tracers::schemas::Run, &super::config::RunnableConfig) + Send + Sync,
            >,
        > = on_start.map(|f| {
            Arc::from(f)
                as Arc<
                    dyn Fn(&crate::tracers::schemas::Run, &super::config::RunnableConfig)
                        + Send
                        + Sync,
                >
        });
        let on_end: Option<
            Arc<
                dyn Fn(&crate::tracers::schemas::Run, &super::config::RunnableConfig) + Send + Sync,
            >,
        > = on_end.map(|f| {
            Arc::from(f)
                as Arc<
                    dyn Fn(&crate::tracers::schemas::Run, &super::config::RunnableConfig)
                        + Send
                        + Sync,
                >
        });
        let on_error: Option<
            Arc<
                dyn Fn(&crate::tracers::schemas::Run, &super::config::RunnableConfig) + Send + Sync,
            >,
        > = on_error.map(|f| {
            Arc::from(f)
                as Arc<
                    dyn Fn(&crate::tracers::schemas::Run, &super::config::RunnableConfig)
                        + Send
                        + Sync,
                >
        });

        let factory: ConfigFactory = Arc::new(move |config: &super::config::RunnableConfig| {
            use crate::callbacks::base::Callbacks;
            use crate::tracers::root_listeners::RootListenersTracer;

            let tracer = RootListenersTracer::new(
                config.clone(),
                on_start.as_ref().map(|f| {
                    Box::new({
                        let f = f.clone();
                        move |run: &crate::tracers::schemas::Run,
                              cfg: &super::config::RunnableConfig| {
                            f(run, cfg)
                        }
                    }) as crate::tracers::root_listeners::Listener
                }),
                on_end.as_ref().map(|f| {
                    Box::new({
                        let f = f.clone();
                        move |run: &crate::tracers::schemas::Run,
                              cfg: &super::config::RunnableConfig| {
                            f(run, cfg)
                        }
                    }) as crate::tracers::root_listeners::Listener
                }),
                on_error.as_ref().map(|f| {
                    Box::new({
                        let f = f.clone();
                        move |run: &crate::tracers::schemas::Run,
                              cfg: &super::config::RunnableConfig| {
                            f(run, cfg)
                        }
                    }) as crate::tracers::root_listeners::Listener
                }),
            );

            super::config::RunnableConfig {
                callbacks: Some(Callbacks::Handlers(vec![
                    Arc::new(tracer) as Arc<dyn crate::callbacks::base::BaseCallbackHandler>
                ])),
                ..Default::default()
            }
        });

        RunnableBinding::with_config_factories(self, HashMap::new(), None, vec![factory])
    }

    /// Bind async lifecycle listeners to this runnable.
    ///
    /// Creates a `RunnableBinding` with a config factory that adds an
    /// `AsyncRootListenersTracer` as a callback. The tracer invokes the provided
    /// async listener functions on the root run's start, end, and error events.
    ///
    /// Mirrors Python's `Runnable.with_alisteners()`.
    fn with_alisteners(
        self,
        on_start: Option<crate::tracers::root_listeners::AsyncListener>,
        on_end: Option<crate::tracers::root_listeners::AsyncListener>,
        on_error: Option<crate::tracers::root_listeners::AsyncListener>,
    ) -> RunnableBinding<Self>
    where
        Self: Sized,
    {
        let on_start: Option<
            Arc<
                dyn Fn(
                        &crate::tracers::schemas::Run,
                        &super::config::RunnableConfig,
                    ) -> futures::future::BoxFuture<'static, ()>
                    + Send
                    + Sync,
            >,
        > = on_start.map(|f| {
            Arc::from(f)
                as Arc<
                    dyn Fn(
                            &crate::tracers::schemas::Run,
                            &super::config::RunnableConfig,
                        ) -> futures::future::BoxFuture<'static, ()>
                        + Send
                        + Sync,
                >
        });
        let on_end: Option<
            Arc<
                dyn Fn(
                        &crate::tracers::schemas::Run,
                        &super::config::RunnableConfig,
                    ) -> futures::future::BoxFuture<'static, ()>
                    + Send
                    + Sync,
            >,
        > = on_end.map(|f| {
            Arc::from(f)
                as Arc<
                    dyn Fn(
                            &crate::tracers::schemas::Run,
                            &super::config::RunnableConfig,
                        ) -> futures::future::BoxFuture<'static, ()>
                        + Send
                        + Sync,
                >
        });
        let on_error: Option<
            Arc<
                dyn Fn(
                        &crate::tracers::schemas::Run,
                        &super::config::RunnableConfig,
                    ) -> futures::future::BoxFuture<'static, ()>
                    + Send
                    + Sync,
            >,
        > = on_error.map(|f| {
            Arc::from(f)
                as Arc<
                    dyn Fn(
                            &crate::tracers::schemas::Run,
                            &super::config::RunnableConfig,
                        ) -> futures::future::BoxFuture<'static, ()>
                        + Send
                        + Sync,
                >
        });

        let factory: ConfigFactory = Arc::new(move |config: &super::config::RunnableConfig| {
            use crate::callbacks::base::Callbacks;
            use crate::tracers::root_listeners::AsyncRootListenersTracer;

            let tracer = AsyncRootListenersTracer::new(
                config.clone(),
                on_start.as_ref().map(|f| {
                    Box::new({
                        let f = f.clone();
                        move |run: &crate::tracers::schemas::Run,
                              cfg: &crate::runnables::config::RunnableConfig|
                              -> futures::future::BoxFuture<'static, ()> {
                            f(run, cfg)
                        }
                    }) as crate::tracers::root_listeners::AsyncListener
                }),
                on_end.as_ref().map(|f| {
                    Box::new({
                        let f = f.clone();
                        move |run: &crate::tracers::schemas::Run,
                              cfg: &crate::runnables::config::RunnableConfig|
                              -> futures::future::BoxFuture<'static, ()> {
                            f(run, cfg)
                        }
                    }) as crate::tracers::root_listeners::AsyncListener
                }),
                on_error.as_ref().map(|f| {
                    Box::new({
                        let f = f.clone();
                        move |run: &crate::tracers::schemas::Run,
                              cfg: &crate::runnables::config::RunnableConfig|
                              -> futures::future::BoxFuture<'static, ()> {
                            f(run, cfg)
                        }
                    }) as crate::tracers::root_listeners::AsyncListener
                }),
            );

            super::config::RunnableConfig {
                callbacks: Some(Callbacks::Handlers(vec![
                    Arc::new(tracer) as Arc<dyn crate::callbacks::base::BaseCallbackHandler>
                ])),
                ..Default::default()
            }
        });

        RunnableBinding::with_config_factories(self, HashMap::new(), None, vec![factory])
    }

    /// List configurable fields for this Runnable.
    ///
    /// Mirrors Python's `Runnable.config_specs` property.
    fn config_specs(&self) -> Result<Vec<ConfigurableFieldSpec>> {
        Ok(vec![])
    }

    /// Return a list of prompts used by this Runnable.
    ///
    /// Mirrors Python's `Runnable.get_prompts()`.
    fn get_prompts(&self) -> Vec<Arc<dyn crate::BasePromptTemplate>> {
        vec![]
    }

    /// Convert this Runnable into a BaseTool.
    ///
    /// Mirrors Python's `Runnable.as_tool()`. Only available when
    /// Input and Output types are compatible with tool interfaces.
    fn as_tool(self: Arc<Self>, name: &str, description: &str) -> crate::tools::StructuredTool
    where
        Self: Sized + Runnable<Input = HashMap<String, Value>, Output = Value> + 'static,
    {
        crate::tools::convert_runnable_to_tool(self, name, description)
    }
}

/// Convert a string to title case.
/// Trait for objects that can provide a graph representation.
///
/// Used by `RunnableLambda` to store dependencies that contribute to its graph.
/// In Python, dependencies are detected via closure inspection;
/// in Rust, they must be set explicitly.
pub trait GraphProvider: Send + Sync + Debug {
    /// Return a graph representation of this object.
    fn provide_graph(&self, config: Option<&RunnableConfig>) -> Result<super::graph::Graph>;
}

/// Wrapper that adapts any Runnable into a GraphProvider.
pub struct RunnableGraphProvider<R: Runnable>(pub R);

impl<R: Runnable> Debug for RunnableGraphProvider<R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("RunnableGraphProvider")
            .field(&self.0)
            .finish()
    }
}

impl<R: Runnable> GraphProvider for RunnableGraphProvider<R> {
    fn provide_graph(&self, config: Option<&RunnableConfig>) -> Result<super::graph::Graph> {
        self.0.get_graph(config)
    }
}

/// Extract a short type name from a fully qualified Rust type path.
///
/// Mirrors Python's behavior where `self.__class__.__name__` returns just
/// the class name (e.g. "RunnableLambda") rather than the full module path.
/// Strips module paths and generic parameters.
fn short_type_name(full_name: &str) -> String {
    let base = full_name.split('<').next().unwrap_or(full_name);
    base.rsplit("::").next().unwrap_or(base).to_string()
}

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
    deps: Vec<Arc<dyn GraphProvider>>,
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
            deps: Vec::new(),
            _phantom: std::marker::PhantomData,
        }
    }

    /// Create a new RunnableLambda with a name.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Add a dependency runnable that contributes to this lambda's graph.
    ///
    /// In Python, dependencies are detected automatically by inspecting
    /// closure variables. In Rust, they must be set explicitly.
    pub fn with_dep(mut self, dep: Arc<dyn GraphProvider>) -> Self {
        self.deps.push(dep);
        self
    }

    /// Add multiple dependency runnables.
    pub fn with_deps(mut self, deps: Vec<Arc<dyn GraphProvider>>) -> Self {
        self.deps.extend(deps);
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
        self.call_with_config(&|input, _config| (self.func)(input), input, config)
    }

    fn stream(
        &self,
        input: Self::Input,
        config: Option<RunnableConfig>,
    ) -> BoxStream<'_, Result<Self::Output>> {
        let result = self.invoke(input, config);
        Box::pin(futures::stream::once(async move { result }))
    }

    fn transform<'a>(
        &'a self,
        input: BoxStream<'a, Self::Input>,
        config: Option<RunnableConfig>,
    ) -> BoxStream<'a, Result<Self::Output>> {
        self.transform_stream_with_config(
            input,
            Box::new(move |input_stream, _config| {
                Box::pin(async_stream::stream! {
                    let mut stream = input_stream;
                    let mut final_input: Option<I> = None;
                    while let Some(ichunk) = stream.next().await {
                        final_input = Some(ichunk);
                    }
                    if let Some(input_val) = final_input {
                        yield (self.func)(input_val);
                    }
                })
            }),
            config,
        )
    }

    fn get_graph(&self, config: Option<&RunnableConfig>) -> Result<super::graph::Graph> {
        if self.deps.is_empty() {
            use super::graph::NodeData;
            let mut graph = super::graph::Graph::new();

            let input_node = graph.add_node(
                Some(NodeData::Schema {
                    name: self.get_name(Some("Input"), None),
                }),
                None,
                None,
            );

            let metadata = config
                .map(|c| &c.metadata)
                .filter(|m| !m.is_empty())
                .cloned();
            let runnable_node = graph.add_node(
                Some(NodeData::Runnable {
                    name: self.get_name(None, None),
                }),
                None,
                metadata,
            );

            let output_node = graph.add_node(
                Some(NodeData::Schema {
                    name: self.get_name(Some("Output"), None),
                }),
                None,
                None,
            );

            graph.add_edge(&input_node, &runnable_node, None, false);
            graph.add_edge(&runnable_node, &output_node, None, false);

            Ok(graph)
        } else {
            use super::graph::NodeData;
            let mut graph = super::graph::Graph::new();

            let input_node = graph.add_node(
                Some(NodeData::Schema {
                    name: self.get_name(Some("Input"), None),
                }),
                None,
                None,
            );
            let output_node = graph.add_node(
                Some(NodeData::Schema {
                    name: self.get_name(Some("Output"), None),
                }),
                None,
                None,
            );

            for dep in &self.deps {
                let mut dep_graph = dep.provide_graph(None)?;
                dep_graph.trim_first_node();
                dep_graph.trim_last_node();

                if dep_graph.nodes.is_empty() {
                    graph.add_edge(&input_node, &output_node, None, false);
                } else {
                    let (dep_first, dep_last) = graph.extend(dep_graph, "");
                    let dep_first = dep_first
                        .ok_or_else(|| Error::other("RunnableLambda dep has no first node"))?;
                    let dep_last = dep_last
                        .ok_or_else(|| Error::other("RunnableLambda dep has no last node"))?;
                    graph.add_edge(&input_node, &dep_first, None, false);
                    graph.add_edge(&dep_last, &output_node, None, false);
                }
            }

            Ok(graph)
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

        self.call_with_config(
            &|input, config| call_func_with_variable_args(func, input, config),
            input,
            config,
        )
    }

    async fn ainvoke(
        &self,
        input: Self::Input,
        config: Option<RunnableConfig>,
    ) -> Result<Self::Output>
    where
        Self: 'static,
    {
        if let Some(afunc) = &self.afunc {
            self.acall_with_config(
                &|input, config: RunnableConfig| {
                    let result = match afunc {
                        AsyncVariableArgsFn::InputOnly(f) => f(input),
                        AsyncVariableArgsFn::WithConfig(f) => f(input, config),
                    };
                    Box::pin(result)
                },
                input,
                config,
            )
            .await
        } else if let Some(func) = &self.func {
            self.call_with_config(
                &|input, config| call_func_with_variable_args(func, input, config),
                input,
                config,
            )
        } else {
            Err(Error::other(
                "RunnableLambdaWithConfig has no func or afunc",
            ))
        }
    }

    fn stream(
        &self,
        input: Self::Input,
        config: Option<RunnableConfig>,
    ) -> BoxStream<'_, Result<Self::Output>> {
        let result = self.invoke(input, config);
        Box::pin(futures::stream::once(async move { result }))
    }

    fn transform<'a>(
        &'a self,
        input: BoxStream<'a, Self::Input>,
        config: Option<RunnableConfig>,
    ) -> BoxStream<'a, Result<Self::Output>> {
        self.transform_stream_with_config(
            input,
            Box::new(move |input_stream, config| {
                let config = config.clone();
                Box::pin(async_stream::stream! {
                    let mut stream = input_stream;
                    let mut final_input: Option<I> = None;
                    while let Some(ichunk) = stream.next().await {
                        final_input = Some(ichunk);
                    }
                    if let Some(input_val) = final_input {
                        if let Some(func) = &self.func {
                            yield call_func_with_variable_args(func, input_val, &config);
                        } else {
                            yield Err(Error::other(
                                "Cannot transform synchronously without a sync function",
                            ));
                        }
                    }
                })
            }),
            config,
        )
    }

    fn atransform<'a>(
        &'a self,
        input: BoxStream<'a, Self::Input>,
        config: Option<RunnableConfig>,
    ) -> BoxStream<'a, Result<Self::Output>>
    where
        Self: 'static,
    {
        if let Some(afunc) = &self.afunc {
            self.transform_stream_with_config(
                input,
                Box::new(move |input_stream, config| {
                    let config = config.clone();
                    Box::pin(async_stream::stream! {
                        let mut stream = input_stream;
                        let mut final_input: Option<I> = None;
                        while let Some(ichunk) = stream.next().await {
                            final_input = Some(ichunk);
                        }
                        if let Some(input_val) = final_input {
                            let result = acall_func_with_variable_args(
                                afunc, input_val, &config
                            ).await;
                            yield result;
                        }
                    })
                }),
                config,
            )
        } else if self.func.is_some() {
            self.transform(input, config)
        } else {
            Box::pin(futures::stream::once(async {
                Err(Error::other(
                    "RunnableLambdaWithConfig has no func or afunc",
                ))
            }))
        }
    }
}


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

    fn config_specs(&self) -> Result<Vec<ConfigurableFieldSpec>> {
        let mut specs = self.first.config_specs()?;
        specs.extend(self.last.config_specs()?);
        get_unique_config_specs(specs).map_err(Error::other)
    }

    fn get_prompts(&self) -> Vec<Arc<dyn crate::BasePromptTemplate>> {
        let mut prompts = self.first.get_prompts();
        prompts.extend(self.last.get_prompts());
        prompts
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
        let input_stream = Box::pin(futures::stream::once(async move { input }));
        self.transform(input_stream, config)
    }

    fn transform<'a>(
        &'a self,
        input: BoxStream<'a, Self::Input>,
        config: Option<RunnableConfig>,
    ) -> BoxStream<'a, Result<Self::Output>> {
        Box::pin(async_stream::stream! {
            let config = ensure_config(config);

            let first_output = self.first.transform(input, Some(config.clone()));

            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
            let mut first_stream = std::pin::pin!(first_output);

            let mut had_error = false;
            while let Some(result) = first_stream.next().await {
                match result {
                    Ok(value) => {
                        if tx.send(value).is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        yield Err(e);
                        had_error = true;
                        break;
                    }
                }
            }
            drop(tx);

            if !had_error {
                let rx_stream: BoxStream<'_, R1::Output> = Box::pin(async_stream::stream! {
                    while let Some(value) = rx.recv().await {
                        yield value;
                    }
                });
                let mut second_output = self.last.transform(rx_stream, Some(config));
                while let Some(result) = second_output.next().await {
                    yield result;
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
        self.transform(input, config)
    }

    fn get_graph(&self, config: Option<&RunnableConfig>) -> Result<super::graph::Graph> {
        let mut graph = super::graph::Graph::new();

        let mut first_graph = self.first.get_graph(config)?;
        first_graph.trim_last_node();
        let (step_first, _) = graph.extend(first_graph, "");
        if step_first.is_none() {
            return Err(Error::other(
                "RunnableSequence first step has no first node",
            ));
        }

        let mut last_graph = self.last.get_graph(config)?;
        last_graph.trim_first_node();
        let current_last = graph.last_node().cloned();
        let (step_first, _) = graph.extend(last_graph, "");
        let step_first = step_first
            .ok_or_else(|| Error::other("RunnableSequence last step has no first node"))?;
        if let Some(last) = current_last {
            graph.add_edge(&last, &step_first, None, false);
        }

        Ok(graph)
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

/// Type alias mirroring Python's `RunnableMap = RunnableParallel`.
pub type RunnableMap<I> = RunnableParallel<I>;

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

impl<I> From<HashMap<String, Arc<dyn Runnable<Input = I, Output = Value> + Send + Sync>>>
    for RunnableParallel<I>
where
    I: Send + Sync + Clone + Debug + 'static,
{
    fn from(
        steps: HashMap<String, Arc<dyn Runnable<Input = I, Output = Value> + Send + Sync>>,
    ) -> Self {
        Self { steps, name: None }
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

    fn config_specs(&self) -> Result<Vec<ConfigurableFieldSpec>> {
        let mut specs = Vec::new();
        for step in self.steps.values() {
            specs.extend(step.config_specs()?);
        }
        get_unique_config_specs(specs).map_err(Error::other)
    }

    fn get_prompts(&self) -> Vec<Arc<dyn crate::BasePromptTemplate>> {
        let mut prompts = Vec::new();
        for step in self.steps.values() {
            prompts.extend(step.get_prompts());
        }
        prompts
    }

    fn invoke(&self, input: Self::Input, config: Option<RunnableConfig>) -> Result<Self::Output> {
        let config = ensure_config(config);
        let callback_manager = get_callback_manager_for_config(&config);
        let run_manager = callback_manager
            .on_chain_start()
            .serialized(&HashMap::new())
            .inputs(&HashMap::new())
            .maybe_run_id(config.run_id)
            .maybe_name(config.run_name.as_deref())
            .call();

        let step_entries: Vec<_> = self.steps.iter().collect();
        let mut results = HashMap::new();

        let outcome: Result<()> = std::thread::scope(|scope| {
            let handles: Vec<_> = step_entries
                .iter()
                .map(|(key, step)| {
                    let input = input.clone();
                    let child_config = patch_config(
                        Some(config.clone()),
                        Some(run_manager.get_child(Some(&format!("map:key:{}", key)))),
                        None,
                        None,
                        None,
                        None,
                    );
                    let key = (*key).clone();
                    scope.spawn(move || {
                        let _context_guard = set_config_context(child_config.clone());
                        let result = step.invoke(input, Some(child_config));
                        (key, result)
                    })
                })
                .collect();

            for handle in handles {
                let (key, result) = handle.join().expect("thread should not panic");
                results.insert(key, result?);
            }

            Ok(())
        });

        match outcome {
            Ok(()) => {
                run_manager.on_chain_end(&HashMap::new());
                Ok(results)
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
            .on_chain_start(
                &HashMap::new(),
                &HashMap::new(),
                config.run_id,
                config.run_name.as_deref(),
            )
            .await;

        let futures: Vec<_> = self
            .steps
            .iter()
            .map(|(key, step)| {
                let input = input.clone();
                let child_config = patch_config(
                    Some(config.clone()),
                    Some(
                        run_manager
                            .get_child(Some(&format!("map:key:{}", key)))
                            .to_callback_manager(),
                    ),
                    None,
                    None,
                    None,
                    None,
                );
                let key = key.clone();
                async move {
                    let result = step.ainvoke(input, Some(child_config)).await;
                    (key, result)
                }
            })
            .collect();

        let completed = futures::future::join_all(futures).await;

        let mut results = HashMap::new();
        let mut error: Option<Error> = None;
        for (key, result) in completed {
            match result {
                Ok(value) => {
                    results.insert(key, value);
                }
                Err(e) => {
                    error = Some(e);
                    break;
                }
            }
        }

        match error {
            None => {
                run_manager.get_sync().on_chain_end(&HashMap::new());
                Ok(results)
            }
            Some(e) => {
                run_manager.get_sync().on_chain_error(&e);
                Err(e)
            }
        }
    }

    fn stream(
        &self,
        input: Self::Input,
        config: Option<RunnableConfig>,
    ) -> BoxStream<'_, Result<Self::Output>> {
        Box::pin(async_stream::stream! {
            let config = ensure_config(config);

            let mut tagged_streams: futures::stream::SelectAll<
                BoxStream<'_, Result<(String, Value)>>
            > = futures::stream::SelectAll::new();

            for (name, step) in &self.steps {
                let name = name.clone();
                let step_stream = step.stream(input.clone(), Some(config.clone()));
                let named_stream = step_stream.map(move |result| {
                    result.map(|value| (name.clone(), value))
                });
                tagged_streams.push(Box::pin(named_stream));
            }

            while let Some(result) = tagged_streams.next().await {
                match result {
                    Ok((key, value)) => {
                        let mut chunk = HashMap::new();
                        chunk.insert(key, value);
                        yield Ok(chunk);
                    }
                    Err(e) => {
                        yield Err(e);
                    }
                }
            }
        })
    }

    fn transform<'a>(
        &'a self,
        input: BoxStream<'a, Self::Input>,
        config: Option<RunnableConfig>,
    ) -> BoxStream<'a, Result<Self::Output>> {
        let num_steps = self.steps.len();
        if num_steps == 0 {
            return Box::pin(futures::stream::empty());
        }

        Box::pin(async_stream::stream! {
            let config = ensure_config(config);

            let input_chunks: Vec<Self::Input> = input.collect().await;

            let mut tagged_streams: futures::stream::SelectAll<
                BoxStream<'_, Result<(String, Value)>>
            > = futures::stream::SelectAll::new();

            for (name, step) in &self.steps {
                let name = name.clone();
                let branch_input: BoxStream<'_, Self::Input> =
                    Box::pin(futures::stream::iter(input_chunks.clone()));
                let branch_config = patch_config(
                    Some(config.clone()),
                    None,
                    None,
                    None,
                    None,
                    None,
                );
                let branch_output = step.transform(branch_input, Some(branch_config));
                let named_stream = branch_output.map(move |result| {
                    result.map(|value| (name.clone(), value))
                });
                tagged_streams.push(Box::pin(named_stream));
            }

            while let Some(result) = tagged_streams.next().await {
                match result {
                    Ok((key, value)) => {
                        let mut chunk = HashMap::new();
                        chunk.insert(key, value);
                        yield Ok(chunk);
                    }
                    Err(e) => {
                        yield Err(e);
                    }
                }
            }
        })
    }

    fn get_graph(&self, _config: Option<&RunnableConfig>) -> Result<super::graph::Graph> {
        use super::graph::NodeData;
        let mut graph = super::graph::Graph::new();

        let input_node = graph.add_node(
            Some(NodeData::Schema {
                name: self.get_name(Some("Input"), None),
            }),
            None,
            None,
        );
        let output_node = graph.add_node(
            Some(NodeData::Schema {
                name: self.get_name(Some("Output"), None),
            }),
            None,
            None,
        );

        for step in self.steps.values() {
            let mut step_graph = step.get_graph(None)?;
            step_graph.trim_first_node();
            step_graph.trim_last_node();

            if step_graph.nodes.is_empty() {
                graph.add_edge(&input_node, &output_node, None, false);
            } else {
                let (first, last) = graph.extend(step_graph, "");
                let first = first.ok_or_else(|| Error::other("Parallel step has no first node"))?;
                let last = last.ok_or_else(|| Error::other("Parallel step has no last node"))?;
                graph.add_edge(&input_node, &first, None, false);
                graph.add_edge(&last, &output_node, None, false);
            }
        }

        Ok(graph)
    }
}


/// A Runnable that binds arguments or config to another Runnable.
pub struct RunnableBinding<R>
where
    R: Runnable,
{
    bound: R,
    kwargs: HashMap<String, Value>,
    config: Option<RunnableConfig>,
    config_factories: Vec<ConfigFactory>,
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
            .field("config_factories_count", &self.config_factories.len())
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
            config_factories: Vec::new(),
        }
    }

    /// Create a new RunnableBinding with config factories.
    ///
    /// Config factories are lazily evaluated functions that produce config
    /// overrides. They are applied after merging the bound and provided configs.
    pub fn with_config_factories(
        bound: R,
        kwargs: HashMap<String, Value>,
        config: Option<RunnableConfig>,
        config_factories: Vec<ConfigFactory>,
    ) -> Self {
        Self {
            bound,
            kwargs,
            config,
            config_factories,
        }
    }

    /// Merge configs for the binding.
    ///
    /// Mirrors Python's `RunnableBinding._merge_configs`: merges the bound config
    /// with the provided config, then applies each config factory to the result.
    fn merge_configs(&self, config: Option<RunnableConfig>) -> RunnableConfig {
        let merged = merge_configs(vec![self.config.clone(), config]);
        if self.config_factories.is_empty() {
            merged
        } else {
            let factory_configs: Vec<Option<RunnableConfig>> = self
                .config_factories
                .iter()
                .map(|f| Some(f(&merged)))
                .collect();
            let mut all = vec![Some(merged)];
            all.extend(factory_configs);
            merge_configs(all)
        }
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

    fn config_specs(&self) -> Result<Vec<ConfigurableFieldSpec>> {
        self.bound.config_specs()
    }

    fn get_prompts(&self) -> Vec<Arc<dyn crate::BasePromptTemplate>> {
        self.bound.get_prompts()
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

    fn get_graph(&self, config: Option<&RunnableConfig>) -> Result<super::graph::Graph> {
        self.bound.get_graph(config)
    }
}


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
        self.call_with_config(
            &|inputs: Vec<R::Input>, config: &RunnableConfig| {
                let configs = super::config::ConfigOrList::List(
                    inputs.iter().map(|_| config.clone()).collect(),
                );
                let results = self.bound.batch(inputs, Some(configs), false);
                results.into_iter().collect()
            },
            inputs,
            config,
        )
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
        let async_callback_manager = get_async_callback_manager_for_config(&config);
        let run_manager = async_callback_manager
            .on_chain_start(
                &HashMap::new(),
                &HashMap::new(),
                config.run_id,
                config.run_name.as_deref(),
            )
            .await;

        let child_config = patch_config(
            Some(config),
            Some(run_manager.get_child(None).to_callback_manager()),
            None,
            None,
            None,
            None,
        );

        let configs = super::config::ConfigOrList::List(
            inputs.iter().map(|_| child_config.clone()).collect(),
        );
        let results = self.bound.abatch(inputs, Some(configs), false).await;

        match results.iter().find(|r| r.is_err()) {
            None => {
                run_manager.get_sync().on_chain_end(&HashMap::new());
                results.into_iter().collect()
            }
            Some(_) => {
                let collected: Result<Vec<R::Output>> = results.into_iter().collect();
                let e = collected.unwrap_err();
                run_manager.get_sync().on_chain_error(&e);
                Err(e)
            }
        }
    }
}


/// Type alias for a transform function that takes an input stream and produces
/// an output stream.
pub type TransformFn<I, O> =
    Arc<dyn Fn(BoxStream<'_, I>) -> BoxStream<'_, Result<O>> + Send + Sync>;

/// A Runnable that wraps a transform function (input stream -> output stream).
///
/// This is the primary mechanism for custom streaming transforms in chains.
/// The wrapped function receives a stream of inputs and produces a stream of
/// outputs, enabling chunk-by-chunk processing without buffering.
///
/// Mirrors Python's `RunnableGenerator`.
///
/// # Example
///
/// ```rust,ignore
/// use agent_chain_core::runnables::base::RunnableGenerator;
///
/// let generator = RunnableGenerator::<String, String>::new(|input_stream| {
///     Box::pin(async_stream::stream! {
///         let mut stream = input_stream;
///         while let Some(chunk) = stream.next().await {
///             yield Ok(format!("processed: {}", chunk));
///         }
///     })
/// });
/// ```
pub struct RunnableGenerator<I, O>
where
    I: Send + Sync + Clone + Debug + 'static,
    O: Send + Sync + Clone + Debug + Addable + 'static,
{
    transform_fn: TransformFn<I, O>,
    name: Option<String>,
}

impl<I, O> Debug for RunnableGenerator<I, O>
where
    I: Send + Sync + Clone + Debug + 'static,
    O: Send + Sync + Clone + Debug + Addable + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RunnableGenerator")
            .field("name", &self.name)
            .finish()
    }
}

impl<I, O> RunnableGenerator<I, O>
where
    I: Send + Sync + Clone + Debug + 'static,
    O: Send + Sync + Clone + Debug + Addable + 'static,
{
    /// Create a new RunnableGenerator from a transform function.
    ///
    /// The transform function takes a `BoxStream<I>` and returns a
    /// `BoxStream<Result<O>>`.
    pub fn new<F>(transform_fn: F) -> Self
    where
        F: Fn(BoxStream<'_, I>) -> BoxStream<'_, Result<O>> + Send + Sync + 'static,
    {
        Self {
            transform_fn: Arc::new(transform_fn),
            name: None,
        }
    }

    /// Set the name of this RunnableGenerator.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }
}

#[async_trait]
impl<I, O> Runnable for RunnableGenerator<I, O>
where
    I: Send + Sync + Clone + Debug + 'static,
    O: Send + Sync + Clone + Debug + Addable + 'static,
{
    type Input = I;
    type Output = O;

    fn name(&self) -> Option<String> {
        self.name.clone()
    }

    fn invoke(&self, input: Self::Input, config: Option<RunnableConfig>) -> Result<Self::Output> {
        let rt = tokio::runtime::Handle::try_current()
            .map_err(|_| Error::other("RunnableGenerator::invoke requires a tokio runtime"))?;

        rt.block_on(async {
            let mut stream = self.stream(input, config);
            let mut final_output: Option<Self::Output> = None;
            while let Some(result) = stream.next().await {
                let chunk = result?;
                final_output = Some(match final_output {
                    None => chunk,
                    Some(prev) => prev.add(chunk),
                });
            }
            final_output.ok_or_else(|| Error::other("RunnableGenerator produced no output"))
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
        let mut stream = self.astream(input, config);
        let mut final_output: Option<Self::Output> = None;
        while let Some(result) = stream.next().await {
            let chunk = result?;
            final_output = Some(match final_output {
                None => chunk,
                Some(prev) => prev.add(chunk),
            });
        }
        final_output.ok_or_else(|| Error::other("RunnableGenerator produced no output"))
    }

    fn stream(
        &self,
        input: Self::Input,
        config: Option<RunnableConfig>,
    ) -> BoxStream<'_, Result<Self::Output>> {
        let input_stream = Box::pin(futures::stream::once(async move { input }));
        self.transform(input_stream, config)
    }

    fn astream(
        &self,
        input: Self::Input,
        config: Option<RunnableConfig>,
    ) -> BoxStream<'_, Result<Self::Output>>
    where
        Self: 'static,
    {
        self.stream(input, config)
    }

    fn transform<'a>(
        &'a self,
        input: BoxStream<'a, Self::Input>,
        config: Option<RunnableConfig>,
    ) -> BoxStream<'a, Result<Self::Output>> {
        self.transform_stream_with_config(
            input,
            Box::new(move |stream, _config| (self.transform_fn)(stream)),
            config,
        )
    }

    fn atransform<'a>(
        &'a self,
        input: BoxStream<'a, Self::Input>,
        config: Option<RunnableConfig>,
    ) -> BoxStream<'a, Result<Self::Output>>
    where
        Self: 'static,
    {
        self.transform(input, config)
    }
}


/// A type-erased Runnable that can be stored in collections.
pub type DynRunnable<I, O> = Arc<dyn Runnable<Input = I, Output = O> + Send + Sync>;

/// Convert any Runnable into a DynRunnable.
pub fn to_dyn<R>(runnable: R) -> DynRunnable<R::Input, R::Output>
where
    R: Runnable + Send + Sync + 'static,
{
    Arc::new(runnable)
}


/// Coerce a function into a Runnable.
pub fn coerce_to_runnable<F, I, O>(func: F) -> RunnableLambda<F, I, O>
where
    F: Fn(I) -> Result<O> + Send + Sync,
    I: Send + Sync + Clone + Debug + 'static,
    O: Send + Sync + Clone + Debug + 'static,
{
    RunnableLambda::new(func)
}

/// Coerce a HashMap of Runnables into a RunnableParallel.
///
/// Mirrors the dict-to-RunnableParallel coercion in Python's `coerce_to_runnable()`.
pub fn coerce_map_to_runnable<I>(
    map: HashMap<String, Arc<dyn Runnable<Input = I, Output = Value> + Send + Sync>>,
) -> RunnableParallel<I>
where
    I: Send + Sync + Clone + Debug + 'static,
{
    RunnableParallel::from(map)
}

/// Decorate a function to make it a Runnable.
///
/// Sets the name of the Runnable to the given name.
/// Any runnables called by the function will be traced as dependencies.
///
/// Mirrors Python's `@chain` decorator from `langchain_core.runnables.base`.
///
/// # Example
///
/// ```rust,ignore
/// use agent_chain_core::runnables::base::chain;
///
/// let my_chain = chain("my_func", |input: String| {
///     Ok(format!("Hello, {input}!"))
/// });
/// ```
pub fn chain<F, I, O>(name: &str, func: F) -> RunnableLambda<F, I, O>
where
    F: Fn(I) -> Result<O> + Send + Sync,
    I: Send + Sync + Clone + Debug + 'static,
    O: Send + Sync + Clone + Debug + 'static,
{
    RunnableLambda::new(func).with_name(name)
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


    #[tokio::test]
    async fn test_runnable_generator_stream() {
        let generator = RunnableGenerator::<String, String>::new(|input_stream| {
            Box::pin(async_stream::stream! {
                use futures::StreamExt;
                let mut stream = input_stream;
                while let Some(chunk) = stream.next().await {
                    yield Ok(format!("processed: {}", chunk));
                }
            })
        });

        let chunks: Vec<_> = generator
            .stream("hello".to_string(), None)
            .collect::<Vec<_>>()
            .await;

        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].as_ref().unwrap(), "processed: hello");
    }

    #[tokio::test]
    async fn test_runnable_generator_transform() {
        let generator = RunnableGenerator::<i32, String>::new(|input_stream| {
            Box::pin(async_stream::stream! {
                use futures::StreamExt;
                let mut stream = input_stream;
                while let Some(num) = stream.next().await {
                    yield Ok(format!("num:{}", num));
                }
            })
        });

        let input = Box::pin(futures::stream::iter(vec![1, 2, 3]));
        let chunks: Vec<_> = generator.transform(input, None).collect::<Vec<_>>().await;

        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].as_ref().unwrap(), "num:1");
        assert_eq!(chunks[1].as_ref().unwrap(), "num:2");
        assert_eq!(chunks[2].as_ref().unwrap(), "num:3");
    }

    #[tokio::test]
    async fn test_runnable_generator_ainvoke() {
        let generator = RunnableGenerator::<String, String>::new(|input_stream| {
            Box::pin(async_stream::stream! {
                use futures::StreamExt;
                let mut stream = input_stream;
                while let Some(_chunk) = stream.next().await {
                    yield Ok("Have".to_string());
                    yield Ok(" a nice day".to_string());
                }
            })
        });

        let result = generator.ainvoke("input".to_string(), None).await.unwrap();
        assert_eq!(result, "Have a nice day");
    }

    #[tokio::test]
    async fn test_runnable_lambda_stream() {
        let runnable = RunnableLambda::new(|x: i32| Ok(x + 1));
        let chunks: Vec<_> = runnable.stream(1, None).collect::<Vec<_>>().await;

        assert_eq!(chunks.len(), 1);
        assert_eq!(*chunks[0].as_ref().unwrap(), 2);
    }

    #[tokio::test]
    async fn test_runnable_lambda_transform() {
        let runnable = RunnableLambda::new(|x: i32| Ok(x * 10));
        let input = Box::pin(futures::stream::iter(vec![1, 2, 3]));
        let chunks: Vec<_> = runnable.transform(input, None).collect::<Vec<_>>().await;

        assert_eq!(chunks.len(), 1);
        assert_eq!(*chunks[0].as_ref().unwrap(), 30); // 3 * 10
    }

    #[tokio::test]
    async fn test_runnable_sequence_stream_pipes_correctly() {
        let first = RunnableLambda::new(|x: i32| Ok(x + 1));
        let second = RunnableLambda::new(|x: i32| Ok(x * 2));
        let sequence = pipe(first, second);

        let chunks: Vec<_> = sequence.stream(1, None).collect::<Vec<_>>().await;

        assert_eq!(chunks.len(), 1);
        assert_eq!(*chunks[0].as_ref().unwrap(), 4); // (1+1)*2
    }

    #[tokio::test]
    async fn test_runnable_sequence_transform() {
        let first = RunnableLambda::new(|x: i32| Ok(x + 1));
        let second = RunnableLambda::new(|x: i32| Ok(x * 2));
        let sequence = pipe(first, second);

        let input = Box::pin(futures::stream::iter(vec![5]));
        let chunks: Vec<_> = sequence.transform(input, None).collect::<Vec<_>>().await;

        assert_eq!(chunks.len(), 1);
        assert_eq!(*chunks[0].as_ref().unwrap(), 12); // (5+1)*2
    }

    #[tokio::test]
    async fn test_nested_sequence_stream() {
        let a = RunnableLambda::new(|x: i32| Ok(x + 1));
        let b = RunnableLambda::new(|x: i32| Ok(x * 2));
        let c = RunnableLambda::new(|x: i32| Ok(x + 10));
        let chain = pipe(pipe(a, b), c);

        let chunks: Vec<_> = chain.stream(1, None).collect::<Vec<_>>().await;

        assert_eq!(chunks.len(), 1);
        assert_eq!(*chunks[0].as_ref().unwrap(), 14); // ((1+1)*2)+10
    }

    #[tokio::test]
    async fn test_runnable_parallel_stream() {
        let parallel = RunnableParallel::<Value>::new()
            .add(
                "double",
                RunnableLambda::new(|x: Value| {
                    let n = x.as_i64().unwrap_or(0);
                    Ok(serde_json::json!(n * 2))
                }),
            )
            .add(
                "triple",
                RunnableLambda::new(|x: Value| {
                    let n = x.as_i64().unwrap_or(0);
                    Ok(serde_json::json!(n * 3))
                }),
            );

        let chunks: Vec<_> = parallel
            .stream(serde_json::json!(5), None)
            .collect::<Vec<_>>()
            .await;

        assert_eq!(chunks.len(), 2);

        let mut combined = HashMap::new();
        for chunk in chunks {
            let map = chunk.unwrap();
            combined.extend(map);
        }

        assert_eq!(combined.get("double"), Some(&serde_json::json!(10)));
        assert_eq!(combined.get("triple"), Some(&serde_json::json!(15)));
    }

    #[tokio::test]
    async fn test_runnable_parallel_stream_matches_invoke() {
        let parallel = RunnableParallel::<Value>::new()
            .add(
                "a",
                RunnableLambda::new(|x: Value| Ok(serde_json::json!(x.as_i64().unwrap_or(0) + 1))),
            )
            .add(
                "b",
                RunnableLambda::new(|x: Value| Ok(serde_json::json!(x.as_i64().unwrap_or(0) * 2))),
            );

        let invoke_result = parallel.invoke(serde_json::json!(3), None).unwrap();

        let stream_chunks: Vec<_> = parallel
            .stream(serde_json::json!(3), None)
            .collect::<Vec<_>>()
            .await;

        let mut stream_combined = HashMap::new();
        for chunk in stream_chunks {
            stream_combined.extend(chunk.unwrap());
        }

        assert_eq!(invoke_result, stream_combined);
    }

    #[tokio::test]
    async fn test_generator_in_sequence() {
        let lambda = RunnableLambda::new(|x: i32| Ok(x + 1));
        let generator = RunnableGenerator::<i32, String>::new(|input_stream| {
            Box::pin(async_stream::stream! {
                use futures::StreamExt;
                let mut stream = input_stream;
                while let Some(num) = stream.next().await {
                    yield Ok(format!("val:{}", num));
                }
            })
        });
        let chain = pipe(lambda, generator);

        let chunks: Vec<_> = chain.stream(5, None).collect::<Vec<_>>().await;

        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].as_ref().unwrap(), "val:6");
    }

    #[tokio::test]
    async fn test_sequence_stream_error_propagation() {
        let first = RunnableLambda::new(|_x: i32| -> Result<i32> {
            Err(Error::other("first step failed"))
        });
        let second = RunnableLambda::new(|x: i32| Ok(x * 2));
        let sequence = pipe(first, second);

        let chunks: Vec<_> = sequence.stream(1, None).collect::<Vec<_>>().await;

        assert_eq!(chunks.len(), 1);
        assert!(chunks[0].is_err());
    }
}
