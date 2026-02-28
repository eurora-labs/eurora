use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;

use async_trait::async_trait;
use futures::stream::BoxStream;
use serde_json::Value;

use crate::error::{Error, Result};

use super::base::Runnable;
use super::config::{ConfigOrList, RunnableConfig, ensure_config, get_config_list, merge_configs};
use super::utils::{
    AnyConfigurableField, ConfigurableField, ConfigurableFieldMultiOption,
    ConfigurableFieldSingleOption, ConfigurableFieldSpec, gather_with_concurrency,
    get_unique_config_specs,
};

pub trait Reconfigurable: Runnable {
    fn reconfigure(
        &self,
        fields: &HashMap<String, Value>,
    ) -> Option<Arc<dyn Runnable<Input = Self::Input, Output = Self::Output> + Send + Sync>>;
}

pub fn prefix_config_spec(spec: &ConfigurableFieldSpec, prefix: &str) -> ConfigurableFieldSpec {
    if spec.is_shared {
        spec.clone()
    } else {
        ConfigurableFieldSpec {
            id: format!("{}/{}", prefix, spec.id),
            annotation: spec.annotation.clone(),
            name: spec.name.clone(),
            description: spec.description.clone(),
            default: spec.default.clone(),
            is_shared: spec.is_shared,
            dependencies: spec.dependencies.clone(),
        }
    }
}

pub fn make_options_spec_single(
    spec: &ConfigurableFieldSingleOption,
    description: Option<&str>,
) -> ConfigurableFieldSpec {
    let options_str = spec.options.keys().cloned().collect::<Vec<_>>().join(", ");
    ConfigurableFieldSpec {
        id: spec.id.clone(),
        annotation: format!("Enum[{}]", options_str),
        name: spec.name.clone(),
        description: spec
            .description
            .clone()
            .or_else(|| description.map(String::from)),
        default: Some(Value::String(spec.default.clone())),
        is_shared: spec.is_shared,
        dependencies: None,
    }
}

pub fn make_options_spec_multi(
    spec: &ConfigurableFieldMultiOption,
    description: Option<&str>,
) -> ConfigurableFieldSpec {
    let options_str = spec.options.keys().cloned().collect::<Vec<_>>().join(", ");
    ConfigurableFieldSpec {
        id: spec.id.clone(),
        annotation: format!("Sequence[Enum[{}]]", options_str),
        name: spec.name.clone(),
        description: spec
            .description
            .clone()
            .or_else(|| description.map(String::from)),
        default: Some(Value::Array(
            spec.default
                .iter()
                .map(|s| Value::String(s.clone()))
                .collect(),
        )),
        is_shared: spec.is_shared,
        dependencies: None,
    }
}

fn str_remove_prefix(s: &str, prefix: &str) -> String {
    if let Some(stripped) = s.strip_prefix(prefix) {
        stripped.to_string()
    } else {
        s.to_string()
    }
}

pub trait DynamicRunnable: Runnable {
    fn config_specs(&self) -> Vec<ConfigurableFieldSpec>;

    fn prepare(
        &self,
        config: Option<RunnableConfig>,
    ) -> (
        Arc<dyn Runnable<Input = Self::Input, Output = Self::Output> + Send + Sync>,
        RunnableConfig,
    );
}

pub struct RunnableConfigurableFields<I, O>
where
    I: Send + Sync + Clone + Debug + 'static,
    O: Send + Sync + Clone + Debug + 'static,
{
    pub default: Arc<dyn Runnable<Input = I, Output = O> + Send + Sync>,
    pub fields: HashMap<String, AnyConfigurableField>,
    pub config: Option<RunnableConfig>,
    #[allow(clippy::type_complexity)]
    reconfigure_fn: Option<
        Arc<
            dyn Fn(
                    &dyn Runnable<Input = I, Output = O>,
                    &HashMap<String, Value>,
                )
                    -> Option<Arc<dyn Runnable<Input = I, Output = O> + Send + Sync>>
                + Send
                + Sync,
        >,
    >,
}

impl<I, O> Debug for RunnableConfigurableFields<I, O>
where
    I: Send + Sync + Clone + Debug + 'static,
    O: Send + Sync + Clone + Debug + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RunnableConfigurableFields")
            .field("default", &self.default)
            .field("fields", &self.fields)
            .field("config", &self.config)
            .field(
                "reconfigure_fn",
                &self.reconfigure_fn.as_ref().map(|_| "..."),
            )
            .finish()
    }
}

impl<I, O> Clone for RunnableConfigurableFields<I, O>
where
    I: Send + Sync + Clone + Debug + 'static,
    O: Send + Sync + Clone + Debug + 'static,
{
    fn clone(&self) -> Self {
        Self {
            default: Arc::clone(&self.default),
            fields: self.fields.clone(),
            config: self.config.clone(),
            reconfigure_fn: self.reconfigure_fn.clone(),
        }
    }
}

impl<I, O> RunnableConfigurableFields<I, O>
where
    I: Send + Sync + Clone + Debug + 'static,
    O: Send + Sync + Clone + Debug + 'static,
{
    pub fn new(
        default: Arc<dyn Runnable<Input = I, Output = O> + Send + Sync>,
        fields: HashMap<String, AnyConfigurableField>,
    ) -> Self {
        Self {
            default,
            fields,
            config: None,
            reconfigure_fn: None,
        }
    }

    #[allow(clippy::type_complexity)]
    pub fn with_reconfigure_fn(
        default: Arc<dyn Runnable<Input = I, Output = O> + Send + Sync>,
        fields: HashMap<String, AnyConfigurableField>,
        reconfigure_fn: Arc<
            dyn Fn(
                    &dyn Runnable<Input = I, Output = O>,
                    &HashMap<String, Value>,
                )
                    -> Option<Arc<dyn Runnable<Input = I, Output = O> + Send + Sync>>
                + Send
                + Sync,
        >,
    ) -> Self {
        Self {
            default,
            fields,
            config: None,
            reconfigure_fn: Some(reconfigure_fn),
        }
    }

    pub fn with_config(mut self, config: RunnableConfig) -> Self {
        self.config = Some(config);
        self
    }

    fn prepare_internal(
        &self,
        config: Option<RunnableConfig>,
    ) -> (
        Arc<dyn Runnable<Input = I, Output = O> + Send + Sync>,
        RunnableConfig,
    ) {
        let merged = merge_configs(vec![self.config.clone(), config]);
        let config = ensure_config(Some(merged));

        let specs_by_id: HashMap<String, (&str, &AnyConfigurableField)> = self
            .fields
            .iter()
            .map(|(key, spec)| {
                let id = match spec {
                    AnyConfigurableField::Field(f) => f.id.clone(),
                    AnyConfigurableField::SingleOption(o) => o.id.clone(),
                    AnyConfigurableField::MultiOption(o) => o.id.clone(),
                };
                (id, (key.as_str(), spec))
            })
            .collect();

        let mut configurable_fields: HashMap<String, Value> = HashMap::new();

        for (key, value) in config.configurable.iter() {
            if let Some((field_name, spec)) = specs_by_id.get(key) {
                match spec {
                    AnyConfigurableField::Field(_) => {
                        configurable_fields.insert(field_name.to_string(), value.clone());
                    }
                    AnyConfigurableField::SingleOption(opt) => {
                        if let Some(selected_key) = value.as_str()
                            && let Some(option_value) = opt.options.get(selected_key)
                        {
                            configurable_fields
                                .insert(field_name.to_string(), option_value.clone());
                        }
                    }
                    AnyConfigurableField::MultiOption(opt) => {
                        if let Some(selected_keys) = value.as_array() {
                            let values: Vec<Value> = selected_keys
                                .iter()
                                .filter_map(|k| k.as_str())
                                .filter_map(|k| opt.options.get(k).cloned())
                                .collect();
                            configurable_fields
                                .insert(field_name.to_string(), Value::Array(values));
                        }
                    }
                }
            }
        }

        if configurable_fields.is_empty() {
            return (Arc::clone(&self.default), config);
        }

        if let Some(reconfigure_fn) = &self.reconfigure_fn
            && let Some(reconfigured) = reconfigure_fn(self.default.as_ref(), &configurable_fields)
        {
            return (reconfigured, config);
        }

        (Arc::clone(&self.default), config)
    }
}

#[async_trait]
impl<I, O> Runnable for RunnableConfigurableFields<I, O>
where
    I: Send + Sync + Clone + Debug + 'static,
    O: Send + Sync + Clone + Debug + 'static,
{
    type Input = I;
    type Output = O;

    fn name(&self) -> Option<String> {
        self.default.name()
    }

    fn config_specs(&self) -> Result<Vec<ConfigurableFieldSpec>> {
        let mut specs = Vec::new();

        for field in self.fields.values() {
            match field {
                AnyConfigurableField::Field(f) => {
                    specs.push(ConfigurableFieldSpec {
                        id: f.id.clone(),
                        annotation: f.annotation.clone().unwrap_or_else(|| "Any".to_string()),
                        name: f.name.clone(),
                        description: f.description.clone(),
                        default: None,
                        is_shared: f.is_shared,
                        dependencies: None,
                    });
                }
                AnyConfigurableField::SingleOption(opt) => {
                    specs.push(make_options_spec_single(opt, None));
                }
                AnyConfigurableField::MultiOption(opt) => {
                    specs.push(make_options_spec_multi(opt, None));
                }
            }
        }

        get_unique_config_specs(specs).map_err(Error::other)
    }

    fn invoke(&self, input: Self::Input, config: Option<RunnableConfig>) -> Result<Self::Output> {
        let (runnable, config) = self.prepare_internal(config);
        runnable.invoke(input, Some(config))
    }

    async fn ainvoke(
        &self,
        input: Self::Input,
        config: Option<RunnableConfig>,
    ) -> Result<Self::Output>
    where
        Self: 'static,
    {
        let (runnable, config) = self.prepare_internal(config);
        runnable.ainvoke(input, Some(config)).await
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

        let configs = match get_config_list(config, inputs.len()) {
            Ok(c) => c,
            Err(e) => return vec![Err(e)],
        };
        let prepared: Vec<_> = configs
            .iter()
            .map(|c| self.prepare_internal(Some(c.clone())))
            .collect();

        let all_default = prepared.iter().all(|(r, _)| Arc::ptr_eq(r, &self.default));

        if all_default {
            let prepared_configs: Vec<_> = prepared.into_iter().map(|(_, c)| c).collect();
            return self.default.batch(
                inputs,
                Some(ConfigOrList::List(prepared_configs)),
                return_exceptions,
            );
        }

        inputs
            .into_iter()
            .zip(prepared)
            .map(|(input, (runnable, config))| runnable.invoke(input, Some(config)))
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

        let configs = match get_config_list(config, inputs.len()) {
            Ok(c) => c,
            Err(e) => return vec![Err(e)],
        };
        let prepared: Vec<_> = configs
            .iter()
            .map(|c| self.prepare_internal(Some(c.clone())))
            .collect();

        let all_default = prepared.iter().all(|(r, _)| Arc::ptr_eq(r, &self.default));

        if all_default {
            let prepared_configs: Vec<_> = prepared.into_iter().map(|(_, c)| c).collect();
            return self
                .default
                .abatch(
                    inputs,
                    Some(ConfigOrList::List(prepared_configs)),
                    return_exceptions,
                )
                .await;
        }

        let max_concurrency = configs.first().and_then(|c| c.max_concurrency);

        let futures: Vec<_> = inputs
            .into_iter()
            .zip(prepared)
            .map(|(input, (runnable, config))| {
                Box::pin(async move { runnable.ainvoke(input, Some(config)).await })
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
        let (runnable, config) = self.prepare_internal(config);
        Box::pin(async_stream::stream! {
            let result = runnable.invoke(input, Some(config));
            yield result;
        })
    }

    fn transform<'a>(
        &'a self,
        input: BoxStream<'a, Self::Input>,
        config: Option<RunnableConfig>,
    ) -> BoxStream<'a, Result<Self::Output>> {
        let (runnable, config) = self.prepare_internal(config);
        Box::pin(async_stream::stream! {
            let mut stream = runnable.transform(input, Some(config));
            while let Some(item) = futures::StreamExt::next(&mut stream).await {
                yield item;
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
        let (runnable, config) = self.prepare_internal(config);
        Box::pin(async_stream::stream! {
            let mut stream = runnable.atransform(input, Some(config));
            while let Some(item) = futures::StreamExt::next(&mut stream).await {
                yield item;
            }
        })
    }

    fn get_graph(&self, config: Option<&RunnableConfig>) -> Result<super::graph::Graph> {
        self.default.get_graph(config)
    }
}

#[derive(Debug)]
pub struct RunnableConfigurableAlternatives<I, O>
where
    I: Send + Sync + Clone + Debug + 'static,
    O: Send + Sync + Clone + Debug + 'static,
{
    pub which: ConfigurableField,
    pub default: Arc<dyn Runnable<Input = I, Output = O> + Send + Sync>,
    pub alternatives: HashMap<String, Alternative<I, O>>,
    pub default_key: String,
    pub prefix_keys: bool,
    pub config: Option<RunnableConfig>,
}

pub enum Alternative<I, O>
where
    I: Send + Sync + Clone + Debug + 'static,
    O: Send + Sync + Clone + Debug + 'static,
{
    Runnable(Arc<dyn Runnable<Input = I, Output = O> + Send + Sync>),
    Factory(Arc<dyn Fn() -> Arc<dyn Runnable<Input = I, Output = O> + Send + Sync> + Send + Sync>),
}

impl<I, O> Debug for Alternative<I, O>
where
    I: Send + Sync + Clone + Debug + 'static,
    O: Send + Sync + Clone + Debug + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Alternative::Runnable(_) => write!(f, "Alternative::Runnable(...)"),
            Alternative::Factory(_) => write!(f, "Alternative::Factory(...)"),
        }
    }
}

impl<I, O> Clone for Alternative<I, O>
where
    I: Send + Sync + Clone + Debug + 'static,
    O: Send + Sync + Clone + Debug + 'static,
{
    fn clone(&self) -> Self {
        match self {
            Alternative::Runnable(r) => Alternative::Runnable(Arc::clone(r)),
            Alternative::Factory(f) => Alternative::Factory(Arc::clone(f)),
        }
    }
}

impl<I, O> Clone for RunnableConfigurableAlternatives<I, O>
where
    I: Send + Sync + Clone + Debug + 'static,
    O: Send + Sync + Clone + Debug + 'static,
{
    fn clone(&self) -> Self {
        Self {
            which: self.which.clone(),
            default: Arc::clone(&self.default),
            alternatives: self.alternatives.clone(),
            default_key: self.default_key.clone(),
            prefix_keys: self.prefix_keys,
            config: self.config.clone(),
        }
    }
}

impl<I, O> RunnableConfigurableAlternatives<I, O>
where
    I: Send + Sync + Clone + Debug + 'static,
    O: Send + Sync + Clone + Debug + 'static,
{
    pub fn new(
        which: ConfigurableField,
        default: Arc<dyn Runnable<Input = I, Output = O> + Send + Sync>,
        alternatives: HashMap<String, Alternative<I, O>>,
        default_key: impl Into<String>,
        prefix_keys: bool,
    ) -> Self {
        Self {
            which,
            default,
            alternatives,
            default_key: default_key.into(),
            prefix_keys,
            config: None,
        }
    }

    pub fn with_config(mut self, config: RunnableConfig) -> Self {
        self.config = Some(config);
        self
    }

    fn prepare_internal(
        &self,
        config: Option<RunnableConfig>,
    ) -> Result<(
        Arc<dyn Runnable<Input = I, Output = O> + Send + Sync>,
        RunnableConfig,
    )> {
        let merged = merge_configs(vec![self.config.clone(), config]);
        let config = ensure_config(Some(merged));

        let which = config
            .configurable
            .get(&self.which.id)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| self.default_key.clone());

        let config = if self.prefix_keys {
            let prefix = format!("{}=={}/", self.which.id, which);
            let new_configurable: HashMap<String, Value> = config
                .configurable
                .iter()
                .map(|(k, v)| (str_remove_prefix(k, &prefix), v.clone()))
                .collect();
            RunnableConfig {
                configurable: new_configurable,
                ..config
            }
        } else {
            config
        };

        if which == self.default_key {
            return Ok((Arc::clone(&self.default), config));
        }

        if let Some(alt) = self.alternatives.get(&which) {
            let runnable = match alt {
                Alternative::Runnable(r) => Arc::clone(r),
                Alternative::Factory(f) => f(),
            };
            return Ok((runnable, config));
        }

        Err(Error::other(format!("Unknown alternative: {}", which)))
    }
}

#[async_trait]
impl<I, O> Runnable for RunnableConfigurableAlternatives<I, O>
where
    I: Send + Sync + Clone + Debug + 'static,
    O: Send + Sync + Clone + Debug + 'static,
{
    type Input = I;
    type Output = O;

    fn name(&self) -> Option<String> {
        self.default.name()
    }

    fn config_specs(&self) -> Result<Vec<ConfigurableFieldSpec>> {
        let mut all_keys: Vec<String> = self.alternatives.keys().cloned().collect();
        all_keys.push(self.default_key.clone());

        let which_spec = ConfigurableFieldSpec {
            id: self.which.id.clone(),
            annotation: format!("Enum[{}]", all_keys.join(", ")),
            name: self.which.name.clone(),
            description: self.which.description.clone(),
            default: Some(Value::String(self.default_key.clone())),
            is_shared: self.which.is_shared,
            dependencies: None,
        };

        let specs = vec![which_spec];
        get_unique_config_specs(specs).map_err(Error::other)
    }

    fn invoke(&self, input: Self::Input, config: Option<RunnableConfig>) -> Result<Self::Output> {
        let (runnable, config) = self.prepare_internal(config)?;
        runnable.invoke(input, Some(config))
    }

    async fn ainvoke(
        &self,
        input: Self::Input,
        config: Option<RunnableConfig>,
    ) -> Result<Self::Output>
    where
        Self: 'static,
    {
        let (runnable, config) = self.prepare_internal(config)?;
        runnable.ainvoke(input, Some(config)).await
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

        let configs = match get_config_list(config, inputs.len()) {
            Ok(c) => c,
            Err(e) => return vec![Err(e)],
        };
        let prepared: Vec<_> = configs
            .iter()
            .map(|c| self.prepare_internal(Some(c.clone())))
            .collect();

        let all_default = prepared.iter().all(|r| {
            r.as_ref()
                .map(|(runnable, _)| Arc::ptr_eq(runnable, &self.default))
                .unwrap_or(false)
        });

        if all_default {
            let prepared_configs: Vec<_> = prepared
                .into_iter()
                .filter_map(|r| r.ok())
                .map(|(_, c)| c)
                .collect();
            return self.default.batch(
                inputs,
                Some(ConfigOrList::List(prepared_configs)),
                return_exceptions,
            );
        }

        inputs
            .into_iter()
            .zip(prepared)
            .map(|(input, prepared_result)| match prepared_result {
                Ok((runnable, config)) => runnable.invoke(input, Some(config)),
                Err(e) => Err(e),
            })
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

        let configs = match get_config_list(config, inputs.len()) {
            Ok(c) => c,
            Err(e) => return vec![Err(e)],
        };
        let prepared: Vec<_> = configs
            .iter()
            .map(|c| self.prepare_internal(Some(c.clone())))
            .collect();

        let all_default = prepared.iter().all(|r| {
            r.as_ref()
                .map(|(runnable, _)| Arc::ptr_eq(runnable, &self.default))
                .unwrap_or(false)
        });

        if all_default {
            let prepared_configs: Vec<_> = prepared
                .into_iter()
                .filter_map(|r| r.ok())
                .map(|(_, c)| c)
                .collect();
            return self
                .default
                .abatch(
                    inputs,
                    Some(ConfigOrList::List(prepared_configs)),
                    return_exceptions,
                )
                .await;
        }

        let mut results = Vec::with_capacity(inputs.len());
        for (input, prepared_result) in inputs.into_iter().zip(prepared) {
            let result = match prepared_result {
                Ok((runnable, config)) => runnable.ainvoke(input, Some(config)).await,
                Err(e) => Err(e),
            };
            results.push(result);
        }

        results
    }

    fn stream(
        &self,
        input: Self::Input,
        config: Option<RunnableConfig>,
    ) -> BoxStream<'_, Result<Self::Output>> {
        Box::pin(async_stream::stream! {
            match self.prepare_internal(config) {
                Ok((runnable, config)) => {
                    let result = runnable.invoke(input, Some(config));
                    yield result;
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
        Box::pin(async_stream::stream! {
            match self.prepare_internal(config) {
                Ok((runnable, config)) => {
                    let mut stream = runnable.transform(input, Some(config));
                    while let Some(item) = futures::StreamExt::next(&mut stream).await {
                        yield item;
                    }
                }
                Err(e) => {
                    yield Err(e);
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
        Box::pin(async_stream::stream! {
            match self.prepare_internal(config) {
                Ok((runnable, config)) => {
                    let mut stream = runnable.atransform(input, Some(config));
                    while let Some(item) = futures::StreamExt::next(&mut stream).await {
                        yield item;
                    }
                }
                Err(e) => {
                    yield Err(e);
                }
            }
        })
    }

    fn get_graph(
        &self,
        config: Option<&super::config::RunnableConfig>,
    ) -> crate::error::Result<super::graph::Graph> {
        let (runnable, config) = self.prepare_internal(config.cloned())?;
        runnable.get_graph(Some(&config))
    }
}

pub trait ConfigurableRunnable: Runnable + Sized {
    fn configurable_fields(
        self,
        fields: HashMap<String, AnyConfigurableField>,
    ) -> RunnableConfigurableFields<Self::Input, Self::Output>
    where
        Self: Send + Sync + 'static,
    {
        RunnableConfigurableFields::new(Arc::new(self), fields)
    }

    fn configurable_fields_reconfigurable(
        self,
        fields: HashMap<String, AnyConfigurableField>,
    ) -> RunnableConfigurableFields<Self::Input, Self::Output>
    where
        Self: Reconfigurable + Send + Sync + Clone + 'static,
    {
        #[allow(clippy::type_complexity)]
        let reconfigure_fn: Arc<
            dyn Fn(
                    &dyn Runnable<Input = Self::Input, Output = Self::Output>,
                    &HashMap<String, Value>,
                ) -> Option<
                    Arc<dyn Runnable<Input = Self::Input, Output = Self::Output> + Send + Sync>,
                > + Send
                + Sync,
        > = {
            let default_clone = self.clone();
            Arc::new(move |_runnable, fields| default_clone.reconfigure(fields))
        };
        RunnableConfigurableFields::with_reconfigure_fn(Arc::new(self), fields, reconfigure_fn)
    }

    fn configurable_alternatives(
        self,
        which: ConfigurableField,
        alternatives: HashMap<String, Alternative<Self::Input, Self::Output>>,
        default_key: impl Into<String>,
        prefix_keys: bool,
    ) -> RunnableConfigurableAlternatives<Self::Input, Self::Output>
    where
        Self: Send + Sync + 'static,
    {
        RunnableConfigurableAlternatives::new(
            which,
            Arc::new(self),
            alternatives,
            default_key,
            prefix_keys,
        )
    }
}

impl<R> ConfigurableRunnable for R where R: Runnable + Sized {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runnables::base::RunnableLambda;

    #[test]
    fn test_prefix_config_spec() {
        let spec = ConfigurableFieldSpec {
            id: "temperature".to_string(),
            annotation: "float".to_string(),
            name: Some("Temperature".to_string()),
            description: None,
            default: Some(Value::Number(serde_json::Number::from_f64(0.7).unwrap())),
            is_shared: false,
            dependencies: None,
        };

        let prefixed = prefix_config_spec(&spec, "model==gpt4");
        assert_eq!(prefixed.id, "model==gpt4/temperature");

        let shared_spec = ConfigurableFieldSpec {
            is_shared: true,
            ..spec.clone()
        };
        let prefixed_shared = prefix_config_spec(&shared_spec, "model==gpt4");
        assert_eq!(prefixed_shared.id, "temperature");
    }

    #[test]
    fn test_str_remove_prefix() {
        assert_eq!(
            str_remove_prefix("model==gpt4/temperature", "model==gpt4/"),
            "temperature"
        );
        assert_eq!(
            str_remove_prefix("temperature", "model==gpt4/"),
            "temperature"
        );
    }

    #[test]
    fn test_make_options_spec_single() {
        let mut options = HashMap::new();
        options.insert(
            "low".to_string(),
            Value::Number(serde_json::Number::from_f64(0.3).unwrap()),
        );
        options.insert(
            "high".to_string(),
            Value::Number(serde_json::Number::from_f64(0.9).unwrap()),
        );

        let spec = ConfigurableFieldSingleOption {
            id: "temp_preset".to_string(),
            options,
            default: "low".to_string(),
            name: Some("Temperature Preset".to_string()),
            description: None,
            is_shared: false,
        };

        let config_spec = make_options_spec_single(&spec, Some("Choose temperature"));
        assert_eq!(config_spec.id, "temp_preset");
        assert!(config_spec.annotation.contains("Enum"));
        assert_eq!(config_spec.default, Some(Value::String("low".to_string())));
    }

    #[test]
    fn test_configurable_fields_invoke() {
        let runnable = RunnableLambda::new(|x: i32| Ok(x * 2));
        let fields = HashMap::new();
        let configurable = runnable.configurable_fields(fields);

        let result = configurable.invoke(5, None).unwrap();
        assert_eq!(result, 10);
    }

    #[test]
    fn test_configurable_alternatives_invoke() {
        let default = RunnableLambda::new(|x: i32| Ok(x * 2));
        let alt = RunnableLambda::new(|x: i32| Ok(x * 3));

        let mut alternatives = HashMap::new();
        alternatives.insert("triple".to_string(), Alternative::Runnable(Arc::new(alt)));

        let configurable = default.configurable_alternatives(
            ConfigurableField::new("multiplier"),
            alternatives,
            "double",
            false,
        );

        let result = configurable.invoke(5, None).unwrap();
        assert_eq!(result, 10);

        let mut config = RunnableConfig::default();
        config.configurable.insert(
            "multiplier".to_string(),
            Value::String("triple".to_string()),
        );
        let result = configurable.invoke(5, Some(config)).unwrap();
        assert_eq!(result, 15);
    }

    #[test]
    fn test_configurable_alternatives_unknown() {
        let default = RunnableLambda::new(|x: i32| Ok(x * 2));
        let alternatives = HashMap::new();

        let configurable = default.configurable_alternatives(
            ConfigurableField::new("multiplier"),
            alternatives,
            "double",
            false,
        );

        let mut config = RunnableConfig::default();
        config.configurable.insert(
            "multiplier".to_string(),
            Value::String("unknown".to_string()),
        );

        let result = configurable.invoke(5, Some(config));
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_configurable_fields_ainvoke() {
        let runnable = RunnableLambda::new(|x: i32| Ok(x * 2));
        let fields = HashMap::new();
        let configurable = runnable.configurable_fields(fields);

        let result = configurable.ainvoke(5, None).await.unwrap();
        assert_eq!(result, 10);
    }

    #[tokio::test]
    async fn test_configurable_alternatives_ainvoke() {
        let default = RunnableLambda::new(|x: i32| Ok(x * 2));
        let alt = RunnableLambda::new(|x: i32| Ok(x + 100));

        let mut alternatives = HashMap::new();
        alternatives.insert(
            "add_hundred".to_string(),
            Alternative::Runnable(Arc::new(alt)),
        );

        let configurable = default.configurable_alternatives(
            ConfigurableField::new("operation"),
            alternatives,
            "double",
            false,
        );

        let mut config = RunnableConfig::default();
        config.configurable.insert(
            "operation".to_string(),
            Value::String("add_hundred".to_string()),
        );

        let result = configurable.ainvoke(5, Some(config)).await.unwrap();
        assert_eq!(result, 105);
    }

    #[test]
    fn test_configurable_with_factory() {
        let default = RunnableLambda::new(|x: i32| Ok(x * 2));

        let mut alternatives = HashMap::new();
        alternatives.insert(
            "triple".to_string(),
            Alternative::Factory(Arc::new(|| {
                Arc::new(RunnableLambda::new(|x: i32| Ok(x * 3)))
                    as Arc<dyn Runnable<Input = i32, Output = i32> + Send + Sync>
            })),
        );

        let configurable = default.configurable_alternatives(
            ConfigurableField::new("multiplier"),
            alternatives,
            "double",
            false,
        );

        let mut config = RunnableConfig::default();
        config.configurable.insert(
            "multiplier".to_string(),
            Value::String("triple".to_string()),
        );

        let result = configurable.invoke(5, Some(config)).unwrap();
        assert_eq!(result, 15);
    }
}
