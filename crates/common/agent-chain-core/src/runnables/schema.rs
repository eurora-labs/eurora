use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EventData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<Value>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<Value>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunk: Option<Value>,
}

impl EventData {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_input(mut self, input: Value) -> Self {
        self.input = Some(input);
        self
    }

    pub fn with_error(mut self, error: impl Into<String>) -> Self {
        self.error = Some(error.into());
        self
    }

    pub fn with_output(mut self, output: Value) -> Self {
        self.output = Some(output);
        self
    }

    pub fn with_chunk(mut self, chunk: Value) -> Self {
        self.chunk = Some(chunk);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseStreamEvent {
    pub event: String,

    pub run_id: String,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,

    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, Value>,

    #[serde(default)]
    pub parent_ids: Vec<String>,
}

impl BaseStreamEvent {
    pub fn new(event: impl Into<String>, run_id: impl Into<String>) -> Self {
        Self {
            event: event.into(),
            run_id: run_id.into(),
            tags: Vec::new(),
            metadata: HashMap::new(),
            parent_ids: Vec::new(),
        }
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    pub fn with_metadata(mut self, metadata: HashMap<String, Value>) -> Self {
        self.metadata = metadata;
        self
    }

    pub fn with_parent_ids(mut self, parent_ids: Vec<String>) -> Self {
        self.parent_ids = parent_ids;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StandardStreamEvent {
    #[serde(flatten)]
    pub base: BaseStreamEvent,

    pub data: EventData,

    pub name: String,
}

impl StandardStreamEvent {
    pub fn new(
        event: impl Into<String>,
        run_id: impl Into<String>,
        name: impl Into<String>,
    ) -> Self {
        Self {
            base: BaseStreamEvent::new(event, run_id),
            data: EventData::new(),
            name: name.into(),
        }
    }

    pub fn with_data(mut self, data: EventData) -> Self {
        self.data = data;
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.base.tags = tags;
        self
    }

    pub fn with_metadata(mut self, metadata: HashMap<String, Value>) -> Self {
        self.base.metadata = metadata;
        self
    }

    pub fn with_parent_ids(mut self, parent_ids: Vec<String>) -> Self {
        self.base.parent_ids = parent_ids;
        self
    }
}

pub const CUSTOM_EVENT_TYPE: &str = "on_custom_event";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomStreamEvent {
    #[serde(flatten)]
    pub base: BaseStreamEvent,

    pub name: String,

    pub data: Value,
}

impl CustomStreamEvent {
    pub fn new(run_id: impl Into<String>, name: impl Into<String>, data: Value) -> Self {
        Self {
            base: BaseStreamEvent::new(CUSTOM_EVENT_TYPE, run_id),
            name: name.into(),
            data,
        }
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.base.tags = tags;
        self
    }

    pub fn with_metadata(mut self, metadata: HashMap<String, Value>) -> Self {
        self.base.metadata = metadata;
        self
    }

    pub fn with_parent_ids(mut self, parent_ids: Vec<String>) -> Self {
        self.base.parent_ids = parent_ids;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum StreamEvent {
    Standard(StandardStreamEvent),
    Custom(CustomStreamEvent),
}

impl StreamEvent {
    pub fn event(&self) -> &str {
        match self {
            StreamEvent::Standard(e) => &e.base.event,
            StreamEvent::Custom(e) => &e.base.event,
        }
    }

    pub fn run_id(&self) -> &str {
        match self {
            StreamEvent::Standard(e) => &e.base.run_id,
            StreamEvent::Custom(e) => &e.base.run_id,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            StreamEvent::Standard(e) => &e.name,
            StreamEvent::Custom(e) => &e.name,
        }
    }

    pub fn tags(&self) -> &[String] {
        match self {
            StreamEvent::Standard(e) => &e.base.tags,
            StreamEvent::Custom(e) => &e.base.tags,
        }
    }

    pub fn metadata(&self) -> &HashMap<String, Value> {
        match self {
            StreamEvent::Standard(e) => &e.base.metadata,
            StreamEvent::Custom(e) => &e.base.metadata,
        }
    }

    pub fn parent_ids(&self) -> &[String] {
        match self {
            StreamEvent::Standard(e) => &e.base.parent_ids,
            StreamEvent::Custom(e) => &e.base.parent_ids,
        }
    }

    pub fn is_custom(&self) -> bool {
        matches!(self, StreamEvent::Custom(_))
    }

    pub fn is_standard(&self) -> bool {
        matches!(self, StreamEvent::Standard(_))
    }
}

impl From<StandardStreamEvent> for StreamEvent {
    fn from(event: StandardStreamEvent) -> Self {
        StreamEvent::Standard(event)
    }
}

impl From<CustomStreamEvent> for StreamEvent {
    fn from(event: CustomStreamEvent) -> Self {
        StreamEvent::Custom(event)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_data() {
        let data = EventData::new()
            .with_input(serde_json::json!("hello"))
            .with_output(serde_json::json!("world"));

        assert_eq!(data.input, Some(serde_json::json!("hello")));
        assert_eq!(data.output, Some(serde_json::json!("world")));
        assert!(data.error.is_none());
        assert!(data.chunk.is_none());
    }

    #[test]
    fn test_standard_stream_event() {
        let event = StandardStreamEvent::new("on_chain_start", "run-123", "my_chain")
            .with_tags(vec!["tag1".to_string()])
            .with_data(EventData::new().with_input(serde_json::json!({"key": "value"})));

        assert_eq!(event.base.event, "on_chain_start");
        assert_eq!(event.base.run_id, "run-123");
        assert_eq!(event.name, "my_chain");
        assert_eq!(event.base.tags, vec!["tag1"]);
        assert!(event.data.input.is_some());
    }

    #[test]
    fn test_custom_stream_event() {
        let event = CustomStreamEvent::new(
            "run-456",
            "my_custom_event",
            serde_json::json!({
                "custom_field": "custom_value"
            }),
        );

        assert_eq!(event.base.event, CUSTOM_EVENT_TYPE);
        assert_eq!(event.base.run_id, "run-456");
        assert_eq!(event.name, "my_custom_event");
        assert_eq!(
            event.data,
            serde_json::json!({"custom_field": "custom_value"})
        );
    }

    #[test]
    fn test_stream_event_enum() {
        let standard =
            StreamEvent::Standard(StandardStreamEvent::new("on_chain_end", "run-1", "chain"));
        let custom = StreamEvent::Custom(CustomStreamEvent::new(
            "run-2",
            "custom",
            serde_json::json!(null),
        ));

        assert!(standard.is_standard());
        assert!(!standard.is_custom());
        assert_eq!(standard.event(), "on_chain_end");
        assert_eq!(standard.name(), "chain");

        assert!(custom.is_custom());
        assert!(!custom.is_standard());
        assert_eq!(custom.event(), CUSTOM_EVENT_TYPE);
        assert_eq!(custom.name(), "custom");
    }

    #[test]
    fn test_stream_event_serialization() {
        let event = StandardStreamEvent::new("on_chain_start", "run-123", "test_chain")
            .with_data(EventData::new().with_input(serde_json::json!("input")));

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("on_chain_start"));
        assert!(json.contains("run-123"));
        assert!(json.contains("test_chain"));

        let deserialized: StandardStreamEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.base.event, "on_chain_start");
        assert_eq!(deserialized.base.run_id, "run-123");
        assert_eq!(deserialized.name, "test_chain");
    }
}
