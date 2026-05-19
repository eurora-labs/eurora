//! Wire-shape helpers shared across adapter argument lists.
//!
//! Adapter trait methods take a concrete `args:` parameter so the
//! macro-generated dispatcher can decode the call's JSON arguments into
//! it. Tools that conceptually take "no input beyond the target" still
//! need *some* type to decode the wire `{}` into; [`Empty`] is the
//! canonical one. Centralising it here means every empty-args tool emits
//! the same input schema and the LLM sees one stable shape.
//!
//! [`Empty`] is declared as `struct Empty {}` with explicit braces.
//! `struct Empty;` (a unit struct) would serialize to `null` instead of
//! `{}`, breaking the dispatcher's decode contract.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// No arguments.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Empty {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_serializes_to_an_object() {
        let value = serde_json::to_value(Empty::default()).expect("serialize");
        assert_eq!(value, serde_json::json!({}));
    }

    #[test]
    fn empty_decodes_from_an_empty_object() {
        let value: Empty = serde_json::from_value(serde_json::json!({})).expect("deserialize");
        assert_eq!(value, Empty::default());
    }
}
