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

#[bon::bon]
impl EventData {
    #[builder]
    pub fn new(
        input: Option<Value>,
        #[builder(into)] error: Option<String>,
        output: Option<Value>,
        chunk: Option<Value>,
    ) -> Self {
        Self {
            input,
            error,
            output,
            chunk,
        }
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

#[bon::bon]
impl BaseStreamEvent {
    #[builder]
    pub fn new(
        #[builder(into)] event: String,
        #[builder(into)] run_id: String,
        #[builder(default)] tags: Vec<String>,
        #[builder(default)] metadata: HashMap<String, Value>,
        #[builder(default)] parent_ids: Vec<String>,
    ) -> Self {
        Self {
            event,
            run_id,
            tags,
            metadata,
            parent_ids,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StandardStreamEvent {
    #[serde(flatten)]
    pub base: BaseStreamEvent,

    pub data: EventData,

    pub name: String,
}

#[bon::bon]
impl StandardStreamEvent {
    #[builder]
    pub fn new(
        #[builder(into)] event: String,
        #[builder(into)] run_id: String,
        #[builder(into)] name: String,
        #[builder(default)] data: EventData,
        #[builder(default)] tags: Vec<String>,
        #[builder(default)] metadata: HashMap<String, Value>,
        #[builder(default)] parent_ids: Vec<String>,
    ) -> Self {
        Self {
            base: BaseStreamEvent::builder()
                .event(event)
                .run_id(run_id)
                .tags(tags)
                .metadata(metadata)
                .parent_ids(parent_ids)
                .build(),
            data,
            name,
        }
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

#[bon::bon]
impl CustomStreamEvent {
    #[builder]
    pub fn new(
        #[builder(into)] run_id: String,
        #[builder(into)] name: String,
        data: Value,
        #[builder(default)] tags: Vec<String>,
        #[builder(default)] metadata: HashMap<String, Value>,
        #[builder(default)] parent_ids: Vec<String>,
    ) -> Self {
        Self {
            base: BaseStreamEvent::builder()
                .event(CUSTOM_EVENT_TYPE)
                .run_id(run_id)
                .tags(tags)
                .metadata(metadata)
                .parent_ids(parent_ids)
                .build(),
            name,
            data,
        }
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
        let data = EventData::builder()
            .input(serde_json::json!("hello"))
            .output(serde_json::json!("world"))
            .build();

        assert_eq!(data.input, Some(serde_json::json!("hello")));
        assert_eq!(data.output, Some(serde_json::json!("world")));
        assert!(data.error.is_none());
        assert!(data.chunk.is_none());
    }

    #[test]
    fn test_standard_stream_event() {
        let event = StandardStreamEvent::builder()
            .event("on_chain_start")
            .run_id("run-123")
            .name("my_chain")
            .tags(vec!["tag1".to_string()])
            .data(
                EventData::builder()
                    .input(serde_json::json!({"key": "value"}))
                    .build(),
            )
            .build();

        assert_eq!(event.base.event, "on_chain_start");
        assert_eq!(event.base.run_id, "run-123");
        assert_eq!(event.name, "my_chain");
        assert_eq!(event.base.tags, vec!["tag1"]);
        assert!(event.data.input.is_some());
    }

    #[test]
    fn test_custom_stream_event() {
        let event = CustomStreamEvent::builder()
            .run_id("run-456")
            .name("my_custom_event")
            .data(serde_json::json!({
                "custom_field": "custom_value"
            }))
            .build();

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
        let standard = StreamEvent::Standard(
            StandardStreamEvent::builder()
                .event("on_chain_end")
                .run_id("run-1")
                .name("chain")
                .build(),
        );
        let custom = StreamEvent::Custom(
            CustomStreamEvent::builder()
                .run_id("run-2")
                .name("custom")
                .data(serde_json::json!(null))
                .build(),
        );

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
        let event = StandardStreamEvent::builder()
            .event("on_chain_start")
            .run_id("run-123")
            .name("test_chain")
            .data(
                EventData::builder()
                    .input(serde_json::json!("input"))
                    .build(),
            )
            .build();

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
