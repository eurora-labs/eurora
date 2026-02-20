use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use serde_json::{Value, json};

use agent_chain_core::runnables::utils::{
    AddableDict, ConfigurableField, ConfigurableFieldMultiOption, ConfigurableFieldSingleOption,
    ConfigurableFieldSpec, RootEventFilter, gather_with_concurrency, get_unique_config_specs,
    indent_lines_after_first,
};

struct IndentTestCase {
    text: &'static str,
    prefix: &'static str,
    expected: &'static str,
}

fn get_indent_test_cases() -> Vec<IndentTestCase> {
    vec![
        IndentTestCase {
            text: "line 1\nline 2\nline 3",
            prefix: "1",
            expected: "line 1\n line 2\n line 3",
        },
        IndentTestCase {
            text: "line 1\nline 2\nline 3",
            prefix: "ax",
            expected: "line 1\n  line 2\n  line 3",
        },
    ]
}

#[test]
fn test_indent_lines_after_first() {
    for (i, case) in get_indent_test_cases().into_iter().enumerate() {
        let result = indent_lines_after_first(case.text, case.prefix);
        assert_eq!(result, case.expected, "Test case {} failed", i);
    }
}

#[test]
fn test_indent_lines_after_first_single_line() {
    let result = indent_lines_after_first("single line", "xx");
    assert_eq!(result, "single line");
}

#[test]
fn test_indent_lines_after_first_empty_prefix() {
    let result = indent_lines_after_first("a\nb\nc", "");
    assert_eq!(result, "a\nb\nc");
}

#[test]
fn test_addable_dict_add_basic() {
    let mut a = AddableDict::new();
    a.0.insert("x".to_string(), json!(1));
    a.0.insert("y".to_string(), json!("hello"));

    let mut b = AddableDict::new();
    b.0.insert("x".to_string(), json!(2));
    b.0.insert("y".to_string(), json!(" world"));

    let result = a + b;
    assert_eq!(result.0.get("x"), Some(&json!(3)));
    assert_eq!(result.0.get("y"), Some(&json!("hello world")));
}

#[test]
fn test_addable_dict_add_new_keys() {
    let mut a = AddableDict::new();
    a.0.insert("x".to_string(), json!(1));

    let mut b = AddableDict::new();
    b.0.insert("y".to_string(), json!(2));

    let result = a + b;
    assert_eq!(result.0.get("x"), Some(&json!(1)));
    assert_eq!(result.0.get("y"), Some(&json!(2)));
}

#[test]
fn test_addable_dict_add_none_values() {
    let mut a = AddableDict::new();
    a.0.insert("x".to_string(), Value::Null);

    let mut b = AddableDict::new();
    b.0.insert("x".to_string(), json!(5));

    let result = a + b;
    assert_eq!(result.0.get("x"), Some(&json!(5)));

    let mut a2 = AddableDict::new();
    a2.0.insert("x".to_string(), json!(5));

    let mut b2 = AddableDict::new();
    b2.0.insert("x".to_string(), Value::Null);

    let result2 = a2 + b2;
    assert_eq!(result2.0.get("x"), Some(&json!(5)));
}

#[test]
fn test_addable_dict_add_type_error_fallback() {
    let mut a = AddableDict::new();
    a.0.insert("x".to_string(), json!(1));

    let mut b = AddableDict::new();
    b.0.insert("x".to_string(), json!("string"));

    let result = a + b;
    assert_eq!(result.0.get("x"), Some(&json!("string")));
}

#[test]
fn test_addable_dict_radd() {
    let a: HashMap<String, Value> = [("x".to_string(), json!(1))].into_iter().collect();

    let mut b = AddableDict::new();
    b.0.insert("x".to_string(), json!(2));
    b.0.insert("y".to_string(), json!(3));

    let result = AddableDict::from_map(a) + b;
    assert_eq!(result.0.get("x"), Some(&json!(3)));
    assert_eq!(result.0.get("y"), Some(&json!(3)));
}

#[test]
fn test_addable_dict_radd_new_keys() {
    let a: HashMap<String, Value> = [("a".to_string(), json!(1))].into_iter().collect();

    let mut b = AddableDict::new();
    b.0.insert("b".to_string(), json!(2));

    let result = AddableDict::from_map(a) + b;
    assert_eq!(result.0.get("a"), Some(&json!(1)));
    assert_eq!(result.0.get("b"), Some(&json!(2)));
}

#[test]
fn test_addable_dict_preserves_dict_behavior() {
    let mut d = AddableDict::new();
    d.0.insert("key".to_string(), json!("value"));

    assert_eq!(d.0.get("key"), Some(&json!("value")));
    let keys: Vec<&String> = d.0.keys().collect();
    assert_eq!(keys, vec!["key"]);
    assert_eq!(d.0.len(), 1);
}

#[tokio::test]
async fn test_gather_with_concurrency_none() {
    let futures: Vec<Pin<Box<dyn Future<Output = i32> + Send>>> = vec![
        Box::pin(async { 1 }),
        Box::pin(async { 2 }),
        Box::pin(async { 3 }),
    ];

    let results = gather_with_concurrency(None, futures).await;
    assert_eq!(results, vec![1, 2, 3]);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_gather_with_concurrency_limited() {
    let running = Arc::new(AtomicUsize::new(0));
    let max_running = Arc::new(AtomicUsize::new(0));

    let futures: Vec<Pin<Box<dyn Future<Output = i32> + Send>>> = (1..=4)
        .map(|x| {
            let running = running.clone();
            let max_running = max_running.clone();
            Box::pin(async move {
                let current = running.fetch_add(1, Ordering::SeqCst) + 1;
                max_running.fetch_max(current, Ordering::SeqCst);
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                running.fetch_sub(1, Ordering::SeqCst);
                x
            }) as Pin<Box<dyn Future<Output = i32> + Send>>
        })
        .collect();

    let results = gather_with_concurrency(Some(2), futures).await;
    let mut sorted_results = results.clone();
    sorted_results.sort();
    assert_eq!(sorted_results, vec![1, 2, 3, 4]);
    assert!(max_running.load(Ordering::SeqCst) <= 2);
}

#[tokio::test]
async fn test_gather_with_concurrency_empty() {
    let futures: Vec<Pin<Box<dyn Future<Output = i32> + Send>>> = vec![];
    let results = gather_with_concurrency(Some(5), futures).await;
    assert!(results.is_empty());
}

#[test]
fn test_configurable_field_defaults() {
    let field = ConfigurableField::new("test_id");
    assert_eq!(field.id, "test_id");
    assert_eq!(field.name, None);
    assert_eq!(field.description, None);
    assert_eq!(field.annotation, None);
    assert!(!field.is_shared);
}

#[test]
fn test_configurable_field_with_values() {
    let field = ConfigurableField::new("temp")
        .with_name("Temperature")
        .with_description("The LLM temperature")
        .with_annotation("float")
        .with_shared(true);

    assert_eq!(field.id, "temp");
    assert_eq!(field.name, Some("Temperature".to_string()));
    assert_eq!(field.annotation, Some("float".to_string()));
    assert!(field.is_shared);
}

#[test]
fn test_configurable_field_hash() {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    fn compute_hash<T: Hash>(value: &T) -> u64 {
        let mut hasher = DefaultHasher::new();
        value.hash(&mut hasher);
        hasher.finish()
    }

    let f1 = ConfigurableField::new("a").with_annotation("int");
    let f2 = ConfigurableField::new("a").with_annotation("int");
    let f3 = ConfigurableField::new("b").with_annotation("int");

    assert_eq!(compute_hash(&f1), compute_hash(&f2));
    assert_ne!(compute_hash(&f1), compute_hash(&f3));
}

#[test]
fn test_configurable_field_single_option() {
    let options: HashMap<String, Value> = [
        ("gpt4".to_string(), json!("gpt-4")),
        ("gpt3".to_string(), json!("gpt-3.5")),
    ]
    .into_iter()
    .collect();

    let field = ConfigurableFieldSingleOption::new("model", options, "gpt4");

    assert_eq!(field.id, "model");
    assert_eq!(field.default, "gpt4");
    assert_eq!(field.options.get("gpt4"), Some(&json!("gpt-4")));
}

#[test]
fn test_configurable_field_single_option_hash() {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    fn compute_hash<T: Hash>(value: &T) -> u64 {
        let mut hasher = DefaultHasher::new();
        value.hash(&mut hasher);
        hasher.finish()
    }

    let options: HashMap<String, Value> =
        [("a".to_string(), json!(1)), ("b".to_string(), json!(2))]
            .into_iter()
            .collect();

    let f1 = ConfigurableFieldSingleOption::new("m", options.clone(), "a");
    let f2 = ConfigurableFieldSingleOption::new("m", options, "a");

    assert_eq!(compute_hash(&f1), compute_hash(&f2));
}

#[test]
fn test_configurable_field_multi_option() {
    let options: HashMap<String, Value> = [
        ("search".to_string(), json!("web_search")),
        ("calc".to_string(), json!("calculator")),
    ]
    .into_iter()
    .collect();

    let field = ConfigurableFieldMultiOption::new("tools", options, vec!["search".to_string()]);

    assert_eq!(field.id, "tools");
    assert_eq!(field.default, vec!["search".to_string()]);
    assert_eq!(field.options.len(), 2);
}

#[test]
fn test_configurable_field_multi_option_hash() {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    fn compute_hash<T: Hash>(value: &T) -> u64 {
        let mut hasher = DefaultHasher::new();
        value.hash(&mut hasher);
        hasher.finish()
    }

    let options: HashMap<String, Value> = [("a".to_string(), json!(1))].into_iter().collect();

    let f1 = ConfigurableFieldMultiOption::new("t", options.clone(), vec!["a".to_string()]);
    let f2 = ConfigurableFieldMultiOption::new("t", options, vec!["a".to_string()]);

    assert_eq!(compute_hash(&f1), compute_hash(&f2));
}

#[test]
fn test_configurable_field_spec_defaults() {
    let spec = ConfigurableFieldSpec::new("s", "str");
    assert_eq!(spec.id, "s");
    assert_eq!(spec.annotation, "str");
    assert_eq!(spec.name, None);
    assert_eq!(spec.description, None);
    assert_eq!(spec.default, None);
    assert!(!spec.is_shared);
    assert_eq!(spec.dependencies, None);
}

#[test]
fn test_configurable_field_spec_with_dependencies() {
    let mut spec = ConfigurableFieldSpec::new("s", "str");
    spec.dependencies = Some(vec!["dep1".to_string(), "dep2".to_string()]);
    assert_eq!(
        spec.dependencies,
        Some(vec!["dep1".to_string(), "dep2".to_string()])
    );
}

#[test]
fn test_get_unique_config_specs_no_duplicates() {
    let specs = vec![
        ConfigurableFieldSpec::new("a", "str"),
        ConfigurableFieldSpec::new("b", "int"),
    ];
    let result = get_unique_config_specs(specs).unwrap();
    assert_eq!(result.len(), 2);
}

#[test]
fn test_get_unique_config_specs_identical_duplicates() {
    let spec = ConfigurableFieldSpec::new("a", "str");
    let result = get_unique_config_specs(vec![spec.clone(), spec]).unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].id, "a");
}

#[test]
fn test_get_unique_config_specs_conflicting_raises() {
    let s1 = ConfigurableFieldSpec::new("a", "str");
    let mut s2 = ConfigurableFieldSpec::new("a", "int");
    s2.default = Some(json!("y"));

    let result = get_unique_config_specs(vec![s1, s2]);
    assert!(result.is_err());
    let err_msg = result.unwrap_err();
    assert!(
        err_msg.contains("conflicting"),
        "Error should mention 'conflicting': {}",
        err_msg
    );
}

#[test]
fn test_get_unique_config_specs_empty() {
    let result = get_unique_config_specs(Vec::<ConfigurableFieldSpec>::new()).unwrap();
    assert!(result.is_empty());
}

#[test]
fn test_root_event_filter_include_all_by_default() {
    let f = RootEventFilter::new();
    assert!(f.include_event("test", &["a".to_string()], "chain"));
}

#[test]
fn test_root_event_filter_include_names() {
    let f = RootEventFilter {
        include_names: Some(vec!["foo".to_string()]),
        ..RootEventFilter::new()
    };
    assert!(f.include_event("foo", &[], "chain"));
    assert!(!f.include_event("bar", &[], "chain"));
}

#[test]
fn test_root_event_filter_include_types() {
    let f = RootEventFilter {
        include_types: Some(vec!["llm".to_string()]),
        ..RootEventFilter::new()
    };
    assert!(f.include_event("x", &[], "llm"));
    assert!(!f.include_event("x", &[], "chain"));
}

#[test]
fn test_root_event_filter_include_tags() {
    let f = RootEventFilter {
        include_tags: Some(vec!["my_tag".to_string()]),
        ..RootEventFilter::new()
    };
    assert!(f.include_event("x", &["my_tag".to_string()], "chain"));
    assert!(!f.include_event("x", &["other".to_string()], "chain"));
}

#[test]
fn test_root_event_filter_exclude_names() {
    let f = RootEventFilter {
        exclude_names: Some(vec!["bad".to_string()]),
        ..RootEventFilter::new()
    };
    assert!(f.include_event("good", &[], "chain"));
    assert!(!f.include_event("bad", &[], "chain"));
}

#[test]
fn test_root_event_filter_exclude_types() {
    let f = RootEventFilter {
        exclude_types: Some(vec!["llm".to_string()]),
        ..RootEventFilter::new()
    };
    assert!(f.include_event("x", &[], "chain"));
    assert!(!f.include_event("x", &[], "llm"));
}

#[test]
fn test_root_event_filter_exclude_tags() {
    let f = RootEventFilter {
        exclude_tags: Some(vec!["secret".to_string()]),
        ..RootEventFilter::new()
    };
    assert!(f.include_event("x", &["public".to_string()], "chain"));
    assert!(!f.include_event("x", &["secret".to_string()], "chain"));
}

#[test]
fn test_root_event_filter_include_and_exclude_combined() {
    let f = RootEventFilter {
        include_names: Some(vec!["foo".to_string()]),
        exclude_tags: Some(vec!["no".to_string()]),
        ..RootEventFilter::new()
    };
    assert!(f.include_event("foo", &[], "chain"));
    assert!(!f.include_event("foo", &["no".to_string()], "chain"));
    assert!(!f.include_event("bar", &[], "chain"));
}

#[test]
fn test_root_event_filter_no_tags_in_event() {
    let f = RootEventFilter {
        include_tags: Some(vec!["needed".to_string()]),
        ..RootEventFilter::new()
    };
    assert!(!f.include_event("x", &[], "chain"));
}
