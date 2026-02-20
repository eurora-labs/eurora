use std::collections::HashMap;
use std::sync::Arc;

use agent_chain_core::error::Result;
use agent_chain_core::runnables::base::Runnable;
use agent_chain_core::runnables::config::RunnableConfig;
use agent_chain_core::runnables::configurable::{
    Alternative, ConfigurableRunnable, Reconfigurable, RunnableConfigurableAlternatives,
    RunnableConfigurableFields, make_options_spec_multi, make_options_spec_single,
    prefix_config_spec,
};
use agent_chain_core::runnables::utils::{
    AnyConfigurableField, ConfigurableField, ConfigurableFieldMultiOption,
    ConfigurableFieldSingleOption, ConfigurableFieldSpec,
};
use async_trait::async_trait;
use serde_json::Value;

#[derive(Debug, Clone)]
struct MyRunnable {
    my_property: String,
}

impl MyRunnable {
    fn new(my_property: impl Into<String>) -> Self {
        Self {
            my_property: my_property.into(),
        }
    }
}

#[async_trait]
impl Runnable for MyRunnable {
    type Input = String;
    type Output = String;

    fn invoke(&self, input: Self::Input, _config: Option<RunnableConfig>) -> Result<Self::Output> {
        Ok(format!("{}{}", input, self.my_property))
    }
}

impl Reconfigurable for MyRunnable {
    fn reconfigure(
        &self,
        fields: &HashMap<String, Value>,
    ) -> Option<Arc<dyn Runnable<Input = String, Output = String> + Send + Sync>> {
        let mut new = self.clone();
        if let Some(val) = fields.get("my_property")
            && let Some(s) = val.as_str()
        {
            new.my_property = s.to_string();
        }
        Some(Arc::new(new))
    }
}

#[derive(Debug, Clone)]
struct MyOtherRunnable {
    my_other_property: String,
}

impl MyOtherRunnable {
    fn new(my_other_property: impl Into<String>) -> Self {
        Self {
            my_other_property: my_other_property.into(),
        }
    }
}

#[async_trait]
impl Runnable for MyOtherRunnable {
    type Input = String;
    type Output = String;

    fn invoke(&self, input: Self::Input, _config: Option<RunnableConfig>) -> Result<Self::Output> {
        Ok(format!("{}{}", input, self.my_other_property))
    }
}

fn make_my_property_field(id: &str) -> HashMap<String, AnyConfigurableField> {
    let mut fields = HashMap::new();
    fields.insert(
        "my_property".to_string(),
        AnyConfigurableField::Field(
            ConfigurableField::new(id)
                .with_name("My property")
                .with_description("The property to test"),
        ),
    );
    fields
}

fn make_configurable_my_runnable(
    property: &str,
    field_id: &str,
) -> RunnableConfigurableFields<String, String> {
    let runnable = MyRunnable::new(property);
    let fields = make_my_property_field(field_id);
    runnable.configurable_fields_reconfigurable(fields)
}

#[test]
fn test_prefix_config_spec_non_shared() {
    let spec = ConfigurableFieldSpec {
        id: "temperature".to_string(),
        annotation: "float".to_string(),
        name: Some("Temperature".to_string()),
        description: Some("LLM temp".to_string()),
        default: Some(Value::Number(serde_json::Number::from_f64(0.7).unwrap())),
        is_shared: false,
        dependencies: None,
    };

    let result = prefix_config_spec(&spec, "model==gpt4");
    assert_eq!(result.id, "model==gpt4/temperature");
    assert_eq!(result.name, Some("Temperature".to_string()));
    assert_eq!(result.description, Some("LLM temp".to_string()));
    assert_eq!(
        result.default,
        Some(Value::Number(serde_json::Number::from_f64(0.7).unwrap()))
    );
    assert!(!result.is_shared);
}

#[test]
fn test_prefix_config_spec_shared_unchanged() {
    let spec = ConfigurableFieldSpec {
        id: "global_setting".to_string(),
        annotation: "String".to_string(),
        name: None,
        description: None,
        default: None,
        is_shared: true,
        dependencies: None,
    };

    let result = prefix_config_spec(&spec, "model==gpt4");
    assert_eq!(result.id, "global_setting");
}

#[test]
fn test_make_options_spec_single_option() {
    let mut options = HashMap::new();
    options.insert("gpt4".to_string(), Value::String("gpt-4".to_string()));
    options.insert("gpt3".to_string(), Value::String("gpt-3.5".to_string()));

    let spec = ConfigurableFieldSingleOption {
        id: "model".to_string(),
        options,
        default: "gpt4".to_string(),
        name: Some("Model".to_string()),
        description: Some("Which model".to_string()),
        is_shared: false,
    };

    let result = make_options_spec_single(&spec, Some("fallback desc"));
    assert_eq!(result.id, "model");
    assert_eq!(result.default, Some(Value::String("gpt4".to_string())));
    assert_eq!(result.description, Some("Which model".to_string()));
    assert!(result.annotation.contains("Enum"));
}

#[test]
fn test_make_options_spec_single_option_uses_fallback_description() {
    let mut options = HashMap::new();
    options.insert("gpt4".to_string(), Value::String("gpt-4".to_string()));

    let spec = ConfigurableFieldSingleOption {
        id: "model".to_string(),
        options,
        default: "gpt4".to_string(),
        name: None,
        description: None,
        is_shared: false,
    };

    let result = make_options_spec_single(&spec, Some("fallback desc"));
    assert_eq!(result.description, Some("fallback desc".to_string()));
}

#[test]
fn test_make_options_spec_multi_option() {
    let mut options = HashMap::new();
    options.insert(
        "search".to_string(),
        Value::String("web_search".to_string()),
    );
    options.insert("calc".to_string(), Value::String("calculator".to_string()));

    let spec = ConfigurableFieldMultiOption {
        id: "tools".to_string(),
        options,
        default: vec!["search".to_string()],
        name: Some("Tools".to_string()),
        description: None,
        is_shared: false,
    };

    let result = make_options_spec_multi(&spec, Some("fallback desc"));
    assert_eq!(result.id, "tools");
    assert_eq!(
        result.default,
        Some(Value::Array(vec![Value::String("search".to_string())]))
    );
}

#[test]
fn test_configurable_alternatives_invoke_default() {
    let default = MyRunnable::new("default_val");
    let alt = MyOtherRunnable::new("alt_val");

    let mut alternatives = HashMap::new();
    alternatives.insert("other".to_string(), Alternative::Runnable(Arc::new(alt)));

    let configurable = RunnableConfigurableAlternatives::new(
        ConfigurableField::new("which"),
        Arc::new(default),
        alternatives,
        "default",
        false,
    );

    let result = configurable.invoke("input_".to_string(), None).unwrap();
    assert_eq!(result, "input_default_val");
}

#[test]
fn test_configurable_alternatives_invoke_alternative() {
    let default = MyRunnable::new("default_val");
    let alt = MyOtherRunnable::new("alt_val");

    let mut alternatives = HashMap::new();
    alternatives.insert("other".to_string(), Alternative::Runnable(Arc::new(alt)));

    let configurable = RunnableConfigurableAlternatives::new(
        ConfigurableField::new("which"),
        Arc::new(default),
        alternatives,
        "default",
        false,
    );

    let config = RunnableConfig::default().with_configurable({
        let mut c = HashMap::new();
        c.insert("which".to_string(), Value::String("other".to_string()));
        c
    });

    let result = configurable
        .invoke("input_".to_string(), Some(config))
        .unwrap();
    assert_eq!(result, "input_alt_val");
}

#[test]
fn test_configurable_alternatives_unknown_raises() {
    let default = MyRunnable::new("default_val");

    let configurable = RunnableConfigurableAlternatives::new(
        ConfigurableField::new("which"),
        Arc::new(default),
        HashMap::new(),
        "default",
        false,
    );

    let config = RunnableConfig::default().with_configurable({
        let mut c = HashMap::new();
        c.insert(
            "which".to_string(),
            Value::String("nonexistent".to_string()),
        );
        c
    });

    let result = configurable.invoke("input_".to_string(), Some(config));
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("Unknown alternative"),
        "Expected 'Unknown alternative' in error, got: {}",
        err_msg
    );
}

#[test]
fn test_configurable_alternatives_with_callable_factory() {
    let default = MyRunnable::new("default_val");

    let mut alternatives: HashMap<String, Alternative<String, String>> = HashMap::new();
    alternatives.insert(
        "other".to_string(),
        Alternative::Factory(Arc::new(|| {
            Arc::new(MyOtherRunnable::new("factory_val"))
                as Arc<dyn Runnable<Input = String, Output = String> + Send + Sync>
        })),
    );

    let configurable = RunnableConfigurableAlternatives::new(
        ConfigurableField::new("which"),
        Arc::new(default),
        alternatives,
        "default",
        false,
    );

    let config = RunnableConfig::default().with_configurable({
        let mut c = HashMap::new();
        c.insert("which".to_string(), Value::String("other".to_string()));
        c
    });

    let result = configurable
        .invoke("input_".to_string(), Some(config))
        .unwrap();
    assert_eq!(result, "input_factory_val");
}

#[test]
fn test_configurable_alternatives_config_specs() {
    let default = MyRunnable::new("a");
    let alt = MyOtherRunnable::new("b");

    let mut alternatives = HashMap::new();
    alternatives.insert("other".to_string(), Alternative::Runnable(Arc::new(alt)));

    let configurable = RunnableConfigurableAlternatives::new(
        ConfigurableField::new("which"),
        Arc::new(default),
        alternatives,
        "default",
        false,
    );

    let specs = configurable.config_specs().unwrap();
    let spec_ids: Vec<&str> = specs.iter().map(|s| s.id.as_str()).collect();
    assert!(spec_ids.contains(&"which"));
}

#[test]
fn test_configurable_alternatives_with_prefix_keys() {
    let configurable_default = make_configurable_my_runnable("a", "my_property");

    let alt = MyOtherRunnable::new("b");

    let mut alternatives = HashMap::new();
    alternatives.insert("other".to_string(), Alternative::Runnable(Arc::new(alt)));

    let configurable = RunnableConfigurableAlternatives::new(
        ConfigurableField::new("which"),
        Arc::new(configurable_default),
        alternatives,
        "default",
        true,
    );

    let specs = configurable.config_specs().unwrap();
    let spec_ids: Vec<&str> = specs.iter().map(|s| s.id.as_str()).collect();
    assert!(spec_ids.contains(&"which"));
}

#[test]
fn test_dynamic_runnable_with_config() {
    let configurable = make_configurable_my_runnable("a", "my_property");
    let new =
        configurable.with_config(RunnableConfig::default().with_tags(vec!["test_tag".to_string()]));
    assert!(new.config.is_some());
    assert!(
        new.config
            .as_ref()
            .unwrap()
            .tags
            .contains(&"test_tag".to_string())
    );
}

#[test]
fn test_configurable_fields_config_specs() {
    let configurable = make_configurable_my_runnable("a", "my_property");
    let specs = configurable.config_specs().unwrap();
    let spec_ids: Vec<&str> = specs.iter().map(|s| s.id.as_str()).collect();
    assert!(spec_ids.contains(&"my_property"));

    let spec = specs.iter().find(|s| s.id == "my_property").unwrap();
    assert_eq!(spec.name, Some("My property".to_string()));
    assert_eq!(spec.description, Some("The property to test".to_string()));
}

#[test]
fn test_configurable_fields_prepare_no_config() {
    let configurable = make_configurable_my_runnable("a", "my_property");
    let result = configurable.invoke("x".to_string(), None).unwrap();
    assert_eq!(result, "xa");
}

#[test]
fn test_configurable_fields_prepare_with_override() {
    let configurable = make_configurable_my_runnable("a", "my_property");
    let config = RunnableConfig::default().with_configurable({
        let mut c = HashMap::new();
        c.insert("my_property".to_string(), Value::String("b".to_string()));
        c
    });
    let result = configurable.invoke("x".to_string(), Some(config)).unwrap();
    assert_eq!(result, "xb");
}

#[test]
fn test_doubly_set_configurable() {
    let configurable = make_configurable_my_runnable("a", "my_property");

    let config = RunnableConfig::default().with_configurable({
        let mut c = HashMap::new();
        c.insert("my_property".to_string(), Value::String("c".to_string()));
        c
    });

    let result = configurable.invoke("d".to_string(), Some(config)).unwrap();
    assert_eq!(result, "dc");
}

#[test]
fn test_configurable_fields_batch() {
    let configurable = make_configurable_my_runnable("a", "my_property");

    let configs = vec![
        RunnableConfig::default().with_configurable({
            let mut c = HashMap::new();
            c.insert("my_property".to_string(), Value::String("1".to_string()));
            c
        }),
        RunnableConfig::default().with_configurable({
            let mut c = HashMap::new();
            c.insert("my_property".to_string(), Value::String("2".to_string()));
            c
        }),
    ];

    let results = configurable.batch(
        vec!["x".to_string(), "y".to_string()],
        Some(agent_chain_core::runnables::config::ConfigOrList::List(
            configs,
        )),
        false,
    );

    assert_eq!(results.len(), 2);
    assert_eq!(results[0].as_ref().unwrap(), "x1");
    assert_eq!(results[1].as_ref().unwrap(), "y2");
}

#[test]
fn test_configurable_fields_stream() {
    let configurable = make_configurable_my_runnable("a", "my_property");

    let config = RunnableConfig::default().with_configurable({
        let mut c = HashMap::new();
        c.insert("my_property".to_string(), Value::String("b".to_string()));
        c
    });

    let rt = tokio::runtime::Runtime::new().unwrap();
    let chunks: Vec<Result<String>> = rt.block_on(async {
        use futures::StreamExt;
        let stream = configurable.stream("x".to_string(), Some(config));
        stream.collect().await
    });

    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0].as_ref().unwrap(), "xb");
}

#[test]
fn test_configurable_fields_chained_configurable_fields() {
    let configurable = make_configurable_my_runnable("a", "my_property");
    let chained = configurable.configurable_fields(HashMap::new());

    let config = RunnableConfig::default().with_configurable({
        let mut c = HashMap::new();
        c.insert("my_property".to_string(), Value::String("c".to_string()));
        c
    });

    let result = chained.invoke("x".to_string(), Some(config)).unwrap();
    assert_eq!(result, "xc");
}

#[tokio::test]
async fn test_configurable_fields_ainvoke_with_override() {
    let configurable = make_configurable_my_runnable("a", "my_property");
    let config = RunnableConfig::default().with_configurable({
        let mut c = HashMap::new();
        c.insert("my_property".to_string(), Value::String("b".to_string()));
        c
    });
    let result = configurable
        .ainvoke("x".to_string(), Some(config))
        .await
        .unwrap();
    assert_eq!(result, "xb");
}

#[tokio::test]
async fn test_configurable_alternatives_ainvoke_default() {
    let default = MyRunnable::new("default_val");
    let alt = MyOtherRunnable::new("alt_val");

    let mut alternatives = HashMap::new();
    alternatives.insert("other".to_string(), Alternative::Runnable(Arc::new(alt)));

    let configurable = RunnableConfigurableAlternatives::new(
        ConfigurableField::new("which"),
        Arc::new(default),
        alternatives,
        "default",
        false,
    );

    let result = configurable
        .ainvoke("input_".to_string(), None)
        .await
        .unwrap();
    assert_eq!(result, "input_default_val");
}

#[tokio::test]
async fn test_configurable_alternatives_ainvoke_alternative() {
    let default = MyRunnable::new("default_val");
    let alt = MyOtherRunnable::new("alt_val");

    let mut alternatives = HashMap::new();
    alternatives.insert("other".to_string(), Alternative::Runnable(Arc::new(alt)));

    let configurable = RunnableConfigurableAlternatives::new(
        ConfigurableField::new("which"),
        Arc::new(default),
        alternatives,
        "default",
        false,
    );

    let config = RunnableConfig::default().with_configurable({
        let mut c = HashMap::new();
        c.insert("which".to_string(), Value::String("other".to_string()));
        c
    });

    let result = configurable
        .ainvoke("input_".to_string(), Some(config))
        .await
        .unwrap();
    assert_eq!(result, "input_alt_val");
}
