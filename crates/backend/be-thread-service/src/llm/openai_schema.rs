//! Normalize JSON Schema input from any tool source into the dialect that
//! OpenAI Chat Completions and its compatible providers accept.
//!
//! Two input shapes flow through here:
//!
//! - **Schemars** (Rust-side `#[tool]` macro, e.g. Firecrawl) targets
//!   JSON Schema 2020-12: emits `$defs`/`$ref`, schemars-specific
//!   `format` keywords (`uint32`, `uint64`, …), and `"type": [..., "null"]`
//!   for `Option<T>`.
//! - **zod-to-json-schema** (TS-side browser tools, sent over the wire as
//!   `WireToolDescriptor.parameters`) targets draft-07 by default: emits
//!   `definitions`/`$ref` (note the spelling), `additionalProperties: false`
//!   from `.strict()`, and omits `required` entirely when no fields are
//!   required.
//!
//! Provider implementations of the OpenAI-compatible API vary in strictness:
//!
//! - OpenAI tolerates `$schema`, `title`, nullable-type arrays, and
//!   `$defs`/`$ref`.
//! - GLM-family providers reject `$schema`/`title` at the parameters
//!   root, refuse to resolve `$ref`, want `"type": "string"` instead
//!   of `["string", "null"]`, and don't recognise schemars-specific
//!   `format` keywords.
//! - GLM hosted on strict validators (SGLang/vLLM/NVIDIA NIM) further
//!   rejects object schemas without an explicit `required` array, even
//!   when the array would be empty.
//!
//! This module strips both input dialects down to the common subset
//! every provider in this family accepts. It is the only place in the
//! backend that knows about provider-specific schema quirks; the
//! `#[adapter]` macro and the tool descriptors stay framework-agnostic.

use serde_json::{Map, Value};

/// Upper bound on `$ref` resolution recursion. Self-referential def
/// blocks (e.g. a tree node whose `children` items `$ref` back to itself)
/// would otherwise expand without bound; stopping at 8 levels leaves any
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
    collapse_nullable_type(obj);
    drop_integer_format(obj);
    collapse_anyof_with_null(obj);
    strip_useless_additional_properties(obj);
    ensure_object_required_array(obj);
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

/// `additionalProperties: false` on an object schema that has no declared
/// `properties` describes an object that cannot have any keys — which is
/// indistinguishable from a freeform object as far as the model can tell,
/// but trips strict validators (SGLang/vLLM/NVIDIA NIM) that interpret it
/// literally and then reject any non-empty object. Stripping the
/// constraint on `properties`-less schemas preserves the model-visible
/// intent ("an object goes here") without the validator footgun.
fn strip_useless_additional_properties(obj: &mut Map<String, Value>) {
    if obj.get("type") != Some(&Value::String("object".to_string())) {
        return;
    }
    if obj.contains_key("properties") {
        return;
    }
    if obj.get("additionalProperties") == Some(&Value::Bool(false)) {
        obj.remove("additionalProperties");
    }
}

/// Strict GLM-family validators (SGLang/vLLM/NVIDIA NIM) reject object
/// schemas that omit the `required` array entirely, even when no fields
/// are required. zod-to-json-schema omits it on objects without required
/// fields; this restores the explicit empty array so the schema parses
/// everywhere. No-op when `required` is already present (whatever its
/// shape) — only the absence case fires.
fn ensure_object_required_array(obj: &mut Map<String, Value>) {
    if obj.get("type") != Some(&Value::String("object".to_string())) {
        return;
    }
    if !obj.contains_key("required") {
        obj.insert("required".to_string(), Value::Array(Vec::new()));
    }
}

/// Inline every `$ref` against the root def block, then drop the def
/// container. Sibling keys on the ref object (such as `description`)
/// win over the resolved schema's matching keys so per-field
/// documentation isn't lost.
///
/// Two def container shapes are supported because the two schema
/// generators we feed in target different drafts:
///
/// - **`$defs`**: schemars and JSON Schema 2020-12 — `$ref` looks like
///   `#/$defs/Foo`.
/// - **`definitions`**: zod-to-json-schema and draft-07 — `$ref` looks
///   like `#/definitions/Foo`.
///
/// Both are read, merged into one lookup table (with `$defs` winning on
/// name collision since it's the modern form), and both container keys
/// are dropped from the output.
fn inline_refs(root: &mut Value) {
    let defs = collect_def_blocks(root);
    if defs.is_object() {
        inline_refs_in(root, &defs, 0);
    }
    if let Some(obj) = root.as_object_mut() {
        obj.remove("$defs");
        obj.remove("definitions");
    }
}

fn collect_def_blocks(root: &Value) -> Value {
    let Some(root_obj) = root.as_object() else {
        return Value::Null;
    };
    let definitions = root_obj.get("definitions").and_then(Value::as_object);
    let dollar_defs = root_obj.get("$defs").and_then(Value::as_object);
    match (definitions, dollar_defs) {
        (None, None) => Value::Null,
        (Some(d), None) => Value::Object(d.clone()),
        (None, Some(d)) => Value::Object(d.clone()),
        (Some(legacy), Some(modern)) => {
            let mut merged = legacy.clone();
            for (k, v) in modern {
                merged.insert(k.clone(), v.clone());
            }
            Value::Object(merged)
        }
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
                .and_then(strip_supported_ref_prefix)
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

fn strip_supported_ref_prefix(reference: &str) -> Option<&str> {
    reference
        .strip_prefix("#/$defs/")
        .or_else(|| reference.strip_prefix("#/definitions/"))
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
        assert_eq!(out, json!({ "type": "object", "required": [] }));
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
                "required": [],
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
                "required": [],
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
                "required": [],
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
                "required": [],
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
    fn inlines_a_ref_against_definitions_block() {
        // Draft-07 emitters (zod-to-json-schema) use `definitions` rather
        // than `$defs`; the inliner must resolve both spellings against
        // the same lookup table.
        let out = normalized(json!({
            "definitions": {
                "Kind": {
                    "oneOf": [
                        { "const": "text", "type": "string" },
                        { "const": "html", "type": "string" }
                    ]
                }
            },
            "type": "object",
            "properties": {
                "kind": { "$ref": "#/definitions/Kind" }
            }
        }));
        assert_eq!(
            out,
            json!({
                "type": "object",
                "required": [],
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
    fn merges_defs_and_definitions_with_defs_winning() {
        // Pathological input that carries both blocks. `$defs` is the
        // modern spelling so its entry takes precedence on name
        // collision; the resolved schema reflects that.
        let out = normalized(json!({
            "definitions": {
                "Kind": { "type": "string", "description": "legacy" }
            },
            "$defs": {
                "Kind": { "type": "string", "description": "modern" }
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
                "required": [],
                "properties": {
                    "kind": { "type": "string", "description": "modern" }
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
                "required": [],
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
                "required": [],
                "properties": {
                    "bounds": {
                        "description": "Optional bounds.",
                        "type": "object",
                        "required": ["x"],
                        "properties": { "x": { "type": "number" } }
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
                "required": [],
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
                "type": "object",
                "required": [],
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
                }
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
        assert!(
            out.get("definitions").is_none(),
            "definitions should also be dropped"
        );
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

    #[test]
    fn ensure_required_array_added_to_object_without_required() {
        // Strict GLM-family validators reject object schemas that omit
        // `required` entirely — even when no fields are required.
        // zod-to-json-schema emits this shape from `z.object({ ... }).strict()`
        // when no field is marked `.optional()` mandatory.
        let out = normalized(json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" }
            }
        }));
        assert_eq!(
            out,
            json!({
                "type": "object",
                "required": [],
                "properties": { "name": { "type": "string" } }
            })
        );
    }

    #[test]
    fn ensure_required_array_preserves_existing_non_empty_required() {
        let out = normalized(json!({
            "type": "object",
            "required": ["selector"],
            "properties": {
                "selector": { "type": "string" }
            }
        }));
        let required = out
            .pointer("/required")
            .and_then(Value::as_array)
            .expect("required preserved");
        assert_eq!(required.len(), 1);
        assert_eq!(required[0], json!("selector"));
    }

    #[test]
    fn ensure_required_array_only_touches_object_schemas() {
        // Non-object schemas (string, integer, array, ...) must not gain a
        // bogus `required` field. The walker visits every node, so the
        // type-check inside the rule is what enforces this.
        let out = normalized(json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" },
                "ages": {
                    "type": "array",
                    "items": { "type": "integer" }
                }
            }
        }));
        assert!(
            out.pointer("/properties/name/required").is_none(),
            "string schemas must not gain `required`: {out}"
        );
        assert!(
            out.pointer("/properties/ages/required").is_none(),
            "array schemas must not gain `required`: {out}"
        );
        assert!(
            out.pointer("/properties/ages/items/required").is_none(),
            "integer item schemas must not gain `required`: {out}"
        );
    }

    #[test]
    fn strips_additional_properties_false_on_props_less_object() {
        // An object schema with `additionalProperties: false` and no
        // declared `properties` describes "an object that cannot have any
        // keys" — a strict-validator footgun the model can't act on
        // usefully. Drop the constraint so the schema reads as a freeform
        // object.
        let out = normalized(json!({
            "type": "object",
            "additionalProperties": false
        }));
        assert_eq!(out, json!({ "type": "object", "required": [] }));
    }

    #[test]
    fn preserves_additional_properties_false_on_object_with_props() {
        // The normal case — `.strict()` on a zod object with declared
        // fields produces `additionalProperties: false` alongside
        // `properties`. That combination is meaningful and must be
        // preserved.
        let out = normalized(json!({
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "name": { "type": "string" }
            }
        }));
        assert_eq!(
            out,
            json!({
                "type": "object",
                "additionalProperties": false,
                "required": [],
                "properties": { "name": { "type": "string" } }
            })
        );
    }

    #[test]
    fn preserves_additional_properties_schema_value() {
        // `additionalProperties: <schema>` (not `false`) is a third
        // meaningful shape: any extra keys must conform to the inner
        // schema. The strip rule must only fire for the exact
        // `additionalProperties: false` + no-properties combination.
        // The inner schema is a string (not an object), so the
        // `required: []` rule must not graft onto it.
        let out = normalized(json!({
            "type": "object",
            "additionalProperties": { "type": "string" }
        }));
        assert_eq!(
            out,
            json!({
                "type": "object",
                "additionalProperties": { "type": "string" },
                "required": []
            })
        );
    }

    /// `zod-to-json-schema` rendering of `z.object({ root_selector: ...,
    /// max_depth: ..., max_nodes: ... }).strict()`. None of the fields are
    /// `.optional()` so zod omits `required` entirely; `.strict()` adds
    /// `additionalProperties: false`. The browser `web_get_accessibility_tree`
    /// tool ships this exact shape; before normalization GLM-family
    /// providers either drop the call silently or reject the schema. After,
    /// the schema matches the dialect every provider in this family
    /// accepts.
    #[test]
    fn fixture_zod_strict_args_without_required_fields() {
        let out = normalized(json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "root_selector": { "type": "string", "minLength": 1 },
                "max_depth": { "type": "integer", "exclusiveMinimum": 0 },
                "max_nodes": { "type": "integer", "exclusiveMinimum": 0 }
            }
        }));
        assert_eq!(
            out,
            json!({
                "type": "object",
                "additionalProperties": false,
                "required": [],
                "properties": {
                    "root_selector": { "type": "string", "minLength": 1 },
                    "max_depth": { "type": "integer", "exclusiveMinimum": 0 },
                    "max_nodes": { "type": "integer", "exclusiveMinimum": 0 }
                }
            })
        );
    }

    /// `zod-to-json-schema` rendering of `z.object({ selector, limit,
    /// include }).strict()` with `selector` required and the rest
    /// optional. `include` references an enum via `#/definitions/...` —
    /// the draft-07 spelling — which the inliner must resolve alongside
    /// the modern `#/$defs/...` form.
    #[test]
    fn fixture_zod_strict_args_with_definitions_ref() {
        let out = normalized(json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "definitions": {
                "QuerySelectorInclude": {
                    "type": "string",
                    "enum": ["text", "html", "attributes", "bounds"]
                }
            },
            "type": "object",
            "additionalProperties": false,
            "required": ["selector"],
            "properties": {
                "selector": { "type": "string", "minLength": 1 },
                "limit": { "type": "integer", "exclusiveMinimum": 0 },
                "include": {
                    "type": "array",
                    "items": { "$ref": "#/definitions/QuerySelectorInclude" }
                }
            }
        }));
        assert!(
            out.get("definitions").is_none(),
            "draft-07 definitions block must be dropped: {out}"
        );
        let include_items = out
            .pointer("/properties/include/items")
            .expect("array items present");
        assert_eq!(
            include_items,
            &json!({
                "type": "string",
                "enum": ["text", "html", "attributes", "bounds"]
            })
        );
        let required = out
            .pointer("/required")
            .and_then(Value::as_array)
            .expect("top-level required preserved");
        assert_eq!(required, &vec![json!("selector")]);
    }
}
