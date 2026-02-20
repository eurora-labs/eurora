use std::collections::HashMap;
use std::sync::Arc;

use agent_chain_core::callbacks::{
    BaseCallbackHandler, CallbackManager, Callbacks, StdOutCallbackHandler,
    StreamingStdOutCallbackHandler,
};
use agent_chain_core::runnables::config::{
    AsyncVariableArgsFn, ConfigOrList, RunnableConfig, VariableArgsFn,
    acall_func_with_variable_args, call_func_with_variable_args, ensure_config,
    get_async_callback_manager_for_config, get_callback_manager_for_config, get_config_list,
    merge_configs, patch_config,
};

#[test]
fn test_ensure_config_none_returns_defaults() {
    let config = ensure_config(None);
    assert!(config.tags.is_empty());
    assert!(config.metadata.is_empty());
    assert!(config.callbacks.is_none());
    assert_eq!(config.recursion_limit, 25);
    assert!(config.configurable.is_empty());
}

#[test]
fn test_ensure_config_preserves_custom_values() {
    let custom = RunnableConfig::new()
        .with_recursion_limit(10)
        .with_tags(vec!["tag1".into()])
        .with_run_name("my_run");

    let config = ensure_config(Some(custom));
    assert_eq!(config.recursion_limit, 10);
    assert_eq!(config.tags, vec!["tag1"]);
    assert_eq!(config.run_name, Some("my_run".to_string()));
}

#[test]
fn test_ensure_config_default_returns_defaults() {
    let config = ensure_config(Some(RunnableConfig::default()));
    assert!(config.tags.is_empty());
    assert!(config.metadata.is_empty());
    assert!(config.callbacks.is_none());
    assert_eq!(config.recursion_limit, 25);
    assert!(config.configurable.is_empty());
}

#[test]
fn test_ensure_config_copies_tags_metadata_configurable() {
    let original_tags = vec!["a".to_string(), "b".to_string()];
    let original_metadata = HashMap::from([("k".to_string(), serde_json::json!("v"))]);
    let original_configurable = HashMap::from([("x".to_string(), serde_json::json!("y"))]);

    let config = RunnableConfig {
        tags: original_tags.clone(),
        metadata: original_metadata.clone(),
        configurable: original_configurable.clone(),
        ..Default::default()
    };

    let ensured = ensure_config(Some(config));

    assert_eq!(ensured.tags, original_tags);
    assert_eq!(ensured.configurable, original_configurable);
    assert_eq!(ensured.metadata["k"], serde_json::json!("v"));
    assert_eq!(ensured.metadata["x"], serde_json::json!("y"));
}

#[test]
fn test_get_config_list_single_config_replicated() {
    let config = RunnableConfig::new().with_tags(vec!["a".into()]);
    let configs = get_config_list(Some(ConfigOrList::Single(Box::new(config))), 3);
    assert_eq!(configs.len(), 3);
    for c in &configs {
        assert_eq!(c.tags, vec!["a"]);
        assert_eq!(c.recursion_limit, 25);
    }
}

#[test]
fn test_get_config_list_none_config() {
    let configs = get_config_list(None, 2);
    assert_eq!(configs.len(), 2);
    for c in &configs {
        assert!(c.tags.is_empty());
    }
}

#[test]
fn test_get_config_list_sequence_of_configs() {
    let config_a = RunnableConfig::new().with_tags(vec!["a".into()]);
    let config_b = RunnableConfig::new().with_tags(vec!["b".into()]);

    let configs = get_config_list(Some(ConfigOrList::List(vec![config_a, config_b])), 2);
    assert_eq!(configs.len(), 2);
    assert_eq!(configs[0].tags, vec!["a"]);
    assert_eq!(configs[1].tags, vec!["b"]);
}

#[test]
#[should_panic(expected = "same length")]
fn test_get_config_list_sequence_length_mismatch_raises() {
    let config_a = RunnableConfig::new().with_tags(vec!["a".into()]);
    get_config_list(Some(ConfigOrList::List(vec![config_a])), 3);
}

#[test]
fn test_get_config_list_zero_length() {
    let configs = get_config_list(None, 0);
    assert!(configs.is_empty());
}

#[test]
fn test_get_config_list_run_id_warning() {
    let run_id = uuid::Uuid::new_v4();
    let config = RunnableConfig::new().with_run_id(run_id);
    let configs = get_config_list(Some(ConfigOrList::Single(Box::new(config))), 3);

    assert_eq!(configs[0].run_id, Some(run_id));
    assert!(configs[1].run_id.is_none());
    assert!(configs[2].run_id.is_none());
}

#[test]
fn test_get_config_list_run_id_single_no_issue() {
    let run_id = uuid::Uuid::new_v4();
    let config = RunnableConfig::new().with_run_id(run_id);
    let configs = get_config_list(Some(ConfigOrList::Single(Box::new(config))), 1);
    assert_eq!(configs.len(), 1);
    assert_eq!(configs[0].run_id, Some(run_id));
}

#[test]
fn test_patch_config_none_input() {
    let config = patch_config(None, None, None, None, None, None);
    assert!(config.tags.is_empty());
    assert_eq!(config.recursion_limit, 25);
}

#[test]
fn test_patch_config_sets_recursion_limit() {
    let config = RunnableConfig::new().with_recursion_limit(10);
    let patched = patch_config(Some(config), None, None, None, Some(50), None);
    assert_eq!(patched.recursion_limit, 50);
}

#[test]
fn test_patch_config_sets_max_concurrency() {
    let patched = patch_config(None, None, None, Some(5), None, None);
    assert_eq!(patched.max_concurrency, Some(5));
}

#[test]
fn test_patch_config_sets_run_name() {
    let patched = patch_config(None, None, Some("my_run".to_string()), None, None, None);
    assert_eq!(patched.run_name, Some("my_run".to_string()));
}

#[test]
fn test_patch_config_configurable_merges() {
    let config = RunnableConfig {
        configurable: HashMap::from([("a".to_string(), serde_json::json!(1))]),
        ..Default::default()
    };
    let patched = patch_config(
        Some(config),
        None,
        None,
        None,
        None,
        Some(HashMap::from([("b".to_string(), serde_json::json!(2))])),
    );
    assert_eq!(patched.configurable["a"], serde_json::json!(1));
    assert_eq!(patched.configurable["b"], serde_json::json!(2));
}

#[test]
fn test_patch_config_callbacks_clears_run_name_and_run_id() {
    let run_id = uuid::Uuid::new_v4();
    let config = RunnableConfig::new()
        .with_run_name("old_name")
        .with_run_id(run_id);

    let callback_mgr = CallbackManager::new();
    let patched = patch_config(Some(config), Some(callback_mgr), None, None, None, None);
    assert!(patched.run_name.is_none());
    assert!(patched.run_id.is_none());
    assert!(patched.callbacks.is_some());
}

#[test]
fn test_merge_configs_tags_are_deduplicated_and_sorted() {
    let c1 = RunnableConfig::new().with_tags(vec!["b".into(), "a".into()]);
    let c2 = RunnableConfig::new().with_tags(vec!["a".into(), "c".into()]);
    let merged = merge_configs(vec![Some(c1), Some(c2)]);
    assert_eq!(merged.tags, vec!["a", "b", "c"]);
}

#[test]
fn test_merge_configs_metadata_is_merged() {
    let c1 = RunnableConfig {
        metadata: HashMap::from([("a".to_string(), serde_json::json!(1))]),
        ..Default::default()
    };
    let c2 = RunnableConfig {
        metadata: HashMap::from([("b".to_string(), serde_json::json!(2))]),
        ..Default::default()
    };
    let merged = merge_configs(vec![Some(c1), Some(c2)]);
    assert_eq!(merged.metadata["a"], serde_json::json!(1));
    assert_eq!(merged.metadata["b"], serde_json::json!(2));
}

#[test]
fn test_merge_configs_metadata_later_overrides() {
    let c1 = RunnableConfig {
        metadata: HashMap::from([("a".to_string(), serde_json::json!(1))]),
        ..Default::default()
    };
    let c2 = RunnableConfig {
        metadata: HashMap::from([("a".to_string(), serde_json::json!(2))]),
        ..Default::default()
    };
    let merged = merge_configs(vec![Some(c1), Some(c2)]);
    assert_eq!(merged.metadata["a"], serde_json::json!(2));
}

#[test]
fn test_merge_configs_configurable_is_merged() {
    let c1 = RunnableConfig {
        configurable: HashMap::from([("a".to_string(), serde_json::json!(1))]),
        ..Default::default()
    };
    let c2 = RunnableConfig {
        configurable: HashMap::from([("b".to_string(), serde_json::json!(2))]),
        ..Default::default()
    };
    let merged = merge_configs(vec![Some(c1), Some(c2)]);
    assert_eq!(merged.configurable["a"], serde_json::json!(1));
    assert_eq!(merged.configurable["b"], serde_json::json!(2));
}

#[test]
fn test_merge_configs_recursion_limit_non_default_wins() {
    let c1 = RunnableConfig::new().with_recursion_limit(10);
    let c2 = RunnableConfig::new().with_recursion_limit(50);
    let merged = merge_configs(vec![Some(c1), Some(c2)]);
    assert_eq!(merged.recursion_limit, 50);
}

#[test]
fn test_merge_configs_recursion_limit_default_does_not_override() {
    let c1 = RunnableConfig::new().with_recursion_limit(50);
    let c2 = RunnableConfig::new().with_recursion_limit(25);
    let merged = merge_configs(vec![Some(c1), Some(c2)]);
    assert_eq!(merged.recursion_limit, 50);
}

#[test]
fn test_merge_configs_none_configs_skipped() {
    let c1 = RunnableConfig::new().with_tags(vec!["a".into()]);
    let c2 = RunnableConfig::new().with_tags(vec!["b".into()]);
    let merged = merge_configs(vec![None, Some(c1), None, Some(c2)]);
    assert!(merged.tags.contains(&"a".to_string()));
    assert!(merged.tags.contains(&"b".to_string()));
}

#[test]
fn test_merge_configs_run_name_and_run_id() {
    let run_id = uuid::Uuid::new_v4();
    let c1 = RunnableConfig::new().with_run_name("first");
    let c2 = RunnableConfig::new()
        .with_run_name("second")
        .with_run_id(run_id);
    let merged = merge_configs(vec![Some(c1), Some(c2)]);
    assert_eq!(merged.run_name, Some("second".to_string()));
    assert_eq!(merged.run_id, Some(run_id));
}

#[test]
fn test_merge_configs_empty() {
    let merged = merge_configs(vec![]);
    assert!(merged.tags.is_empty());
    assert!(merged.metadata.is_empty());
    assert!(merged.configurable.is_empty());
    assert_eq!(merged.recursion_limit, 25);
}

#[test]
fn test_merge_config_callbacks_handler_lists() {
    let handler1: Arc<dyn BaseCallbackHandler> = Arc::new(StdOutCallbackHandler::new());
    let handler2: Arc<dyn BaseCallbackHandler> = Arc::new(StreamingStdOutCallbackHandler::new());

    let c1 = RunnableConfig {
        callbacks: Some(Callbacks::Handlers(vec![handler1])),
        ..Default::default()
    };
    let c2 = RunnableConfig {
        callbacks: Some(Callbacks::Handlers(vec![handler2])),
        ..Default::default()
    };

    let merged = merge_configs(vec![Some(c1), Some(c2)]);
    match &merged.callbacks {
        Some(Callbacks::Handlers(handlers)) => {
            assert_eq!(handlers.len(), 2);
            assert_eq!(handlers[0].name(), "StdOutCallbackHandler");
            assert_eq!(handlers[1].name(), "StreamingStdOutCallbackHandler");
        }
        _ => panic!("Expected Callbacks::Handlers"),
    }
}

#[test]
fn test_merge_config_callbacks_manager_with_handlers() {
    let handler1: Arc<dyn BaseCallbackHandler> = Arc::new(StdOutCallbackHandler::new());
    let mut mgr = CallbackManager::new();
    mgr.add_handler(handler1, true);

    let handler2: Arc<dyn BaseCallbackHandler> = Arc::new(StreamingStdOutCallbackHandler::new());

    let c1 = RunnableConfig {
        callbacks: Some(Callbacks::Manager(
            agent_chain_core::callbacks::BaseCallbackManager::with_handlers(
                mgr.handlers.clone(),
                Some(mgr.inheritable_handlers.clone()),
                mgr.parent_run_id,
                Some(mgr.tags.clone()),
                Some(mgr.inheritable_tags.clone()),
                Some(mgr.metadata.clone()),
                Some(mgr.inheritable_metadata.clone()),
            ),
        )),
        ..Default::default()
    };
    let c2 = RunnableConfig {
        callbacks: Some(Callbacks::Handlers(vec![handler2])),
        ..Default::default()
    };

    let merged = merge_configs(vec![Some(c1), Some(c2)]);
    match &merged.callbacks {
        Some(Callbacks::Manager(base_mgr)) => {
            assert_eq!(base_mgr.handlers.len(), 2);
            assert_eq!(base_mgr.handlers[0].name(), "StdOutCallbackHandler");
            assert_eq!(
                base_mgr.handlers[1].name(),
                "StreamingStdOutCallbackHandler"
            );
        }
        _ => panic!("Expected Callbacks::Manager"),
    }
}

#[test]
fn test_merge_config_callbacks_handlers_with_manager() {
    let handler1: Arc<dyn BaseCallbackHandler> = Arc::new(StdOutCallbackHandler::new());
    let handler2: Arc<dyn BaseCallbackHandler> = Arc::new(StreamingStdOutCallbackHandler::new());
    let mut mgr = CallbackManager::new();
    mgr.add_handler(handler2, true);

    let c1 = RunnableConfig {
        callbacks: Some(Callbacks::Handlers(vec![handler1])),
        ..Default::default()
    };
    let c2 = RunnableConfig {
        callbacks: Some(Callbacks::Manager(
            agent_chain_core::callbacks::BaseCallbackManager::with_handlers(
                mgr.handlers.clone(),
                Some(mgr.inheritable_handlers.clone()),
                mgr.parent_run_id,
                Some(mgr.tags.clone()),
                Some(mgr.inheritable_tags.clone()),
                Some(mgr.metadata.clone()),
                Some(mgr.inheritable_metadata.clone()),
            ),
        )),
        ..Default::default()
    };

    let merged = merge_configs(vec![Some(c1), Some(c2)]);
    match &merged.callbacks {
        Some(Callbacks::Manager(base_mgr)) => {
            assert!(!base_mgr.handlers.is_empty());
        }
        _ => panic!("Expected Callbacks::Manager"),
    }
}

#[test]
fn test_call_func_with_variable_args_simple() {
    let func = VariableArgsFn::InputOnly(Box::new(|x: String| x.to_uppercase()));
    let result = call_func_with_variable_args(&func, "hello".to_string(), &ensure_config(None));
    assert_eq!(result, "HELLO");
}

#[test]
fn test_call_func_with_variable_args_with_config() {
    let func = VariableArgsFn::WithConfig(Box::new(|x: String, config: &RunnableConfig| {
        format!("{}{}", x, config.recursion_limit)
    }));
    let result = call_func_with_variable_args(&func, "val".to_string(), &ensure_config(None));
    assert_eq!(result, "val25");
}

#[tokio::test]
async fn test_acall_func_with_variable_args_simple() {
    let func = AsyncVariableArgsFn::InputOnly(Box::new(|x: String| {
        Box::pin(async move { x.to_uppercase() })
    }));
    let result =
        acall_func_with_variable_args(&func, "hello".to_string(), &ensure_config(None)).await;
    assert_eq!(result, "HELLO");
}

#[tokio::test]
async fn test_acall_func_with_variable_args_with_config() {
    let func = AsyncVariableArgsFn::WithConfig(Box::new(|x: String, config: RunnableConfig| {
        Box::pin(async move { format!("{}{}", x, config.recursion_limit) })
    }));
    let result =
        acall_func_with_variable_args(&func, "val".to_string(), &ensure_config(None)).await;
    assert_eq!(result, "val25");
}

#[test]
fn test_get_callback_manager_for_config_basic() {
    let config = ensure_config(None);
    let mgr = get_callback_manager_for_config(&config);
    let _ = mgr;
}

#[test]
fn test_get_callback_manager_for_config_with_tags_and_metadata() {
    let config = RunnableConfig::new()
        .with_tags(vec!["a".into()])
        .with_metadata(HashMap::from([("k".to_string(), serde_json::json!("v"))]));
    let mgr = get_callback_manager_for_config(&config);
    assert!(mgr.inheritable_tags.contains(&"a".to_string()));
    assert_eq!(
        mgr.inheritable_metadata.get("k"),
        Some(&serde_json::json!("v"))
    );
}

#[test]
fn test_get_async_callback_manager_for_config_basic() {
    let config = ensure_config(None);
    let mgr = get_async_callback_manager_for_config(&config);
    let _ = mgr;
}

#[test]
fn test_get_async_callback_manager_for_config_with_tags_and_metadata() {
    let config = RunnableConfig::new()
        .with_tags(vec!["a".into()])
        .with_metadata(HashMap::from([("k".to_string(), serde_json::json!("v"))]));
    let mgr = get_async_callback_manager_for_config(&config);

    let _ = mgr;
}

#[test]
fn test_runnable_config_with_run_id() {
    let run_id = uuid::Uuid::new_v4();
    let config = RunnableConfig::new().with_run_id(run_id);
    assert_eq!(config.run_id, Some(run_id));
}

#[test]
fn test_runnable_config_with_callbacks() {
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(StdOutCallbackHandler::new());
    let callbacks = Callbacks::from_handlers(vec![handler]);
    let config = RunnableConfig::new().with_callbacks(callbacks);
    assert!(config.callbacks.is_some());
}

#[test]
fn test_runnable_config_with_configurable() {
    let mut configurable = HashMap::new();
    configurable.insert("model".to_string(), serde_json::json!("gpt-4"));
    let config = RunnableConfig::new().with_configurable(configurable);
    assert_eq!(config.configurable["model"], serde_json::json!("gpt-4"));
}

#[test]
fn test_config_or_list_from_single() {
    let config = RunnableConfig::new().with_recursion_limit(10);
    let col: ConfigOrList = config.into();
    match col {
        ConfigOrList::Single(c) => assert_eq!(c.recursion_limit, 10),
        _ => panic!("Expected Single"),
    }
}

#[test]
fn test_config_or_list_from_vec() {
    let configs = vec![
        RunnableConfig::new().with_recursion_limit(10),
        RunnableConfig::new().with_recursion_limit(20),
    ];
    let col: ConfigOrList = configs.into();
    match col {
        ConfigOrList::List(list) => {
            assert_eq!(list.len(), 2);
            assert_eq!(list[0].recursion_limit, 10);
            assert_eq!(list[1].recursion_limit, 20);
        }
        _ => panic!("Expected List"),
    }
}

#[test]
fn test_runnable_config_serialization_roundtrip() {
    let config = RunnableConfig::new()
        .with_tags(vec!["tag1".into(), "tag2".into()])
        .with_run_name("test_run")
        .with_max_concurrency(4)
        .with_recursion_limit(10);

    let json = serde_json::to_string(&config).expect("serialization should succeed");
    let deserialized: RunnableConfig =
        serde_json::from_str(&json).expect("deserialization should succeed");

    assert_eq!(deserialized.tags, vec!["tag1", "tag2"]);
    assert_eq!(deserialized.run_name, Some("test_run".to_string()));
    assert_eq!(deserialized.max_concurrency, Some(4));
    assert_eq!(deserialized.recursion_limit, 10);
    assert!(deserialized.callbacks.is_none());
}

#[test]
fn test_runnable_config_deserialization_defaults() {
    let json = "{}";
    let config: RunnableConfig =
        serde_json::from_str(json).expect("deserialization should succeed");
    assert!(config.tags.is_empty());
    assert!(config.metadata.is_empty());
    assert_eq!(config.recursion_limit, 25);
    assert!(config.configurable.is_empty());
}

#[test]
fn test_merge_configs_max_concurrency_last_wins() {
    let c1 = RunnableConfig::new().with_max_concurrency(2);
    let c2 = RunnableConfig::new().with_max_concurrency(8);
    let merged = merge_configs(vec![Some(c1), Some(c2)]);
    assert_eq!(merged.max_concurrency, Some(8));
}

#[test]
fn test_merge_configs_single_config() {
    let config = RunnableConfig::new()
        .with_tags(vec!["only".into()])
        .with_recursion_limit(42);
    let merged = merge_configs(vec![Some(config)]);
    assert_eq!(merged.tags, vec!["only"]);
    assert_eq!(merged.recursion_limit, 42);
}

#[test]
fn test_merge_configs_all_none() {
    let merged = merge_configs(vec![None, None, None]);
    assert!(merged.tags.is_empty());
    assert_eq!(merged.recursion_limit, 25);
}

#[test]
fn test_patch_config_preserves_existing_tags() {
    let config = RunnableConfig::new().with_tags(vec!["existing".into()]);
    let patched = patch_config(Some(config), None, None, None, None, None);
    assert_eq!(patched.tags, vec!["existing"]);
}

#[test]
fn test_patch_config_preserves_callbacks_when_not_replaced() {
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(StdOutCallbackHandler::new());
    let config = RunnableConfig::new()
        .with_callbacks(Callbacks::from_handlers(vec![handler]))
        .with_run_name("keep_me")
        .with_run_id(uuid::Uuid::new_v4());

    let patched = patch_config(Some(config), None, None, None, Some(99), None);

    assert_eq!(patched.run_name, Some("keep_me".to_string()));
    assert!(patched.run_id.is_some());
    assert_eq!(patched.recursion_limit, 99);
}

#[test]
fn test_get_config_list_single_with_length_one() {
    let config = RunnableConfig::new()
        .with_recursion_limit(42)
        .with_tags(vec!["solo".into()]);
    let configs = get_config_list(Some(ConfigOrList::Single(Box::new(config))), 1);
    assert_eq!(configs.len(), 1);
    assert_eq!(configs[0].recursion_limit, 42);
    assert_eq!(configs[0].tags, vec!["solo"]);
}

#[test]
fn test_get_config_list_empty_list() {
    let configs = get_config_list(Some(ConfigOrList::List(vec![])), 0);
    assert!(configs.is_empty());
}

#[test]
fn test_get_config_list_preserves_all_fields() {
    let run_id = uuid::Uuid::new_v4();
    let config = RunnableConfig::new()
        .with_tags(vec!["t1".into()])
        .with_run_name("run")
        .with_max_concurrency(3)
        .with_recursion_limit(15)
        .with_run_id(run_id);

    let configs = get_config_list(Some(ConfigOrList::Single(Box::new(config))), 1);
    assert_eq!(configs.len(), 1);
    assert_eq!(configs[0].tags, vec!["t1"]);
    assert_eq!(configs[0].run_name, Some("run".to_string()));
    assert_eq!(configs[0].max_concurrency, Some(3));
    assert_eq!(configs[0].recursion_limit, 15);
    assert_eq!(configs[0].run_id, Some(run_id));
}
