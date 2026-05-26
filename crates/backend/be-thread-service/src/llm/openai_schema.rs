//! Normalize schemars-emitted JSON Schema into the dialect that
//! OpenAI Chat Completions and its compatible providers accept.
//!
//! Schemars targets JSON Schema 2020-12; OpenAI's function-call schema
//! accepts a working subset of that. Provider implementations of the
//! same API vary in strictness:
//!
//! - OpenAI tolerates `$schema`, `title`, nullable-type arrays, and
//!   `$defs`/`$ref`.
//! - GLM-family providers reject `$schema`/`title` at the parameters
//!   root, refuse to resolve `$ref`, want `"type": "string"` instead
//!   of `["string", "null"]`, and don't recognise schemars-specific
//!   `format` keywords (`uint32`, `uint64`, …).
//!
//! This module strips the framework's schema down to the common subset
//! every provider in this family accepts. It is the only place in the
//! backend that knows about provider-specific schema quirks; the
//! `#[adapter]` macro and the tool descriptors stay framework-agnostic.

use serde_json::{Map, Value};

/// Upper bound on `$ref` resolution recursion. Self-referential `$defs`
/// (e.g. a tree node whose `children` items `$ref` back to itself) would
/// otherwise expand without bound; stopping at 8 levels leaves any
/// remaining `$ref` strings in place, which the LLM can still see well
/// enough to call.
const MAX_REF_DEPTH: usize = 8;

/// Normalize a JSON Schema in place into the OpenAI-compat dialect.
///
/// Idempotent: applying `normalize` to an already-normalized schema is a
/// no-op.
pub fn normalize(schema: &mut Value) {
    inline_refs(schema);
    walk(schema, &mut normalize_node);
    ensure_required_present(schema);
}

/// Backfill `required: []` at the schema root when schemars omitted it.
///
/// schemars elides the `required` array entirely from object schemas
/// whose every field is optional. Most strict tool-calling providers
/// (GLM-family especially) treat `required` as a load-bearing field of
/// the function-parameter contract: when it's missing, the tool is
/// silently dropped from the model's callable set. The model can
/// reference it in its reasoning (`Let me get the readability article`)
/// but the structured `tool_call` emission never fires. Backfill the
/// empty array so the schema matches the shape the
/// `agent-chain-macros::tool` macro produces for working tools.
fn ensure_required_present(schema: &mut Value) {
    let Some(obj) = schema.as_object_mut() else {
        return;
    };
    if obj.get("type") != Some(&Value::String("object".to_string())) {
        return;
    }
    obj.entry("required".to_string())
        .or_insert_with(|| Value::Array(Vec::new()));
}

fn walk<F: FnMut(&mut Value)>(node: &mut Value, f: &mut F) {
    f(node);
    match node {
        Value::Object(obj) => {
            for child in obj.values_mut() {
                walk(child, f);
            }
        }
        Value::Array(arr) => {
            for child in arr.iter_mut() {
                walk(child, f);
            }
        }
        _ => {}
    }
}

fn normalize_node(node: &mut Value) {
    let Some(obj) = node.as_object_mut() else {
        return;
    };
    obj.remove("$schema");
    obj.remove("title");
    // Strip every `description` field inside parameters. The tool's
    // top-level function description (set on the `ToolDefinition`, not
    // inside `parameters`) is what the model uses to decide whether
    // to call. Parameter-level descriptions get in the way: the
    // `agent-chain-macros::tool` macro (the path firecrawl tools take,
    // which work reliably end-to-end) emits no parameter-level
    // descriptions at all — schemas are pure structure. Mimicking
    // that minimal shape removes a class of subtle model-decision
    // differences between adapter-emitted and macro-emitted tools.
    obj.remove("description");
    collapse_nullable_type(obj);
    drop_integer_format(obj);
    collapse_anyof_with_null(obj);
}

/// `"type": ["string", "null"]` is how schemars renders `Option<String>`.
/// Strict providers want a scalar `"type"`; the field's optionality is
/// already conveyed by its absence from `required`.
fn collapse_nullable_type(obj: &mut Map<String, Value>) {
    let Some(Value::Array(arr)) = obj.get("type") else {
        return;
    };
    if arr.len() != 2 {
        return;
    }
    let has_null = arr.iter().any(|v| v.as_str() == Some("null"));
    if !has_null {
        return;
    }
    let Some(non_null) = arr.iter().find(|v| v.as_str() != Some("null")).cloned() else {
        return;
    };
    obj.insert("type".to_string(), non_null);
}

/// Schemars emits Rust-specific width hints (`uint32`, `int64`, …) in
/// the `format` slot. OpenAI's documented format list doesn't include
/// them and GLM rejects unrecognized hints outright.
fn drop_integer_format(obj: &mut Map<String, Value>) {
    let Some(Value::String(format)) = obj.get("format") else {
        return;
    };
    if matches!(
        format.as_str(),
        "uint"
            | "uint8"
            | "uint16"
            | "uint32"
            | "uint64"
            | "uint128"
            | "int"
            | "int8"
            | "int16"
            | "int32"
            | "int64"
            | "int128",
    ) {
        obj.remove("format");
    }
}

/// `anyOf: [<schema>, {"type": "null"}]` is schemars's rendering of
/// `Option<NamedStruct>`. Drop the null branch and fold the remaining
/// schema's keys into the parent so the result is a plain object schema
/// rather than a two-branch union.
fn collapse_anyof_with_null(obj: &mut Map<String, Value>) {
    let Some(Value::Array(arr)) = obj.get("anyOf") else {
        return;
    };
    if arr.len() != 2 {
        return;
    }
    let Some(null_idx) = arr.iter().position(is_null_only) else {
        return;
    };
    let other_idx = 1 - null_idx;
    let Some(Value::Object(other)) = arr.get(other_idx).cloned() else {
        return;
    };
    obj.remove("anyOf");
    for (k, v) in other {
        obj.entry(k).or_insert(v);
    }
}

fn is_null_only(v: &Value) -> bool {
    let Some(obj) = v.as_object() else {
        return false;
    };
    obj.len() == 1 && obj.get("type") == Some(&Value::String("null".to_string()))
}

/// Inline every `$ref: "#/$defs/Foo"` against the root `$defs` block,
/// then drop `$defs`. Sibling keys on the ref object (such as
/// `description`) win over the resolved schema's matching keys so
/// per-field documentation isn't lost.
fn inline_refs(root: &mut Value) {
    let defs = root
        .as_object()
        .and_then(|o| o.get("$defs"))
        .cloned()
        .unwrap_or(Value::Null);
    if defs.is_object() {
        inline_refs_in(root, &defs, 0);
    }
    if let Some(obj) = root.as_object_mut() {
        obj.remove("$defs");
    }
}

fn inline_refs_in(node: &mut Value, defs: &Value, depth: usize) {
    if depth > MAX_REF_DEPTH {
        return;
    }
    match node {
        Value::Object(obj) => {
            if let Some(target_name) = obj
                .get("$ref")
                .and_then(Value::as_str)
                .and_then(|r| r.strip_prefix("#/$defs/"))
                .map(str::to_owned)
                && let Some(target) = defs.get(&target_name).cloned()
            {
                let preserved: Vec<(String, Value)> = obj
                    .iter()
                    .filter(|(k, _)| k.as_str() != "$ref")
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();
                *node = target;
                if let Some(node_obj) = node.as_object_mut() {
                    for (k, v) in preserved {
                        node_obj.insert(k, v);
                    }
                }
                inline_refs_in(node, defs, depth + 1);
                return;
            }
            for child in obj.values_mut() {
                inline_refs_in(child, defs, depth);
            }
        }
        Value::Array(arr) => {
            for child in arr.iter_mut() {
                inline_refs_in(child, defs, depth);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn normalized(mut v: Value) -> Value {
        normalize(&mut v);
        v
    }

    #[test]
    fn strips_schema_and_title_at_root() {
        let out = normalized(json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "title": "GetAccessibilityTreeArgs",
            "type": "object",
        }));
        assert_eq!(out, json!({ "type": "object" }));
    }

    #[test]
    fn strips_schema_and_title_in_nested_objects() {
        let out = normalized(json!({
            "type": "object",
            "properties": {
                "child": {
                    "$schema": "x",
                    "title": "ChildType",
                    "type": "string",
                }
            }
        }));
        assert_eq!(
            out,
            json!({
                "type": "object",
                "properties": {
                    "child": { "type": "string" }
                }
            })
        );
    }

    #[test]
    fn collapses_nullable_string_type() {
        let out = normalized(json!({
            "type": "object",
            "properties": {
                "name": { "type": ["string", "null"] }
            }
        }));
        assert_eq!(
            out,
            json!({
                "type": "object",
                "properties": { "name": { "type": "string" } }
            })
        );
    }

    #[test]
    fn collapses_nullable_integer_type() {
        let out = normalized(json!({
            "type": "object",
            "properties": {
                "max_nodes": {
                    "type": ["integer", "null"],
                    "format": "uint32",
                    "minimum": 0,
                }
            }
        }));
        assert_eq!(
            out,
            json!({
                "type": "object",
                "properties": {
                    "max_nodes": { "type": "integer", "minimum": 0 }
                }
            })
        );
    }

    #[test]
    fn drops_uint32_format_keyword() {
        let out = normalized(json!({
            "type": "integer",
            "format": "uint32",
            "minimum": 0,
        }));
        assert_eq!(out, json!({ "type": "integer", "minimum": 0 }));
    }

    #[test]
    fn drops_int64_format_keyword() {
        let out = normalized(json!({ "type": "integer", "format": "int64" }));
        assert_eq!(out, json!({ "type": "integer" }));
    }

    #[test]
    fn preserves_double_format_keyword() {
        // `double`/`float` aren't in the OpenAI list either, but
        // schemars uses them on `f32`/`f64`; many providers tolerate
        // unknown `number` formats. Leave them alone — only the
        // explicitly-rejected integer-width hints are dropped.
        let out = normalized(json!({ "type": "number", "format": "double" }));
        assert_eq!(out, json!({ "type": "number", "format": "double" }));
    }

    #[test]
    fn inlines_a_ref_against_defs() {
        let out = normalized(json!({
            "$defs": {
                "Kind": {
                    "oneOf": [
                        { "const": "text", "type": "string" },
                        { "const": "html", "type": "string" }
                    ]
                }
            },
            "type": "object",
            "properties": {
                "kind": { "$ref": "#/$defs/Kind" }
            }
        }));
        assert_eq!(
            out,
            json!({
                "type": "object",
                "properties": {
                    "kind": {
                        "oneOf": [
                            { "const": "text", "type": "string" },
                            { "const": "html", "type": "string" }
                        ]
                    }
                }
            })
        );
    }

    #[test]
    fn preserves_sibling_description_when_inlining_ref() {
        let out = normalized(json!({
            "$defs": {
                "Kind": { "type": "string" }
            },
            "type": "object",
            "properties": {
                "kind": {
                    "$ref": "#/$defs/Kind",
                    "description": "per-field docs",
                }
            }
        }));
        assert_eq!(
            out,
            json!({
                "type": "object",
                "properties": {
                    "kind": {
                        "type": "string",
                        "description": "per-field docs",
                    }
                }
            })
        );
    }

    #[test]
    fn collapses_anyof_with_null_after_ref_inlining() {
        let out = normalized(json!({
            "$defs": {
                "BoundingBox": {
                    "type": "object",
                    "properties": { "x": { "type": "number" } },
                    "required": ["x"]
                }
            },
            "type": "object",
            "properties": {
                "bounds": {
                    "anyOf": [
                        { "$ref": "#/$defs/BoundingBox" },
                        { "type": "null" }
                    ],
                    "description": "Optional bounds.",
                }
            }
        }));
        assert_eq!(
            out,
            json!({
                "type": "object",
                "properties": {
                    "bounds": {
                        "description": "Optional bounds.",
                        "type": "object",
                        "properties": { "x": { "type": "number" } },
                        "required": ["x"]
                    }
                }
            })
        );
    }

    #[test]
    fn tolerates_self_referential_defs_via_depth_cap() {
        // A self-referential `$defs` would expand indefinitely; the
        // depth cap leaves the deepest `$ref` in place rather than
        // panicking or running out of memory.
        let mut value = json!({
            "$defs": {
                "AxNode": {
                    "type": "object",
                    "properties": {
                        "children": {
                            "type": "array",
                            "items": { "$ref": "#/$defs/AxNode" }
                        }
                    }
                }
            },
            "$ref": "#/$defs/AxNode"
        });
        normalize(&mut value);
        assert!(value.is_object());
    }

    #[test]
    fn normalize_is_idempotent() {
        let raw = json!({
            "$schema": "x",
            "title": "T",
            "type": "object",
            "properties": {
                "x": { "type": ["integer", "null"], "format": "uint32" }
            }
        });
        let once = normalized(raw.clone());
        let twice = normalized(once.clone());
        assert_eq!(once, twice);
    }

    #[test]
    fn empty_struct_with_optional_note_keeps_properties_block() {
        // Equivalent of `schema_for!(eurora_tools::Empty)` after the
        // Part 1 redesign.
        let out = normalized(json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "title": "Empty",
            "type": "object",
            "properties": {
                "note": { "type": ["string", "null"] }
            }
        }));
        assert_eq!(
            out,
            json!({
                "type": "object",
                "properties": { "note": { "type": "string" } }
            })
        );
        assert!(
            out.get("properties").is_some(),
            "Empty must still expose `properties` after normalization: {out}"
        );
    }

    #[test]
    fn fixture_get_accessibility_tree_args() {
        // Snapshot of the real `GetAccessibilityTreeArgs` input schema
        // observed failing in production. Asserts the post-normalize
        // shape matches the documented dialect.
        let out = normalized(json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "description": "Arguments to `get_accessibility_tree`.",
            "properties": {
                "max_depth": {
                    "description": "Maximum traversal depth.",
                    "format": "uint32",
                    "minimum": 0,
                    "type": ["integer", "null"]
                },
                "max_nodes": {
                    "description": "Soft cap on emitted nodes.",
                    "format": "uint32",
                    "minimum": 0,
                    "type": ["integer", "null"]
                },
                "root_selector": {
                    "description": "CSS selector for the subtree root.",
                    "type": ["string", "null"]
                }
            },
            "title": "GetAccessibilityTreeArgs",
            "type": "object"
        }));
        assert_eq!(
            out,
            json!({
                "description": "Arguments to `get_accessibility_tree`.",
                "properties": {
                    "max_depth": {
                        "description": "Maximum traversal depth.",
                        "minimum": 0,
                        "type": "integer"
                    },
                    "max_nodes": {
                        "description": "Soft cap on emitted nodes.",
                        "minimum": 0,
                        "type": "integer"
                    },
                    "root_selector": {
                        "description": "CSS selector for the subtree root.",
                        "type": "string"
                    }
                },
                "type": "object"
            })
        );
    }

    #[test]
    fn fixture_query_selector_args_inlines_include_enum() {
        // `QuerySelectorArgs` references `QuerySelectorInclude` via
        // `$defs`. Normalization should inline the enum and drop the
        // `$defs` block.
        let out = normalized(json!({
            "$defs": {
                "QuerySelectorInclude": {
                    "oneOf": [
                        { "const": "text", "type": "string" },
                        { "const": "html", "type": "string" },
                        { "const": "attributes", "type": "string" },
                        { "const": "bounds", "type": "string" }
                    ]
                }
            },
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "description": "Arguments to `query_selector`.",
            "properties": {
                "include": {
                    "default": [],
                    "description": "Per-match facets to populate.",
                    "items": { "$ref": "#/$defs/QuerySelectorInclude" },
                    "type": "array"
                },
                "limit": {
                    "default": 50,
                    "description": "Maximum matches to return.",
                    "format": "uint32",
                    "minimum": 0,
                    "type": "integer"
                },
                "selector": {
                    "description": "CSS selector.",
                    "type": "string"
                }
            },
            "required": ["selector"],
            "title": "QuerySelectorArgs",
            "type": "object"
        }));
        assert!(out.get("$defs").is_none(), "$defs should be dropped");
        let include_items = out
            .pointer("/properties/include/items")
            .expect("inlined items present");
        assert_eq!(
            include_items,
            &json!({
                "oneOf": [
                    { "const": "text", "type": "string" },
                    { "const": "html", "type": "string" },
                    { "const": "attributes", "type": "string" },
                    { "const": "bounds", "type": "string" }
                ]
            })
        );
        let limit = out.pointer("/properties/limit").expect("limit present");
        assert!(
            limit.get("format").is_none(),
            "uint32 format should be dropped from limit: {limit}"
        );
    }
}
