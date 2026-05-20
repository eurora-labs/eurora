//! Wire-shape helpers shared across adapter argument lists.
//!
//! Adapter trait methods take a concrete `args:` parameter so the
//! macro-generated dispatcher can decode the call's JSON arguments into
//! it. Tools that conceptually take "no input beyond the target" still
//! need *some* type to decode the wire `{}` into; [`Empty`] is the
//! canonical one. Centralising it here means every empty-args tool emits
//! the same input schema and the LLM sees one stable shape.
//!
//! # Why `Empty` carries a required `task` field
//!
//! Schemars would render a zero-field struct as
//! `{ "type": "object", "properties": {} }`. That's valid JSON Schema,
//! but OpenAI-compatible providers in the GLM family refuse to emit a
//! structured `tool_calls` payload unless the function has at least one
//! **required** parameter for the model to populate.
//!
//! The field name is `task` rather than something generic like
//! `reason` because the model uses field names as hints for what to
//! put in them. `task` invites the model to restate the user's
//! intent in a short string (e.g. `"transcribe the page"`), giving it
//! a concrete value to commit on emit. Every dispatcher ignores the
//! value; only its presence in the wire schema matters.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// No-input args carrier. Carries a required `task` field that every
/// dispatcher ignores; see the module-level docs for why.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Empty {
    pub task: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_default_serializes_to_an_object_with_empty_task() {
        let value = serde_json::to_value(Empty::default()).expect("serialize");
        assert_eq!(value, serde_json::json!({ "task": "" }));
    }

    #[test]
    fn empty_rejects_missing_task() {
        let result: Result<Empty, _> = serde_json::from_value(serde_json::json!({}));
        assert!(
            result.is_err(),
            "missing required `task` field should fail to deserialize"
        );
    }

    #[test]
    fn empty_decodes_with_task_present() {
        let value: Empty = serde_json::from_value(serde_json::json!({ "task": "transcribe page" }))
            .expect("deserialize");
        assert_eq!(value.task, "transcribe page");
    }

    #[test]
    fn empty_schema_marks_task_as_required() {
        let schema = schemars::schema_for!(Empty);
        let value = serde_json::to_value(&schema).expect("schema serializes");
        let properties = value
            .get("properties")
            .and_then(|v| v.as_object())
            .expect("schema has a properties block");
        assert!(
            properties.contains_key("task"),
            "expected `task` property in {value}"
        );
        let required = value
            .get("required")
            .and_then(|v| v.as_array())
            .expect("schema has a required array");
        assert!(
            required.iter().any(|v| v.as_str() == Some("task")),
            "expected `task` listed in required in {value}"
        );
    }
}
