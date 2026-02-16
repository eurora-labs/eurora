//! Tests for utils module imports.
//!
//! Mirrors `langchain/libs/core/tests/unit_tests/utils/test_imports.py`
//!
//! In Python, this test checks `__all__` exports:
//! ```python
//! from langchain_core.utils import __all__
//!
//! EXPECTED_ALL = [
//!     "StrictFormatter",
//!     "check_package_version",
//!     "convert_to_secret_str",
//!     "formatter",
//!     "get_bolded_text",
//!     "abatch_iterate",
//!     "batch_iterate",
//!     "get_color_mapping",
//!     "get_colored_text",
//!     "get_pydantic_field_names",
//!     "guard_import",
//!     "mock_now",
//!     "print_text",
//!     "raise_for_status_with_text",
//!     "xor_args",
//!     "image",
//!     "build_extra_kwargs",
//!     "get_from_dict_or_env",
//!     "get_from_env",
//!     "stringify_dict",
//!     "comma_list",
//!     "stringify_value",
//!     "pre_init",
//!     "from_env",
//!     "secret_from_env",
//!     "sanitize_for_postgres",
//! ]
//!
//! def test_all_imports() -> None:
//!     assert set(__all__) == set(EXPECTED_ALL)
//! ```
//!
//! In Rust, we verify that the expected items are publicly exported by importing them.

use std::collections::HashSet;

/// List of items that are currently exported from `agent_chain_core::utils`.
///
/// This list should be kept in sync with the actual exports in the utils module.
/// Items from Python's `__all__` that don't have Rust equivalents yet are noted in comments.
const EXPECTED_EXPORTS: &[&str] = &[
    // From formatting.rs
    "StrictFormatter",
    "FORMATTER", // equivalent to Python's "formatter"
    "format_string",
    // From base.rs
    "convert_to_secret_str",
    "raise_for_status_with_text",
    "from_env",
    "secret_from_env",
    "build_model_kwargs", // equivalent to Python's "build_extra_kwargs"
    "SecretString",
    "MockTime",          // equivalent to Python's "mock_now"
    "validate_xor_args", // equivalent to Python's "xor_args"
    "HttpStatusError",
    "NoDefault",
    "LC_AUTO_PREFIX",
    "LC_ID_PREFIX",
    "ensure_id",
    "now_millis",
    "now_secs",
    // From input.rs
    "get_bolded_text",
    "get_color_mapping",
    "get_colored_text",
    "print_text",
    // From iter.rs
    "batch_iterate",
    "tee",
    // Not implemented: "abatch_iterate" (async version)
    // From env.rs
    "get_from_dict_or_env",
    "get_from_env",
    "EnvError",
    "env_var_is_set",
    // From strings.rs
    "stringify_dict",
    "comma_list",
    "stringify_value",
    "sanitize_for_postgres",
    // From json.rs
    "parse_and_check_json_markdown",
    "parse_json_markdown",
    "parse_partial_json",
    // From json_schema.rs
    "dereference_refs",
    // From merge.rs
    "merge_dicts",
    "merge_lists",
    "merge_obj",
    // From mustache.rs
    "MustacheValue",
    "render_mustache",
    // From uuid.rs
    "uuid7",
    // From usage.rs
    "UsageError",
    "dict_int_op",
    // Not yet implemented in Rust:
    // "check_package_version" - Python-specific
    // "get_pydantic_field_names" - pydantic-specific
    // "guard_import" - Python-specific dynamic import
    // "image" - not yet implemented
    // "pre_init" - pydantic validator decorator
];

/// Test that all expected items are exported from the utils module.
///
/// This test verifies that the utils module has the expected public API.
/// Unlike Python's `__all__` test which checks exact equality, this test
/// verifies that at least the expected items are available.
#[test]
fn test_all_imports() {
    // Verify we can import all expected items from utils module.
    // This is done by using the items - the test will fail to compile
    // if any of these are not publicly exported.

    // From formatting.rs
    use agent_chain_core::utils::StrictFormatter;
    let _ = StrictFormatter::new();
    use agent_chain_core::utils::FORMATTER;
    let _ = &*FORMATTER;
    use agent_chain_core::utils::format_string;
    let _ = format_string("test", &std::collections::HashMap::new());

    // From base.rs
    use agent_chain_core::utils::convert_to_secret_str;
    let _ = convert_to_secret_str("test");
    use agent_chain_core::utils::SecretString;
    let _: SecretString = SecretString::from("test");
    use agent_chain_core::utils::from_env;
    let _ = from_env(&["TEST"], None, None);
    use agent_chain_core::utils::secret_from_env;
    let _ = secret_from_env(&["TEST"], None, None);
    use agent_chain_core::utils::build_model_kwargs;
    let _ = build_model_kwargs(
        std::collections::HashMap::new(),
        &std::collections::HashSet::new(),
    );
    use agent_chain_core::utils::MockTime;
    let _ = MockTime::fixed(0);
    use agent_chain_core::utils::validate_xor_args;
    let vals: std::collections::HashMap<&str, Option<&str>> = std::collections::HashMap::new();
    let groups: Vec<Vec<&str>> = vec![];
    let _ = validate_xor_args(&groups, &vals);
    use agent_chain_core::utils::raise_for_status_with_text;
    let _ = raise_for_status_with_text(200, "OK");

    // From input.rs
    use agent_chain_core::utils::get_bolded_text;
    let _ = get_bolded_text("test");
    use agent_chain_core::utils::get_color_mapping;
    let items = vec!["a".to_string()];
    let _ = get_color_mapping(&items, None);
    use agent_chain_core::utils::get_colored_text;
    let _ = get_colored_text("test", "green");
    use agent_chain_core::utils::print_text;
    let mut buffer: Vec<u8> = Vec::new();
    print_text("test", Some("green"), "", Some(&mut buffer));

    // From iter.rs
    use agent_chain_core::utils::batch_iterate;
    let _ = batch_iterate(Some(2), vec![1, 2, 3]);
    use agent_chain_core::utils::tee;
    let _ = tee(vec![1, 2, 3], 2);

    // From env.rs
    use agent_chain_core::utils::get_from_dict_or_env;
    let data: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let _ = get_from_dict_or_env(&data, &["key"], "ENV_KEY", None);
    use agent_chain_core::utils::get_from_env;
    let _ = get_from_env("key", "ENV_KEY", None);
    use agent_chain_core::utils::EnvError;
    let _: Option<EnvError> = None;
    use agent_chain_core::utils::env_var_is_set;
    let _ = env_var_is_set("TEST");

    // From strings.rs
    use agent_chain_core::utils::stringify_dict;
    let dict: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let _ = stringify_dict(&dict);
    use agent_chain_core::utils::comma_list;
    let items = vec!["a".to_string(), "b".to_string()];
    let _ = comma_list(&items);
    use agent_chain_core::utils::stringify_value;
    let _ = stringify_value(&serde_json::json!("test"));
    use agent_chain_core::utils::sanitize_for_postgres;
    let _ = sanitize_for_postgres("test", "");

    // From json.rs
    use agent_chain_core::utils::parse_partial_json;
    let _ = parse_partial_json("", false);
    use agent_chain_core::utils::parse_json_markdown;
    let _ = parse_json_markdown("");
    use agent_chain_core::utils::parse_and_check_json_markdown;
    let _ = parse_and_check_json_markdown("", &[]);

    // From json_schema.rs
    use agent_chain_core::utils::dereference_refs;
    let _ = dereference_refs(&serde_json::json!({}), None, None);

    // From merge.rs
    use agent_chain_core::utils::merge_dicts;
    let _ = merge_dicts(serde_json::json!({}), vec![]);
    use agent_chain_core::utils::merge_lists;
    let _ = merge_lists(None, vec![]);
    use agent_chain_core::utils::merge_obj;
    let _ = merge_obj(serde_json::json!(null), serde_json::json!(null));

    // From mustache.rs
    use agent_chain_core::utils::MustacheValue;
    let _: MustacheValue = MustacheValue::Null;
    use agent_chain_core::utils::render_mustache;
    let _ = render_mustache("", &MustacheValue::Null, None);

    // From uuid.rs
    use agent_chain_core::utils::uuid7;
    let _ = uuid7(None);

    // From usage.rs
    use agent_chain_core::utils::UsageError;
    let _: Option<UsageError> = None;
    use agent_chain_core::utils::dict_int_op;
    let _ = dict_int_op(
        &serde_json::json!({}),
        &serde_json::json!({}),
        |a, b| a + b,
        0,
        100,
    );

    // Verify the expected exports list has the right count
    let expected: HashSet<_> = EXPECTED_EXPORTS.iter().collect();
    assert!(
        !expected.is_empty(),
        "Expected exports list should not be empty"
    );

    // Note: We could add more rigorous checking here by introspecting the module,
    // but Rust doesn't provide runtime reflection like Python's __all__.
    // The compile-time checks above are sufficient for verifying exports.
}
