//! Unit tests for JSON schema dereferencing utilities.
//!
//! Converted from langchain/libs/core/tests/unit_tests/utils/test_json_schema.py

use agent_chain_core::utils::json_schema::dereference_refs;
use serde_json::json;

#[test]
fn test_dereference_refs_no_refs() {
    let schema = json!({
        "type": "object",
        "properties": {
            "first_name": {"type": "string"},
        },
    });
    let actual = dereference_refs(&schema, None, None);
    assert_eq!(actual, schema);
}

#[test]
fn test_dereference_refs_one_ref() {
    let schema = json!({
        "type": "object",
        "properties": {
            "first_name": {"$ref": "#/$defs/name"},
        },
        "$defs": {"name": {"type": "string"}},
    });
    let expected = json!({
        "type": "object",
        "properties": {
            "first_name": {"type": "string"},
        },
        "$defs": {"name": {"type": "string"}},
    });
    let actual = dereference_refs(&schema, None, None);
    assert_eq!(actual, expected);
}

#[test]
fn test_dereference_refs_multiple_refs() {
    let schema = json!({
        "type": "object",
        "properties": {
            "first_name": {"$ref": "#/$defs/name"},
            "other": {"$ref": "#/$defs/other"},
        },
        "$defs": {
            "name": {"type": "string"},
            "other": {"type": "object", "properties": {"age": "int", "height": "int"}},
        },
    });
    let expected = json!({
        "type": "object",
        "properties": {
            "first_name": {"type": "string"},
            "other": {"type": "object", "properties": {"age": "int", "height": "int"}},
        },
        "$defs": {
            "name": {"type": "string"},
            "other": {"type": "object", "properties": {"age": "int", "height": "int"}},
        },
    });
    let actual = dereference_refs(&schema, None, None);
    assert_eq!(actual, expected);
}

#[test]
fn test_dereference_refs_nested_refs_skip() {
    let schema = json!({
        "type": "object",
        "properties": {
            "info": {"$ref": "#/$defs/info"},
        },
        "$defs": {
            "name": {"type": "string"},
            "info": {
                "type": "object",
                "properties": {"age": "int", "name": {"$ref": "#/$defs/name"}},
            },
        },
    });
    let expected = json!({
        "type": "object",
        "properties": {
            "info": {
                "type": "object",
                "properties": {"age": "int", "name": {"type": "string"}},
            },
        },
        "$defs": {
            "name": {"type": "string"},
            "info": {
                "type": "object",
                "properties": {"age": "int", "name": {"$ref": "#/$defs/name"}},
            },
        },
    });
    let actual = dereference_refs(&schema, None, None);
    assert_eq!(actual, expected);
}

#[test]
fn test_dereference_refs_nested_refs_no_skip() {
    let schema = json!({
        "type": "object",
        "properties": {
            "info": {"$ref": "#/$defs/info"},
        },
        "$defs": {
            "name": {"type": "string"},
            "info": {
                "type": "object",
                "properties": {"age": "int", "name": {"$ref": "#/$defs/name"}},
            },
        },
    });
    let expected = json!({
        "type": "object",
        "properties": {
            "info": {
                "type": "object",
                "properties": {"age": "int", "name": {"type": "string"}},
            },
        },
        "$defs": {
            "name": {"type": "string"},
            "info": {
                "type": "object",
                "properties": {"age": "int", "name": {"type": "string"}},
            },
        },
    });
    // Empty skip_keys means no keys are skipped
    let actual = dereference_refs(&schema, None, Some(&[]));
    assert_eq!(actual, expected);
}

#[test]
fn test_dereference_refs_missing_ref() {
    // In the Python implementation, missing refs raise KeyError.
    // In the Rust implementation, missing refs return null.
    let schema = json!({
        "type": "object",
        "properties": {
            "first_name": {"$ref": "#/$defs/name"},
        },
        "$defs": {},
    });
    let actual = dereference_refs(&schema, None, None);
    // The resolved reference should be null since "name" doesn't exist in $defs
    assert!(actual["properties"]["first_name"].is_null());
}

#[test]
fn test_dereference_refs_remote_ref() {
    // In the Python implementation, remote refs raise ValueError.
    // In the Rust implementation, remote refs return null since they don't start with "#".
    let schema = json!({
        "type": "object",
        "properties": {
            "first_name": {"$ref": "https://somewhere/else/name"},
        },
    });
    let actual = dereference_refs(&schema, None, None);
    // Remote refs are not resolved, should return null
    assert!(actual["properties"]["first_name"].is_null());
}

#[test]
fn test_dereference_refs_integer_ref() {
    // In Python, $defs can have integer keys. In Rust with serde_json,
    // JSON object keys are always strings, so we use "400" as a string key.
    let schema = json!({
        "type": "object",
        "properties": {
            "error_400": {"$ref": "#/$defs/400"},
        },
        "$defs": {
            "400": {
                "type": "object",
                "properties": {"description": "Bad Request"},
            },
        },
    });
    let expected = json!({
        "type": "object",
        "properties": {
            "error_400": {
                "type": "object",
                "properties": {"description": "Bad Request"},
            },
        },
        "$defs": {
            "400": {
                "type": "object",
                "properties": {"description": "Bad Request"},
            },
        },
    });
    let actual = dereference_refs(&schema, None, None);
    assert_eq!(actual, expected);
}

#[test]
fn test_dereference_refs_string_ref() {
    let schema = json!({
        "type": "object",
        "properties": {
            "error_400": {"$ref": "#/$defs/400"},
        },
        "$defs": {
            "400": {
                "type": "object",
                "properties": {"description": "Bad Request"},
            },
        },
    });
    let expected = json!({
        "type": "object",
        "properties": {
            "error_400": {
                "type": "object",
                "properties": {"description": "Bad Request"},
            },
        },
        "$defs": {
            "400": {
                "type": "object",
                "properties": {"description": "Bad Request"},
            },
        },
    });
    let actual = dereference_refs(&schema, None, None);
    assert_eq!(actual, expected);
}

#[test]
fn test_dereference_refs_cyclical_refs() {
    let schema = json!({
        "type": "object",
        "properties": {
            "user": {"$ref": "#/$defs/user"},
            "customer": {"$ref": "#/$defs/user"},
        },
        "$defs": {
            "user": {
                "type": "object",
                "properties": {
                    "friends": {"type": "array", "items": {"$ref": "#/$defs/user"}}
                },
            }
        },
    });
    let expected = json!({
        "type": "object",
        "properties": {
            "user": {
                "type": "object",
                "properties": {
                    "friends": {
                        "type": "array",
                        "items": {},  // Recursion is broken here
                    }
                },
            },
            "customer": {
                "type": "object",
                "properties": {
                    "friends": {
                        "type": "array",
                        "items": {},  // Recursion is broken here
                    }
                },
            },
        },
        "$defs": {
            "user": {
                "type": "object",
                "properties": {
                    "friends": {"type": "array", "items": {"$ref": "#/$defs/user"}}
                },
            }
        },
    });
    let actual = dereference_refs(&schema, None, None);
    assert_eq!(actual, expected);
}

#[test]
fn test_dereference_refs_list_index() {
    // Test dereferencing refs that use list indices (e.g., anyOf/1).
    // Test case from the issue report - anyOf array with numeric index reference
    let schema = json!({
        "type": "object",
        "properties": {
            "payload": {
                "anyOf": [
                    {  // variant 0
                        "type": "object",
                        "properties": {"kind": {"type": "string", "const": "ONE"}},
                    },
                    {  // variant 1
                        "type": "object",
                        "properties": {
                            "kind": {"type": "string", "const": "TWO"},
                            "startDate": {
                                "type": "string",
                                "pattern": r"^\d{4}-\d{2}-\d{2}$",
                            },
                            "endDate": {
                                "$ref": "#/properties/payload/anyOf/1/properties/startDate"
                            },
                        },
                    },
                ]
            }
        },
    });

    let expected = json!({
        "type": "object",
        "properties": {
            "payload": {
                "anyOf": [
                    {  // variant 0
                        "type": "object",
                        "properties": {"kind": {"type": "string", "const": "ONE"}},
                    },
                    {  // variant 1
                        "type": "object",
                        "properties": {
                            "kind": {"type": "string", "const": "TWO"},
                            "startDate": {
                                "type": "string",
                                "pattern": r"^\d{4}-\d{2}-\d{2}$",
                            },
                            "endDate": {
                                "type": "string",
                                "pattern": r"^\d{4}-\d{2}-\d{2}$",
                            },
                        },
                    },
                ]
            }
        },
    });

    let actual = dereference_refs(&schema, None, None);
    assert_eq!(actual, expected);
}

#[test]
fn test_dereference_refs_list_index_oneof() {
    // Test oneOf array with numeric index reference
    let schema_oneof = json!({
        "type": "object",
        "properties": {
            "data": {
                "oneOf": [
                    {"type": "string"},
                    {"type": "number"},
                    {
                        "type": "object",
                        "properties": {"value": {"$ref": "#/properties/data/oneOf/1"}},
                    },
                ]
            }
        },
    });

    let expected_oneof = json!({
        "type": "object",
        "properties": {
            "data": {
                "oneOf": [
                    {"type": "string"},
                    {"type": "number"},
                    {"type": "object", "properties": {"value": {"type": "number"}}},
                ]
            }
        },
    });

    let actual_oneof = dereference_refs(&schema_oneof, None, None);
    assert_eq!(actual_oneof, expected_oneof);
}

#[test]
fn test_dereference_refs_list_index_allof() {
    // Test allOf array with numeric index reference
    let schema_allof = json!({
        "type": "object",
        "allOf": [
            {"properties": {"name": {"type": "string"}}},
            {"properties": {"age": {"type": "number"}}},
        ],
        "properties": {"copy_name": {"$ref": "#/allOf/0/properties/name"}},
    });

    let expected_allof = json!({
        "type": "object",
        "allOf": [
            {"properties": {"name": {"type": "string"}}},
            {"properties": {"age": {"type": "number"}}},
        ],
        "properties": {"copy_name": {"type": "string"}},
    });

    let actual_allof = dereference_refs(&schema_allof, None, None);
    assert_eq!(actual_allof, expected_allof);
}

#[test]
fn test_dereference_refs_list_index_out_of_bounds() {
    // Test edge case: out-of-bounds index should return null
    let schema_invalid = json!({
        "type": "object",
        "properties": {
            "data": {"anyOf": [{"type": "string"}]},
            "invalid": {"$ref": "#/properties/data/anyOf/5"},  // Index 5 doesn't exist
        },
    });

    let actual = dereference_refs(&schema_invalid, None, None);
    // Out of bounds should resolve to null
    assert!(actual["properties"]["invalid"].is_null());
}

#[test]
fn test_dereference_refs_list_index_negative() {
    // Test edge case: negative index should return null
    let schema_negative = json!({
        "type": "object",
        "properties": {
            "data": {"anyOf": [{"type": "string"}]},
            "invalid": {"$ref": "#/properties/data/anyOf/-1"},  // Negative index
        },
    });

    let actual = dereference_refs(&schema_negative, None, None);
    // Negative index should return null (not supported)
    assert!(actual["properties"]["invalid"].is_null());
}

#[test]
fn test_dereference_refs_mixed_ref_with_properties() {
    // Test dereferencing refs that have $ref plus other properties.
    let schema = json!({
        "type": "object",
        "properties": {
            "data": {
                "$ref": "#/$defs/BaseType",
                "description": "Additional description",
                "example": "some example",
            }
        },
        "$defs": {"BaseType": {"type": "string", "minLength": 1}},
    });

    let expected = json!({
        "type": "object",
        "properties": {
            "data": {
                "type": "string",
                "minLength": 1,
                "description": "Additional description",
                "example": "some example",
            }
        },
        "$defs": {"BaseType": {"type": "string", "minLength": 1}},
    });

    let actual = dereference_refs(&schema, None, None);
    assert_eq!(actual, expected);
}

#[test]
fn test_dereference_refs_complex_pattern() {
    // Test pattern that caused infinite recursion in MCP server schemas.
    let schema = json!({
        "type": "object",
        "properties": {
            "query": {"$ref": "#/$defs/Query", "additionalProperties": false}
        },
        "$defs": {
            "Query": {
                "type": "object",
                "properties": {"user": {"$ref": "#/$defs/User"}},
            },
            "User": {
                "type": "object",
                "properties": {
                    "id": {"type": "string"},
                    "profile": {"$ref": "#/$defs/UserProfile", "nullable": true},
                },
            },
            "UserProfile": {
                "type": "object",
                "properties": {"bio": {"type": "string"}},
            },
        },
    });

    // This should not cause infinite recursion
    let actual = dereference_refs(&schema, None, None);

    let expected = json!({
        "$defs": {
            "Query": {
                "properties": {"user": {"$ref": "#/$defs/User"}},
                "type": "object",
            },
            "User": {
                "properties": {
                    "id": {"type": "string"},
                    "profile": {"$ref": "#/$defs/UserProfile", "nullable": true},
                },
                "type": "object",
            },
            "UserProfile": {
                "properties": {"bio": {"type": "string"}},
                "type": "object",
            },
        },
        "properties": {
            "query": {
                "additionalProperties": false,
                "properties": {
                    "user": {
                        "properties": {
                            "id": {"type": "string"},
                            "profile": {
                                "nullable": true,
                                "properties": {"bio": {"type": "string"}},
                                "type": "object",
                            },
                        },
                        "type": "object",
                    }
                },
                "type": "object",
            }
        },
        "type": "object",
    });

    assert_eq!(actual, expected);
}

#[test]
fn test_dereference_refs_cyclical_mixed_refs() {
    // Test cyclical references with mixed $ref properties don't cause loops.
    let schema = json!({
        "type": "object",
        "properties": {"node": {"$ref": "#/$defs/Node"}},
        "$defs": {
            "Node": {
                "type": "object",
                "properties": {
                    "id": {"type": "string"},
                    "parent": {"$ref": "#/$defs/Node", "nullable": true},
                    "children": {"type": "array", "items": {"$ref": "#/$defs/Node"}},
                },
            }
        },
    });

    // This should handle cycles gracefully
    let actual = dereference_refs(&schema, None, None);

    assert_eq!(actual, json!({
        "$defs": {
            "Node": {
                "properties": {
                    "children": {"items": {"$ref": "#/$defs/Node"}, "type": "array"},
                    "id": {"type": "string"},
                    "parent": {"$ref": "#/$defs/Node", "nullable": true},
                },
                "type": "object",
            }
        },
        "properties": {
            "node": {
                "properties": {
                    "children": {"items": {}, "type": "array"},
                    "id": {"type": "string"},
                    "parent": {"nullable": true},
                },
                "type": "object",
            }
        },
        "type": "object",
    }));
}

#[test]
fn test_dereference_refs_empty_mixed_ref() {
    // Test mixed $ref with empty other properties.
    let schema = json!({
        "type": "object",
        "properties": {"data": {"$ref": "#/$defs/Base"}},
        "$defs": {"Base": {"type": "string"}},
    });

    let expected = json!({
        "type": "object",
        "properties": {"data": {"type": "string"}},
        "$defs": {"Base": {"type": "string"}},
    });

    let actual = dereference_refs(&schema, None, None);
    assert_eq!(actual, expected);
}

#[test]
fn test_dereference_refs_nested_mixed_refs() {
    // Test nested objects with mixed $ref properties.
    let schema = json!({
        "type": "object",
        "properties": {
            "outer": {
                "type": "object",
                "properties": {
                    "inner": {"$ref": "#/$defs/Base", "title": "Custom Title"}
                },
            }
        },
        "$defs": {"Base": {"type": "string", "minLength": 1}},
    });

    let expected = json!({
        "type": "object",
        "properties": {
            "outer": {
                "type": "object",
                "properties": {
                    "inner": {"type": "string", "minLength": 1, "title": "Custom Title"}
                },
            }
        },
        "$defs": {"Base": {"type": "string", "minLength": 1}},
    });

    let actual = dereference_refs(&schema, None, None);
    assert_eq!(actual, expected);
}

#[test]
fn test_dereference_refs_array_with_mixed_refs() {
    // Test arrays containing mixed $ref objects.
    let schema = json!({
        "type": "object",
        "properties": {
            "items": {
                "type": "array",
                "items": {"$ref": "#/$defs/Item", "description": "An item"},
            }
        },
        "$defs": {"Item": {"type": "string", "enum": ["a", "b", "c"]}},
    });

    let expected = json!({
        "type": "object",
        "properties": {
            "items": {
                "type": "array",
                "items": {
                    "type": "string",
                    "enum": ["a", "b", "c"],
                    "description": "An item",
                },
            }
        },
        "$defs": {"Item": {"type": "string", "enum": ["a", "b", "c"]}},
    });

    let actual = dereference_refs(&schema, None, None);
    assert_eq!(actual, expected);
}

#[test]
fn test_dereference_refs_mixed_ref_overrides_property() {
    // Test that mixed $ref properties override resolved properties correctly.
    let schema = json!({
        "type": "object",
        "properties": {
            "data": {
                "$ref": "#/$defs/Base",
                "type": "number",  // Override the resolved type
                "description": "Overridden description",
            }
        },
        "$defs": {"Base": {"type": "string", "description": "Original description"}},
    });

    let expected = json!({
        "type": "object",
        "properties": {
            "data": {
                "type": "number",  // Mixed property should override
                "description": "Overridden description",  // Mixed property should override
            }
        },
        "$defs": {"Base": {"type": "string", "description": "Original description"}},
    });

    let actual = dereference_refs(&schema, None, None);
    assert_eq!(actual, expected);
}

#[test]
fn test_dereference_refs_mixed_ref_cyclical_with_properties() {
    // Test cyclical mixed $refs preserve non-ref properties correctly.
    let schema = json!({
        "type": "object",
        "properties": {"root": {"$ref": "#/$defs/Node", "required": true}},
        "$defs": {
            "Node": {
                "type": "object",
                "properties": {
                    "id": {"type": "string"},
                    "child": {"$ref": "#/$defs/Node", "nullable": true},
                },
            }
        },
    });

    let expected = json!({
        "type": "object",
        "properties": {
            "root": {
                "type": "object",
                "properties": {
                    "id": {"type": "string"},
                    "child": {"nullable": true},  // Cycle broken but nullable preserved
                },
                "required": true,  // Mixed property preserved
            }
        },
        "$defs": {
            "Node": {
                "type": "object",
                "properties": {
                    "id": {"type": "string"},
                    "child": {"$ref": "#/$defs/Node", "nullable": true},
                },
            }
        },
    });

    let actual = dereference_refs(&schema, None, None);
    assert_eq!(actual, expected);
}

#[test]
fn test_dereference_refs_non_dict_ref_target() {
    // Test $ref that resolves to non-dict values.
    let schema = json!({
        "type": "object",
        "properties": {
            "simple_ref": {"$ref": "#/$defs/SimpleString"},
            "mixed_ref": {
                "$ref": "#/$defs/SimpleString",
                "description": "With description",
            },
        },
        "$defs": {
            "SimpleString": "string"  // Non-dict definition
        },
    });

    let expected = json!({
        "type": "object",
        "properties": {
            "simple_ref": "string",
            "mixed_ref": {
                "description": "With description"
            },  // Can't merge with non-dict
        },
        "$defs": {"SimpleString": "string"},
    });

    let actual = dereference_refs(&schema, None, None);
    assert_eq!(actual, expected);
}
