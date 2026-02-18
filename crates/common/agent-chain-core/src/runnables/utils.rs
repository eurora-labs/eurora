use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use futures::stream::{Stream, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::Semaphore;

/// Run a coroutine with a semaphore.
///
/// This is a helper function that acquires a permit from the semaphore before
/// running the future, and releases it when the future completes.
///
/// # Arguments
/// * `semaphore` - The semaphore to use for limiting concurrency
/// * `fut` - The future to run
///
/// # Returns
/// The result of the future
pub async fn gated_coro<T>(semaphore: Arc<Semaphore>, fut: impl Future<Output = T>) -> T {
    let _permit = semaphore
        .acquire()
        .await
        .expect("semaphore should not be closed");
    fut.await
}

/// Gather futures with a limit on the number of concurrent futures.
///
/// This function runs multiple futures concurrently, but limits the number of
/// futures that can run at the same time using a semaphore.
///
/// # Arguments
/// * `n` - The maximum number of futures to run concurrently. If None, all futures run concurrently.
/// * `futures` - The futures to run
///
/// # Returns
/// A vector of results in the same order as the input futures
pub async fn gather_with_concurrency<T: Send + 'static>(
    n: Option<usize>,
    futures: Vec<Pin<Box<dyn Future<Output = T> + Send>>>,
) -> Vec<T> {
    if futures.is_empty() {
        return Vec::new();
    }

    match n {
        Some(limit) if limit > 0 => {
            let semaphore = Arc::new(Semaphore::new(limit));
            let gated_futures: Vec<_> = futures
                .into_iter()
                .map(|fut| {
                    let sem = semaphore.clone();
                    Box::pin(gated_coro(sem, fut)) as Pin<Box<dyn Future<Output = T> + Send>>
                })
                .collect();
            futures::future::join_all(gated_futures).await
        }
        _ => futures::future::join_all(futures).await,
    }
}

/// Indent all lines of text after the first line.
///
/// # Arguments
/// * `text` - The text to indent
/// * `prefix` - Used to determine the number of spaces to indent (uses len of prefix)
///
/// # Returns
/// The indented text with all lines after the first indented by spaces equal to prefix length
pub fn indent_lines_after_first(text: &str, prefix: &str) -> String {
    let n_spaces = prefix.len();
    let spaces = " ".repeat(n_spaces);
    let lines: Vec<&str> = text.lines().collect();

    if lines.is_empty() {
        return String::new();
    }

    let mut result = lines[0].to_string();
    for line in &lines[1..] {
        result.push('\n');
        result.push_str(&spaces);
        result.push_str(line);
    }

    result
}

/// Dictionary that can be added to another dictionary.
///
/// When adding two AddableDict instances:
/// - Keys only in the other dict are added
/// - Keys with null values in self are replaced
/// - Values that support addition are added together
/// - Otherwise the value from other replaces the value in self
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AddableDict(pub HashMap<String, Value>);

impl AddableDict {
    /// Create a new empty AddableDict
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    /// Create an AddableDict from an existing HashMap
    pub fn from_map(map: HashMap<String, Value>) -> Self {
        Self(map)
    }
}

impl std::ops::Add for AddableDict {
    type Output = Self;

    fn add(self, other: Self) -> Self::Output {
        let mut chunk = self.clone();

        for (key, value) in other.0 {
            match chunk.0.get(&key) {
                None => {
                    chunk.0.insert(key, value);
                }
                Some(existing) if existing.is_null() => {
                    chunk.0.insert(key, value);
                }
                Some(existing) if !value.is_null() => {
                    let added = try_add_values(existing, &value);
                    chunk.0.insert(key, added);
                }
                _ => {}
            }
        }

        chunk
    }
}

/// Try to add two JSON values together.
///
/// - Strings are concatenated
/// - Arrays are concatenated
/// - Objects are merged (later values overwrite)
/// - Numbers are added
/// - For other types or mismatched types, returns the second value
fn try_add_values(a: &Value, b: &Value) -> Value {
    match (a, b) {
        (Value::String(s1), Value::String(s2)) => Value::String(format!("{}{}", s1, s2)),
        (Value::Array(arr1), Value::Array(arr2)) => {
            let mut result = arr1.clone();
            result.extend(arr2.clone());
            Value::Array(result)
        }
        (Value::Object(obj1), Value::Object(obj2)) => {
            let mut result = obj1.clone();
            for (k, v) in obj2 {
                result.insert(k.clone(), v.clone());
            }
            Value::Object(result)
        }
        (Value::Number(n1), Value::Number(n2)) => {
            if let (Some(i1), Some(i2)) = (n1.as_i64(), n2.as_i64()) {
                Value::Number((i1 + i2).into())
            } else if let (Some(f1), Some(f2)) = (n1.as_f64(), n2.as_f64()) {
                serde_json::Number::from_f64(f1 + f2)
                    .map(Value::Number)
                    .unwrap_or_else(|| b.clone())
            } else {
                b.clone()
            }
        }
        _ => b.clone(),
    }
}

/// Trait for types that support addition.
///
/// This is the Rust equivalent of Python's `SupportsAdd` protocol.
pub trait Addable: Clone {
    /// Add another value to this one
    fn add(self, other: Self) -> Self;
}

/// Add a sequence of addable objects together.
///
/// # Arguments
/// * `addables` - An iterator of addable objects
///
/// # Returns
/// The result of adding all objects, or None if the iterator was empty
pub fn add<T: Addable>(addables: impl IntoIterator<Item = T>) -> Option<T> {
    let mut final_value: Option<T> = None;

    for chunk in addables {
        final_value = match final_value {
            None => Some(chunk),
            Some(prev) => Some(prev.add(chunk)),
        };
    }

    final_value
}

/// Asynchronously add a sequence of addable objects together.
///
/// # Arguments
/// * `addables` - A stream of addable objects
///
/// # Returns
/// The result of adding all objects, or None if the stream was empty
pub async fn aadd<T: Addable>(addables: impl Stream<Item = T> + Unpin) -> Option<T> {
    let mut final_value: Option<T> = None;
    let mut stream = addables;

    while let Some(chunk) = stream.next().await {
        final_value = match final_value {
            None => Some(chunk),
            Some(prev) => Some(prev.add(chunk)),
        };
    }

    final_value
}

impl Addable for String {
    fn add(self, other: Self) -> Self {
        self + &other
    }
}

impl Addable for Value {
    fn add(self, other: Self) -> Self {
        try_add_values(&self, &other)
    }
}

impl Addable for AddableDict {
    fn add(self, other: Self) -> Self {
        std::ops::Add::add(self, other)
    }
}

impl Addable for HashMap<String, Value> {
    fn add(mut self, other: Self) -> Self {
        for (key, value) in other {
            match self.get(&key) {
                None => {
                    self.insert(key, value);
                }
                Some(existing) if existing.is_null() => {
                    self.insert(key, value);
                }
                Some(existing) if !value.is_null() => {
                    let added = try_add_values(existing, &value);
                    self.insert(key, added);
                }
                _ => {}
            }
        }
        self
    }
}

/// Field that can be configured by the user.
///
/// This corresponds to Python's `ConfigurableField` NamedTuple.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ConfigurableField {
    /// The unique identifier of the field
    pub id: String,
    /// The name of the field
    pub name: Option<String>,
    /// The description of the field
    pub description: Option<String>,
    /// The annotation of the field (type hint as string in Rust)
    pub annotation: Option<String>,
    /// Whether the field is shared across runnables
    pub is_shared: bool,
}

impl ConfigurableField {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: None,
            description: None,
            annotation: None,
            is_shared: false,
        }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_annotation(mut self, annotation: impl Into<String>) -> Self {
        self.annotation = Some(annotation.into());
        self
    }

    pub fn with_shared(mut self, is_shared: bool) -> Self {
        self.is_shared = is_shared;
        self
    }
}

/// Field that can be configured by the user with a single option from a set.
///
/// This corresponds to Python's `ConfigurableFieldSingleOption` NamedTuple.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigurableFieldSingleOption {
    /// The unique identifier of the field
    pub id: String,
    /// The available options for the field
    pub options: HashMap<String, serde_json::Value>,
    /// The default option key
    pub default: String,
    /// The name of the field
    pub name: Option<String>,
    /// The description of the field
    pub description: Option<String>,
    /// Whether the field is shared across runnables
    pub is_shared: bool,
}

impl std::hash::Hash for ConfigurableFieldSingleOption {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
        let mut keys: Vec<_> = self.options.keys().collect();
        keys.sort();
        for key in keys {
            key.hash(state);
        }
        self.default.hash(state);
    }
}

impl ConfigurableFieldSingleOption {
    pub fn new(
        id: impl Into<String>,
        options: HashMap<String, serde_json::Value>,
        default: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            options,
            default: default.into(),
            name: None,
            description: None,
            is_shared: false,
        }
    }
}

/// Field that can be configured by the user with multiple options from a set.
///
/// This corresponds to Python's `ConfigurableFieldMultiOption` NamedTuple.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigurableFieldMultiOption {
    /// The unique identifier of the field
    pub id: String,
    /// The available options for the field
    pub options: HashMap<String, serde_json::Value>,
    /// The default option keys (multiple can be selected)
    pub default: Vec<String>,
    /// The name of the field
    pub name: Option<String>,
    /// The description of the field
    pub description: Option<String>,
    /// Whether the field is shared across runnables
    pub is_shared: bool,
}

impl std::hash::Hash for ConfigurableFieldMultiOption {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
        let mut keys: Vec<_> = self.options.keys().collect();
        keys.sort();
        for key in keys {
            key.hash(state);
        }
        for d in &self.default {
            d.hash(state);
        }
    }
}

impl ConfigurableFieldMultiOption {
    pub fn new(
        id: impl Into<String>,
        options: HashMap<String, serde_json::Value>,
        default: Vec<String>,
    ) -> Self {
        Self {
            id: id.into(),
            options,
            default,
            name: None,
            description: None,
            is_shared: false,
        }
    }
}

/// Union type for any configurable field variant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnyConfigurableField {
    Field(ConfigurableField),
    SingleOption(ConfigurableFieldSingleOption),
    MultiOption(ConfigurableFieldMultiOption),
}

/// Specification of a configurable field.
///
/// This corresponds to Python's `ConfigurableFieldSpec` NamedTuple.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigurableFieldSpec {
    /// The unique identifier of the field
    pub id: String,
    /// The annotation (type) of the field
    pub annotation: String,
    /// The name of the field
    pub name: Option<String>,
    /// The description of the field
    pub description: Option<String>,
    /// The default value for the field
    pub default: Option<serde_json::Value>,
    /// Whether the field is shared across runnables
    pub is_shared: bool,
    /// Dependencies on other fields
    pub dependencies: Option<Vec<String>>,
}

impl ConfigurableFieldSpec {
    pub fn new(id: impl Into<String>, annotation: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            annotation: annotation.into(),
            name: None,
            description: None,
            default: None,
            is_shared: false,
            dependencies: None,
        }
    }
}

/// Get the unique config specs from a sequence of config specs.
///
/// This function groups specs by ID and ensures there are no conflicts.
/// If two specs have the same ID but different values, an error is returned.
///
/// # Arguments
/// * `specs` - An iterable of config specs
///
/// # Returns
/// A vector of unique config specs, or an error if there are conflicts
pub fn get_unique_config_specs(
    specs: impl IntoIterator<Item = ConfigurableFieldSpec>,
) -> Result<Vec<ConfigurableFieldSpec>, String> {
    use std::collections::BTreeMap;

    let mut grouped: BTreeMap<String, Vec<ConfigurableFieldSpec>> = BTreeMap::new();

    for spec in specs {
        grouped.entry(spec.id.clone()).or_default().push(spec);
    }

    let mut unique = Vec::new();

    for (spec_id, dupes) in grouped {
        if dupes.is_empty() {
            continue;
        }

        let first = &dupes[0];

        if dupes.len() == 1 || dupes.iter().skip(1).all(|s| s == first) {
            unique.push(first.clone());
        } else {
            return Err(format!(
                "RunnableSequence contains conflicting config specs for {}: {:?}",
                spec_id, dupes
            ));
        }
    }

    Ok(unique)
}

/// Utility to filter events in the astream_events implementation.
///
/// This class provides filtering based on names, types, and tags for both
/// inclusion and exclusion criteria.
#[derive(Debug, Clone)]
pub struct RootEventFilter {
    /// Names to include (if any match, include)
    pub include_names: Option<Vec<String>>,
    /// Types to include (if any match, include)
    pub include_types: Option<Vec<String>>,
    /// Tags to include (if any match, include)
    pub include_tags: Option<Vec<String>>,
    /// Names to exclude (if any match, exclude)
    pub exclude_names: Option<Vec<String>>,
    /// Types to exclude (if any match, exclude)
    pub exclude_types: Option<Vec<String>>,
    /// Tags to exclude (if any match, exclude)
    pub exclude_tags: Option<Vec<String>>,
}

impl RootEventFilter {
    /// Create a new event filter with no filters applied
    pub fn new() -> Self {
        Self {
            include_names: None,
            include_types: None,
            include_tags: None,
            exclude_names: None,
            exclude_types: None,
            exclude_tags: None,
        }
    }

    /// Determine whether to include an event based on the filter criteria.
    ///
    /// # Arguments
    /// * `event_name` - The name of the event
    /// * `event_tags` - Tags associated with the event
    /// * `root_type` - The type of the root runnable
    ///
    /// # Returns
    /// `true` if the event should be included, `false` otherwise
    pub fn include_event(&self, event_name: &str, event_tags: &[String], root_type: &str) -> bool {
        let mut include = self.include_names.is_none()
            && self.include_types.is_none()
            && self.include_tags.is_none();

        if let Some(names) = &self.include_names {
            include = include || names.iter().any(|n| n == event_name);
        }

        if let Some(types) = &self.include_types {
            include = include || types.iter().any(|t| t == root_type);
        }

        if let Some(tags) = &self.include_tags {
            include = include || event_tags.iter().any(|tag| tags.contains(tag));
        }

        if let Some(names) = &self.exclude_names {
            include = include && !names.iter().any(|n| n == event_name);
        }

        if let Some(types) = &self.exclude_types {
            include = include && !types.iter().any(|t| t == root_type);
        }

        if let Some(tags) = &self.exclude_tags {
            include = include && !event_tags.iter().any(|tag| tags.contains(tag));
        }

        include
    }
}

impl Default for RootEventFilter {
    fn default() -> Self {
        Self::new()
    }
}

/// Check if a function is an async generator.
///
/// In Rust, this is determined at compile time through type bounds rather than
/// runtime inspection as in Python. This function serves as a marker for
/// API compatibility.
///
/// Note: In Rust, async generators are represented as types implementing
/// `Stream` rather than a special function type.
///
/// # Arguments
/// * `_f` - The function to check (unused, type is checked at compile time)
///
/// # Returns
/// Always returns `true` when the type bounds are satisfied
pub fn is_async_generator<F, S, T>(_f: F) -> bool
where
    F: Fn() -> S,
    S: Stream<Item = T>,
{
    true
}

/// Check if a function is async.
///
/// In Rust, this is determined at compile time through type bounds rather than
/// runtime inspection as in Python. This function serves as a marker for
/// API compatibility.
///
/// # Arguments
/// * `_f` - The function to check (unused, type is checked at compile time)
///
/// # Returns
/// Always returns `true` when the type bounds are satisfied
pub fn is_async_callable<F, Fut>(_f: F) -> bool
where
    F: Fn() -> Fut,
    Fut: Future,
{
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_indent_lines_after_first() {
        let text = "line1\nline2\nline3";
        let result = indent_lines_after_first(text, "  ");
        assert_eq!(result, "line1\n  line2\n  line3");
    }

    #[tokio::test]
    async fn test_gather_with_concurrency() {
        let futures: Vec<Pin<Box<dyn Future<Output = i32> + Send>>> = vec![
            Box::pin(async { 1 }),
            Box::pin(async { 2 }),
            Box::pin(async { 3 }),
        ];

        let results = gather_with_concurrency(Some(2), futures).await;
        assert_eq!(results, vec![1, 2, 3]);
    }

    #[tokio::test]
    async fn test_gather_with_concurrency_no_limit() {
        let futures: Vec<Pin<Box<dyn Future<Output = i32> + Send>>> = vec![
            Box::pin(async { 1 }),
            Box::pin(async { 2 }),
            Box::pin(async { 3 }),
        ];

        let results = gather_with_concurrency(None, futures).await;
        assert_eq!(results, vec![1, 2, 3]);
    }

    #[tokio::test]
    async fn test_gather_with_concurrency_empty() {
        let futures: Vec<Pin<Box<dyn Future<Output = i32> + Send>>> = vec![];
        let results = gather_with_concurrency(Some(2), futures).await;
        assert!(results.is_empty());
    }

    #[test]
    fn test_addable_dict() {
        let mut dict1 = AddableDict::new();
        dict1
            .0
            .insert("a".to_string(), Value::String("hello".to_string()));
        dict1.0.insert("b".to_string(), Value::Number(1.into()));

        let mut dict2 = AddableDict::new();
        dict2.0.insert("b".to_string(), Value::Number(2.into()));
        dict2
            .0
            .insert("c".to_string(), Value::String(" world".to_string()));

        let result = dict1 + dict2;
        assert_eq!(result.0.get("a"), Some(&Value::String("hello".to_string())));
        assert_eq!(result.0.get("b"), Some(&Value::Number(3.into())));
        assert_eq!(
            result.0.get("c"),
            Some(&Value::String(" world".to_string()))
        );
    }

    #[test]
    fn test_configurable_field() {
        let field = ConfigurableField::new("test_id")
            .with_name("Test Field")
            .with_description("A test field")
            .with_shared(true);

        assert_eq!(field.id, "test_id");
        assert_eq!(field.name, Some("Test Field".to_string()));
        assert_eq!(field.description, Some("A test field".to_string()));
        assert!(field.is_shared);
    }

    #[test]
    fn test_get_unique_config_specs() {
        let spec1 = ConfigurableFieldSpec::new("id1", "String");
        let spec2 = ConfigurableFieldSpec::new("id1", "String");
        let spec3 = ConfigurableFieldSpec::new("id2", "Int");

        let specs = vec![spec1, spec2, spec3];
        let result = get_unique_config_specs(specs).unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].id, "id1");
        assert_eq!(result[1].id, "id2");
    }

    #[test]
    fn test_get_unique_config_specs_conflict() {
        let spec1 = ConfigurableFieldSpec::new("id1", "String");
        let mut spec2 = ConfigurableFieldSpec::new("id1", "String");
        spec2.description = Some("Different".to_string());

        let specs = vec![spec1, spec2];
        let result = get_unique_config_specs(specs);

        assert!(result.is_err());
    }

    #[test]
    fn test_root_event_filter() {
        let filter = RootEventFilter {
            include_names: Some(vec!["test".to_string()]),
            include_types: None,
            include_tags: None,
            exclude_names: None,
            exclude_types: None,
            exclude_tags: None,
        };

        assert!(filter.include_event("test", &[], "chain"));
        assert!(!filter.include_event("other", &[], "chain"));
    }

    #[test]
    fn test_root_event_filter_tags() {
        let filter = RootEventFilter {
            include_names: None,
            include_types: None,
            include_tags: Some(vec!["important".to_string()]),
            exclude_names: None,
            exclude_types: None,
            exclude_tags: None,
        };

        assert!(filter.include_event("test", &["important".to_string()], "chain"));
        assert!(!filter.include_event("test", &["unimportant".to_string()], "chain"));
    }

    #[test]
    fn test_root_event_filter_exclude() {
        let filter = RootEventFilter {
            include_names: None,
            include_types: None,
            include_tags: None,
            exclude_names: Some(vec!["skip".to_string()]),
            exclude_types: None,
            exclude_tags: None,
        };

        assert!(filter.include_event("test", &[], "chain"));
        assert!(!filter.include_event("skip", &[], "chain"));
    }
}
