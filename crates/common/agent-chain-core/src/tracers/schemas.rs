use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum RunType {
    Tool,
    Chain,
    Llm,
    Retriever,
    Embedding,
    Prompt,
    Parser,
    ChatModel,
}

impl std::fmt::Display for RunType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RunType::Tool => write!(f, "tool"),
            RunType::Chain => write!(f, "chain"),
            RunType::Llm => write!(f, "llm"),
            RunType::Retriever => write!(f, "retriever"),
            RunType::Embedding => write!(f, "embedding"),
            RunType::Prompt => write!(f, "prompt"),
            RunType::Parser => write!(f, "parser"),
            RunType::ChatModel => write!(f, "chat_model"),
        }
    }
}

impl From<&str> for RunType {
    fn from(s: &str) -> Self {
        match s {
            "tool" => RunType::Tool,
            "chain" => RunType::Chain,
            "llm" => RunType::Llm,
            "retriever" => RunType::Retriever,
            "embedding" => RunType::Embedding,
            "prompt" => RunType::Prompt,
            "parser" => RunType::Parser,
            "chat_model" => RunType::ChatModel,
            _ => RunType::Chain,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunEvent {
    pub name: String,
    pub time: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kwargs: Option<HashMap<String, Value>>,
}

impl RunEvent {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            time: Utc::now(),
            kwargs: None,
        }
    }

    pub fn with_time(name: impl Into<String>, time: DateTime<Utc>) -> Self {
        Self {
            name: name.into(),
            time,
            kwargs: None,
        }
    }

    pub fn with_kwargs(name: impl Into<String>, kwargs: HashMap<String, Value>) -> Self {
        Self {
            name: name.into(),
            time: Utc::now(),
            kwargs: Some(kwargs),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Run {
    pub id: Uuid,

    pub name: String,

    pub run_type: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_run_id: Option<Uuid>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<Uuid>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub dotted_order: Option<String>,

    pub start_time: DateTime<Utc>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time: Option<DateTime<Utc>>,

    pub inputs: HashMap<String, Value>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub outputs: Option<HashMap<String, Value>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,

    pub serialized: HashMap<String, Value>,

    #[serde(default)]
    pub extra: HashMap<String, Value>,

    #[serde(default)]
    pub events: Vec<RunEvent>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,

    #[serde(default)]
    pub child_runs: Vec<Run>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference_example_id: Option<Uuid>,
}

impl Run {
    pub fn new(
        id: Uuid,
        name: impl Into<String>,
        run_type: impl Into<String>,
        inputs: HashMap<String, Value>,
        serialized: HashMap<String, Value>,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            run_type: run_type.into(),
            parent_run_id: None,
            trace_id: None,
            dotted_order: None,
            start_time: Utc::now(),
            end_time: None,
            inputs,
            outputs: None,
            error: None,
            serialized,
            extra: HashMap::new(),
            events: vec![RunEvent::new("start")],
            tags: None,
            child_runs: Vec::new(),
            session_name: None,
            reference_example_id: None,
        }
    }

    pub fn copy(&self) -> Self {
        self.clone()
    }

    pub fn dict(&self, exclude: Option<&[&str]>) -> HashMap<String, Value> {
        let value = serde_json::to_value(self).unwrap_or_default();
        if let Value::Object(mut map) = value {
            if let Some(excluded_fields) = exclude {
                for field in excluded_fields {
                    map.remove(*field);
                }
            }
            map.into_iter().collect()
        } else {
            HashMap::new()
        }
    }

    pub fn set_end(&mut self) {
        self.end_time = Some(Utc::now());
    }

    pub fn set_error(&mut self, error: impl Into<String>) {
        self.error = Some(error.into());
        self.end_time = Some(Utc::now());
    }

    pub fn set_outputs(&mut self, outputs: HashMap<String, Value>) {
        self.outputs = Some(outputs);
    }

    pub fn add_child(&mut self, child: Run) {
        self.child_runs.push(child);
    }

    pub fn add_event(&mut self, event: RunEvent) {
        self.events.push(event);
    }

    pub fn add_tags(&mut self, tags: Vec<String>) {
        match &mut self.tags {
            Some(existing) => existing.extend(tags),
            None => self.tags = Some(tags),
        }
    }

    pub fn set_metadata(&mut self, metadata: HashMap<String, Value>) {
        self.extra.insert(
            "metadata".to_string(),
            serde_json::to_value(metadata).unwrap_or_default(),
        );
    }

    pub fn get_metadata(&self) -> Option<HashMap<String, Value>> {
        self.extra
            .get("metadata")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    pub fn is_ended(&self) -> bool {
        self.end_time.is_some()
    }

    pub fn has_error(&self) -> bool {
        self.error.is_some()
    }

    pub fn id_str(&self) -> String {
        self.id.to_string()
    }
}

impl Default for Run {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: String::new(),
            run_type: "chain".to_string(),
            parent_run_id: None,
            trace_id: None,
            dotted_order: None,
            start_time: Utc::now(),
            end_time: None,
            inputs: HashMap::new(),
            outputs: None,
            error: None,
            serialized: HashMap::new(),
            extra: HashMap::new(),
            events: Vec::new(),
            tags: None,
            child_runs: Vec::new(),
            session_name: None,
            reference_example_id: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run_new() {
        let run = Run::new(
            Uuid::new_v4(),
            "test_run",
            "chain",
            HashMap::new(),
            HashMap::new(),
        );

        assert_eq!(run.name, "test_run");
        assert_eq!(run.run_type, "chain");
        assert!(run.parent_run_id.is_none());
        assert!(run.end_time.is_none());
        assert!(!run.events.is_empty());
    }

    #[test]
    fn test_run_set_end() {
        let mut run = Run::new(
            Uuid::new_v4(),
            "test_run",
            "chain",
            HashMap::new(),
            HashMap::new(),
        );

        assert!(run.end_time.is_none());
        run.set_end();
        assert!(run.end_time.is_some());
        assert!(run.is_ended());
    }

    #[test]
    fn test_run_set_error() {
        let mut run = Run::new(
            Uuid::new_v4(),
            "test_run",
            "chain",
            HashMap::new(),
            HashMap::new(),
        );

        run.set_error("Something went wrong");
        assert!(run.has_error());
        assert_eq!(run.error, Some("Something went wrong".to_string()));
        assert!(run.is_ended());
    }

    #[test]
    fn test_run_add_tags() {
        let mut run = Run::default();
        assert!(run.tags.is_none());

        run.add_tags(vec!["tag1".to_string(), "tag2".to_string()]);
        assert_eq!(run.tags, Some(vec!["tag1".to_string(), "tag2".to_string()]));

        run.add_tags(vec!["tag3".to_string()]);
        assert_eq!(
            run.tags,
            Some(vec![
                "tag1".to_string(),
                "tag2".to_string(),
                "tag3".to_string()
            ])
        );
    }

    #[test]
    fn test_run_type_display() {
        assert_eq!(RunType::Chain.to_string(), "chain");
        assert_eq!(RunType::Llm.to_string(), "llm");
        assert_eq!(RunType::Tool.to_string(), "tool");
        assert_eq!(RunType::ChatModel.to_string(), "chat_model");
    }

    #[test]
    fn test_run_type_from_str() {
        assert_eq!(RunType::from("chain"), RunType::Chain);
        assert_eq!(RunType::from("llm"), RunType::Llm);
        assert_eq!(RunType::from("tool"), RunType::Tool);
        assert_eq!(RunType::from("unknown"), RunType::Chain);
    }

    #[test]
    fn test_run_event() {
        let event = RunEvent::new("test_event");
        assert_eq!(event.name, "test_event");
        assert!(event.kwargs.is_none());

        let mut kwargs = HashMap::new();
        kwargs.insert("key".to_string(), serde_json::json!("value"));
        let event_with_kwargs = RunEvent::with_kwargs("test", kwargs);
        assert!(event_with_kwargs.kwargs.is_some());
    }

    #[test]
    fn test_run_dict() {
        let run = Run::new(
            Uuid::new_v4(),
            "test",
            "chain",
            HashMap::new(),
            HashMap::new(),
        );

        let dict = run.dict(Some(&["child_runs"]));
        assert!(!dict.contains_key("child_runs"));
        assert!(dict.contains_key("name"));
    }
}
