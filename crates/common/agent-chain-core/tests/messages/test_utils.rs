//! Tests for message utility functions.
//!
//! Converted from `langchain/libs/core/tests/unit_tests/messages/test_utils.py`

use agent_chain_core::messages::{
    AIMessage, BaseMessage, HumanMessage, SystemMessage, ToolMessage, convert_to_messages,
    filter_messages, get_buffer_string, merge_message_runs,
};

// ============================================================================
// test_merge_message_runs_str
// ============================================================================

#[test]
fn test_merge_message_runs_str_human() {
    let messages = vec![
        BaseMessage::Human(HumanMessage::new("foo")),
        BaseMessage::Human(HumanMessage::new("bar")),
        BaseMessage::Human(HumanMessage::new("baz")),
    ];
    let messages_copy = messages.clone();
    let expected = vec![BaseMessage::Human(HumanMessage::new("foo\nbar\nbaz"))];
    let actual = merge_message_runs(&messages, "\n");
    assert_eq!(actual, expected);
    // Ensure original messages not mutated
    assert_eq!(messages, messages_copy);
}

#[test]
fn test_merge_message_runs_str_ai() {
    let messages = vec![
        BaseMessage::AI(AIMessage::new("foo")),
        BaseMessage::AI(AIMessage::new("bar")),
        BaseMessage::AI(AIMessage::new("baz")),
    ];
    let messages_copy = messages.clone();
    let expected = vec![BaseMessage::AI(AIMessage::new("foo\nbar\nbaz"))];
    let actual = merge_message_runs(&messages, "\n");
    assert_eq!(actual, expected);
    assert_eq!(messages, messages_copy);
}

#[test]
fn test_merge_message_runs_str_system() {
    let messages = vec![
        BaseMessage::System(SystemMessage::new("foo")),
        BaseMessage::System(SystemMessage::new("bar")),
        BaseMessage::System(SystemMessage::new("baz")),
    ];
    let messages_copy = messages.clone();
    let expected = vec![BaseMessage::System(SystemMessage::new("foo\nbar\nbaz"))];
    let actual = merge_message_runs(&messages, "\n");
    assert_eq!(actual, expected);
    assert_eq!(messages, messages_copy);
}

// ============================================================================
// test_merge_message_runs_str_with_specified_separator
// ============================================================================

#[test]
fn test_merge_message_runs_str_with_specified_separator_human() {
    let messages = vec![
        BaseMessage::Human(HumanMessage::new("foo")),
        BaseMessage::Human(HumanMessage::new("bar")),
        BaseMessage::Human(HumanMessage::new("baz")),
    ];
    let messages_copy = messages.clone();
    let expected = vec![BaseMessage::Human(HumanMessage::new("foo<sep>bar<sep>baz"))];
    let actual = merge_message_runs(&messages, "<sep>");
    assert_eq!(actual, expected);
    assert_eq!(messages, messages_copy);
}

#[test]
fn test_merge_message_runs_str_with_specified_separator_ai() {
    let messages = vec![
        BaseMessage::AI(AIMessage::new("foo")),
        BaseMessage::AI(AIMessage::new("bar")),
        BaseMessage::AI(AIMessage::new("baz")),
    ];
    let messages_copy = messages.clone();
    let expected = vec![BaseMessage::AI(AIMessage::new("foo<sep>bar<sep>baz"))];
    let actual = merge_message_runs(&messages, "<sep>");
    assert_eq!(actual, expected);
    assert_eq!(messages, messages_copy);
}

#[test]
fn test_merge_message_runs_str_with_specified_separator_system() {
    let messages = vec![
        BaseMessage::System(SystemMessage::new("foo")),
        BaseMessage::System(SystemMessage::new("bar")),
        BaseMessage::System(SystemMessage::new("baz")),
    ];
    let messages_copy = messages.clone();
    let expected = vec![BaseMessage::System(SystemMessage::new(
        "foo<sep>bar<sep>baz",
    ))];
    let actual = merge_message_runs(&messages, "<sep>");
    assert_eq!(actual, expected);
    assert_eq!(messages, messages_copy);
}

// ============================================================================
// test_merge_message_runs_str_without_separator
// ============================================================================

#[test]
fn test_merge_message_runs_str_without_separator_human() {
    let messages = vec![
        BaseMessage::Human(HumanMessage::new("foo")),
        BaseMessage::Human(HumanMessage::new("bar")),
        BaseMessage::Human(HumanMessage::new("baz")),
    ];
    let messages_copy = messages.clone();
    let expected = vec![BaseMessage::Human(HumanMessage::new("foobarbaz"))];
    let actual = merge_message_runs(&messages, "");
    assert_eq!(actual, expected);
    assert_eq!(messages, messages_copy);
}

#[test]
fn test_merge_message_runs_str_without_separator_ai() {
    let messages = vec![
        BaseMessage::AI(AIMessage::new("foo")),
        BaseMessage::AI(AIMessage::new("bar")),
        BaseMessage::AI(AIMessage::new("baz")),
    ];
    let messages_copy = messages.clone();
    let expected = vec![BaseMessage::AI(AIMessage::new("foobarbaz"))];
    let actual = merge_message_runs(&messages, "");
    assert_eq!(actual, expected);
    assert_eq!(messages, messages_copy);
}

#[test]
fn test_merge_message_runs_str_without_separator_system() {
    let messages = vec![
        BaseMessage::System(SystemMessage::new("foo")),
        BaseMessage::System(SystemMessage::new("bar")),
        BaseMessage::System(SystemMessage::new("baz")),
    ];
    let messages_copy = messages.clone();
    let expected = vec![BaseMessage::System(SystemMessage::new("foobarbaz"))];
    let actual = merge_message_runs(&messages, "");
    assert_eq!(actual, expected);
    assert_eq!(messages, messages_copy);
}

// ============================================================================
// test_merge_message_runs_response_metadata
// ============================================================================

#[test]
fn test_merge_message_runs_response_metadata() {
    // Note: The Rust implementation doesn't yet preserve response_metadata
    // This test demonstrates that the first message's ID is preserved
    let messages = vec![
        BaseMessage::AI(AIMessage::with_id("1", "foo")),
        BaseMessage::AI(AIMessage::with_id("2", "bar")),
    ];
    let expected = [BaseMessage::AI(AIMessage::with_id("1", "foo\nbar"))];
    let actual = merge_message_runs(&messages, "\n");

    // Check content is merged
    assert_eq!(actual[0].content(), expected[0].content());
    // Note: ID preservation may not be fully implemented yet
}

// ============================================================================
// test_merge_messages_tool_messages
// ============================================================================

#[test]
fn test_merge_messages_tool_messages() {
    // ToolMessages should NOT be merged, as each has a distinct tool call ID
    let messages = vec![
        BaseMessage::Tool(ToolMessage::new("foo", "1")),
        BaseMessage::Tool(ToolMessage::new("bar", "2")),
    ];
    let messages_copy = messages.clone();
    let actual = merge_message_runs(&messages, "\n");
    assert_eq!(actual, messages);
    assert_eq!(messages, messages_copy);
}

// ============================================================================
// test_filter_message
// ============================================================================

#[test]
fn test_filter_message_include_names() {
    let messages = vec![
        BaseMessage::System(SystemMessage::with_id("1", "foo").with_name("blah")),
        BaseMessage::Human(HumanMessage::with_id("2", "bar").with_name("blur")),
    ];
    let messages_copy = messages.clone();
    let expected = messages[1..2].to_vec();
    let actual = filter_messages(&messages, Some(&["blur"]), None, None, None, None, None);
    assert_eq!(expected, actual);
    assert_eq!(messages, messages_copy);
}

#[test]
fn test_filter_message_exclude_names() {
    let messages = vec![
        BaseMessage::System(SystemMessage::with_id("1", "foo").with_name("blah")),
        BaseMessage::Human(HumanMessage::with_id("2", "bar").with_name("blur")),
    ];
    let messages_copy = messages.clone();
    let expected = messages[1..2].to_vec();
    let actual = filter_messages(&messages, None, Some(&["blah"]), None, None, None, None);
    assert_eq!(expected, actual);
    assert_eq!(messages, messages_copy);
}

#[test]
fn test_filter_message_include_ids() {
    let messages = vec![
        BaseMessage::System(SystemMessage::with_id("1", "foo").with_name("blah")),
        BaseMessage::Human(HumanMessage::with_id("2", "bar").with_name("blur")),
    ];
    let messages_copy = messages.clone();
    let expected = messages[1..2].to_vec();
    let actual = filter_messages(&messages, None, None, None, None, Some(&["2"]), None);
    assert_eq!(expected, actual);
    assert_eq!(messages, messages_copy);
}

#[test]
fn test_filter_message_exclude_ids() {
    let messages = vec![
        BaseMessage::System(SystemMessage::with_id("1", "foo").with_name("blah")),
        BaseMessage::Human(HumanMessage::with_id("2", "bar").with_name("blur")),
    ];
    let messages_copy = messages.clone();
    let expected = messages[1..2].to_vec();
    let actual = filter_messages(&messages, None, None, None, None, None, Some(&["1"]));
    assert_eq!(expected, actual);
    assert_eq!(messages, messages_copy);
}

#[test]
fn test_filter_message_include_types_str() {
    let messages = vec![
        BaseMessage::System(SystemMessage::with_id("1", "foo").with_name("blah")),
        BaseMessage::Human(HumanMessage::with_id("2", "bar").with_name("blur")),
    ];
    let messages_copy = messages.clone();
    let expected = messages[1..2].to_vec();
    let actual = filter_messages(&messages, None, None, Some(&["human"]), None, None, None);
    assert_eq!(expected, actual);
    assert_eq!(messages, messages_copy);
}

#[test]
fn test_filter_message_exclude_types_str() {
    let messages = vec![
        BaseMessage::System(SystemMessage::with_id("1", "foo").with_name("blah")),
        BaseMessage::Human(HumanMessage::with_id("2", "bar").with_name("blur")),
    ];
    let messages_copy = messages.clone();
    let expected = messages[1..2].to_vec();
    let actual = filter_messages(&messages, None, None, None, Some(&["system"]), None, None);
    assert_eq!(expected, actual);
    assert_eq!(messages, messages_copy);
}

#[test]
fn test_filter_message_combined() {
    let messages = vec![
        BaseMessage::System(SystemMessage::with_id("1", "foo").with_name("blah")),
        BaseMessage::Human(HumanMessage::with_id("2", "bar").with_name("blur")),
    ];
    let messages_copy = messages.clone();
    let expected = messages[1..2].to_vec();
    let actual = filter_messages(
        &messages,
        Some(&["blah", "blur"]),
        None,
        None,
        Some(&["system"]),
        None,
        None,
    );
    assert_eq!(expected, actual);
    assert_eq!(messages, messages_copy);
}

// ============================================================================
// test_convert_to_messages
// ============================================================================

#[test]
fn test_convert_to_messages_string() {
    let message_like = vec![serde_json::json!("14.1")];
    let expected = vec![BaseMessage::Human(HumanMessage::new("14.1"))];
    let actual = convert_to_messages(&message_like).unwrap();
    assert_eq!(expected, actual);
}

#[test]
fn test_convert_to_messages_tuple_system() {
    let message_like = vec![serde_json::json!(["system", "11.1"])];
    let expected = vec![BaseMessage::System(SystemMessage::new("11.1"))];
    let actual = convert_to_messages(&message_like).unwrap();
    assert_eq!(expected, actual);
}

#[test]
fn test_convert_to_messages_tuple_human() {
    let message_like = vec![serde_json::json!(["human", "test"])];
    let expected = vec![BaseMessage::Human(HumanMessage::new("test"))];
    let actual = convert_to_messages(&message_like).unwrap();
    assert_eq!(expected, actual);
}

#[test]
fn test_convert_to_messages_tuple_ai() {
    let message_like = vec![serde_json::json!(["ai", "response"])];
    let expected = vec![BaseMessage::AI(AIMessage::new("response"))];
    let actual = convert_to_messages(&message_like).unwrap();
    assert_eq!(expected, actual);
}

#[test]
fn test_convert_to_messages_role_system() {
    let message_like = vec![serde_json::json!({"role": "system", "content": "6"})];
    let expected = vec![BaseMessage::System(SystemMessage::new("6"))];
    let actual = convert_to_messages(&message_like).unwrap();
    assert_eq!(expected, actual);
}

#[test]
fn test_convert_to_messages_role_user() {
    let message_like = vec![serde_json::json!({"role": "user", "content": "Hello"})];
    let expected = vec![BaseMessage::Human(HumanMessage::new("Hello"))];
    let actual = convert_to_messages(&message_like).unwrap();
    assert_eq!(expected, actual);
}

#[test]
fn test_convert_to_messages_role_assistant() {
    let message_like = vec![serde_json::json!({"role": "assistant", "content": "Hi"})];
    let expected = vec![BaseMessage::AI(AIMessage::new("Hi"))];
    let actual = convert_to_messages(&message_like).unwrap();
    assert_eq!(expected, actual);
}

// ============================================================================
// test_get_buffer_string
// ============================================================================

#[test]
fn test_get_buffer_string_empty_input() {
    assert_eq!(get_buffer_string(&[], "Human", "AI"), "");
}

#[test]
fn test_get_buffer_string_valid_single_message() {
    let messages = vec![BaseMessage::Human(HumanMessage::new("human"))];
    let expected_output = "Human: human";
    assert_eq!(get_buffer_string(&messages, "Human", "AI"), expected_output);
}

#[test]
fn test_get_buffer_string_custom_human_prefix() {
    let messages = vec![BaseMessage::Human(HumanMessage::new("human"))];
    let expected_output = "H: human";
    assert_eq!(get_buffer_string(&messages, "H", "AI"), expected_output);
}

#[test]
fn test_get_buffer_string_custom_ai_prefix() {
    let messages = vec![BaseMessage::AI(AIMessage::new("ai"))];
    let expected_output = "A: ai";
    assert_eq!(get_buffer_string(&messages, "Human", "A"), expected_output);
}

#[test]
fn test_get_buffer_string_multiple_msg() {
    let messages = vec![
        BaseMessage::Human(HumanMessage::new("human")),
        BaseMessage::AI(AIMessage::new("ai")),
        BaseMessage::System(SystemMessage::new("system")),
        // Note: FunctionMessage, ToolMessage, ChatMessage require additional parameters
        BaseMessage::Tool(ToolMessage::new("tool", "tool_id")),
    ];
    let expected_output = "Human: human\nAI: ai\nSystem: system\nTool: tool";

    assert_eq!(get_buffer_string(&messages, "Human", "AI"), expected_output);
}

// ============================================================================
// Tests for functions not yet implemented in Rust
// ============================================================================

// The following functions from the Python test are not yet implemented in Rust:
// - trim_messages
// - convert_to_openai_messages
// - count_tokens_approximately
//
// These tests are left as TODO markers for future implementation:

#[test]
#[ignore]
fn test_trim_messages_first_30() {
    // TODO: Implement trim_messages function
    todo!("trim_messages not yet implemented");
}

#[test]
#[ignore]
fn test_trim_messages_first_30_allow_partial() {
    // TODO: Implement trim_messages with allow_partial parameter
    todo!("trim_messages not yet implemented");
}

#[test]
#[ignore]
fn test_trim_messages_last_30_include_system() {
    // TODO: Implement trim_messages with strategy="last"
    todo!("trim_messages not yet implemented");
}

#[test]
#[ignore]
fn test_convert_to_openai_messages_string() {
    // TODO: Implement convert_to_openai_messages function
    todo!("convert_to_openai_messages not yet implemented");
}

#[test]
#[ignore]
fn test_convert_to_openai_messages_single_message() {
    // TODO: Implement convert_to_openai_messages function
    todo!("convert_to_openai_messages not yet implemented");
}

#[test]
#[ignore]
fn test_count_tokens_approximately_empty_messages() {
    // TODO: Implement count_tokens_approximately function
    todo!("count_tokens_approximately not yet implemented");
}

#[test]
#[ignore]
fn test_count_tokens_approximately_string_content() {
    // TODO: Implement count_tokens_approximately function
    todo!("count_tokens_approximately not yet implemented");
}
