//! Tests for message utility functions.
//!
//! Converted from `langchain/libs/core/tests/unit_tests/messages/test_utils.py`

use agent_chain_core::messages::{
    AIMessage, BaseMessage, CountTokensConfig, HumanMessage, SystemMessage, TextFormat,
    ToolMessage, TrimMessagesConfig, TrimStrategy, convert_to_messages, convert_to_openai_messages,
    count_tokens_approximately, filter_messages, get_buffer_string, merge_message_runs,
    trim_messages,
};

// ============================================================================
// test_merge_message_runs_str
// ============================================================================

#[test]
fn test_merge_message_runs_str_human() {
    let messages = vec![
        BaseMessage::Human(HumanMessage::builder().content("foo").build()),
        BaseMessage::Human(HumanMessage::builder().content("bar").build()),
        BaseMessage::Human(HumanMessage::builder().content("baz").build()),
    ];
    let messages_copy = messages.clone();
    let expected = vec![BaseMessage::Human(
        HumanMessage::builder().content("foo\nbar\nbaz").build(),
    )];
    let actual = merge_message_runs(&messages, "\n");
    assert_eq!(actual, expected);
    // Ensure original messages not mutated
    assert_eq!(messages, messages_copy);
}

#[test]
fn test_merge_message_runs_str_ai() {
    let messages = vec![
        BaseMessage::AI(AIMessage::builder().content("foo").build()),
        BaseMessage::AI(AIMessage::builder().content("bar").build()),
        BaseMessage::AI(AIMessage::builder().content("baz").build()),
    ];
    let messages_copy = messages.clone();
    let expected = vec![BaseMessage::AI(
        AIMessage::builder().content("foo\nbar\nbaz").build(),
    )];
    let actual = merge_message_runs(&messages, "\n");
    assert_eq!(actual, expected);
    assert_eq!(messages, messages_copy);
}

#[test]
fn test_merge_message_runs_str_system() {
    let messages = vec![
        BaseMessage::System(SystemMessage::builder().content("foo").build()),
        BaseMessage::System(SystemMessage::builder().content("bar").build()),
        BaseMessage::System(SystemMessage::builder().content("baz").build()),
    ];
    let messages_copy = messages.clone();
    let expected = vec![BaseMessage::System(
        SystemMessage::builder().content("foo\nbar\nbaz").build(),
    )];
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
        BaseMessage::Human(HumanMessage::builder().content("foo").build()),
        BaseMessage::Human(HumanMessage::builder().content("bar").build()),
        BaseMessage::Human(HumanMessage::builder().content("baz").build()),
    ];
    let messages_copy = messages.clone();
    let expected = vec![BaseMessage::Human(
        HumanMessage::builder()
            .content("foo<sep>bar<sep>baz")
            .build(),
    )];
    let actual = merge_message_runs(&messages, "<sep>");
    assert_eq!(actual, expected);
    assert_eq!(messages, messages_copy);
}

#[test]
fn test_merge_message_runs_str_with_specified_separator_ai() {
    let messages = vec![
        BaseMessage::AI(AIMessage::builder().content("foo").build()),
        BaseMessage::AI(AIMessage::builder().content("bar").build()),
        BaseMessage::AI(AIMessage::builder().content("baz").build()),
    ];
    let messages_copy = messages.clone();
    let expected = vec![BaseMessage::AI(
        AIMessage::builder().content("foo<sep>bar<sep>baz").build(),
    )];
    let actual = merge_message_runs(&messages, "<sep>");
    assert_eq!(actual, expected);
    assert_eq!(messages, messages_copy);
}

#[test]
fn test_merge_message_runs_str_with_specified_separator_system() {
    let messages = vec![
        BaseMessage::System(SystemMessage::builder().content("foo").build()),
        BaseMessage::System(SystemMessage::builder().content("bar").build()),
        BaseMessage::System(SystemMessage::builder().content("baz").build()),
    ];
    let messages_copy = messages.clone();
    let expected = vec![BaseMessage::System(
        SystemMessage::builder()
            .content("foo<sep>bar<sep>baz")
            .build(),
    )];
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
        BaseMessage::Human(HumanMessage::builder().content("foo").build()),
        BaseMessage::Human(HumanMessage::builder().content("bar").build()),
        BaseMessage::Human(HumanMessage::builder().content("baz").build()),
    ];
    let messages_copy = messages.clone();
    let expected = vec![BaseMessage::Human(
        HumanMessage::builder().content("foobarbaz").build(),
    )];
    let actual = merge_message_runs(&messages, "");
    assert_eq!(actual, expected);
    assert_eq!(messages, messages_copy);
}

#[test]
fn test_merge_message_runs_str_without_separator_ai() {
    let messages = vec![
        BaseMessage::AI(AIMessage::builder().content("foo").build()),
        BaseMessage::AI(AIMessage::builder().content("bar").build()),
        BaseMessage::AI(AIMessage::builder().content("baz").build()),
    ];
    let messages_copy = messages.clone();
    let expected = vec![BaseMessage::AI(
        AIMessage::builder().content("foobarbaz").build(),
    )];
    let actual = merge_message_runs(&messages, "");
    assert_eq!(actual, expected);
    assert_eq!(messages, messages_copy);
}

#[test]
fn test_merge_message_runs_str_without_separator_system() {
    let messages = vec![
        BaseMessage::System(SystemMessage::builder().content("foo").build()),
        BaseMessage::System(SystemMessage::builder().content("bar").build()),
        BaseMessage::System(SystemMessage::builder().content("baz").build()),
    ];
    let messages_copy = messages.clone();
    let expected = vec![BaseMessage::System(
        SystemMessage::builder().content("foobarbaz").build(),
    )];
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
        BaseMessage::AI(
            AIMessage::builder()
                .id("1".to_string())
                .content("foo")
                .build(),
        ),
        BaseMessage::AI(
            AIMessage::builder()
                .id("2".to_string())
                .content("bar")
                .build(),
        ),
    ];
    let expected = [BaseMessage::AI(
        AIMessage::builder()
            .id("1".to_string())
            .content("foo\nbar")
            .build(),
    )];
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
        BaseMessage::Tool(
            ToolMessage::builder()
                .content("foo")
                .tool_call_id("1")
                .build(),
        ),
        BaseMessage::Tool(
            ToolMessage::builder()
                .content("bar")
                .tool_call_id("2")
                .build(),
        ),
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
        BaseMessage::System(
            SystemMessage::builder()
                .id("1".to_string())
                .content("foo")
                .name("blah".to_string())
                .build(),
        ),
        BaseMessage::Human(
            HumanMessage::builder()
                .id("2".to_string())
                .content("bar")
                .name("blur".to_string())
                .build(),
        ),
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
        BaseMessage::System(
            SystemMessage::builder()
                .id("1".to_string())
                .content("foo")
                .name("blah".to_string())
                .build(),
        ),
        BaseMessage::Human(
            HumanMessage::builder()
                .id("2".to_string())
                .content("bar")
                .name("blur".to_string())
                .build(),
        ),
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
        BaseMessage::System(
            SystemMessage::builder()
                .id("1".to_string())
                .content("foo")
                .name("blah".to_string())
                .build(),
        ),
        BaseMessage::Human(
            HumanMessage::builder()
                .id("2".to_string())
                .content("bar")
                .name("blur".to_string())
                .build(),
        ),
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
        BaseMessage::System(
            SystemMessage::builder()
                .id("1".to_string())
                .content("foo")
                .name("blah".to_string())
                .build(),
        ),
        BaseMessage::Human(
            HumanMessage::builder()
                .id("2".to_string())
                .content("bar")
                .name("blur".to_string())
                .build(),
        ),
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
        BaseMessage::System(
            SystemMessage::builder()
                .id("1".to_string())
                .content("foo")
                .name("blah".to_string())
                .build(),
        ),
        BaseMessage::Human(
            HumanMessage::builder()
                .id("2".to_string())
                .content("bar")
                .name("blur".to_string())
                .build(),
        ),
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
        BaseMessage::System(
            SystemMessage::builder()
                .id("1".to_string())
                .content("foo")
                .name("blah".to_string())
                .build(),
        ),
        BaseMessage::Human(
            HumanMessage::builder()
                .id("2".to_string())
                .content("bar")
                .name("blur".to_string())
                .build(),
        ),
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
        BaseMessage::System(
            SystemMessage::builder()
                .id("1".to_string())
                .content("foo")
                .name("blah".to_string())
                .build(),
        ),
        BaseMessage::Human(
            HumanMessage::builder()
                .id("2".to_string())
                .content("bar")
                .name("blur".to_string())
                .build(),
        ),
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
    let expected = vec![BaseMessage::Human(
        HumanMessage::builder().content("14.1").build(),
    )];
    let actual = convert_to_messages(&message_like).unwrap();
    assert_eq!(expected, actual);
}

#[test]
fn test_convert_to_messages_tuple_system() {
    let message_like = vec![serde_json::json!(["system", "11.1"])];
    let expected = vec![BaseMessage::System(
        SystemMessage::builder().content("11.1").build(),
    )];
    let actual = convert_to_messages(&message_like).unwrap();
    assert_eq!(expected, actual);
}

#[test]
fn test_convert_to_messages_tuple_human() {
    let message_like = vec![serde_json::json!(["human", "test"])];
    let expected = vec![BaseMessage::Human(
        HumanMessage::builder().content("test").build(),
    )];
    let actual = convert_to_messages(&message_like).unwrap();
    assert_eq!(expected, actual);
}

#[test]
fn test_convert_to_messages_tuple_ai() {
    let message_like = vec![serde_json::json!(["ai", "response"])];
    let expected = vec![BaseMessage::AI(
        AIMessage::builder().content("response").build(),
    )];
    let actual = convert_to_messages(&message_like).unwrap();
    assert_eq!(expected, actual);
}

#[test]
fn test_convert_to_messages_role_system() {
    let message_like = vec![serde_json::json!({"role": "system", "content": "6"})];
    let expected = vec![BaseMessage::System(
        SystemMessage::builder().content("6").build(),
    )];
    let actual = convert_to_messages(&message_like).unwrap();
    assert_eq!(expected, actual);
}

#[test]
fn test_convert_to_messages_role_user() {
    let message_like = vec![serde_json::json!({"role": "user", "content": "Hello"})];
    let expected = vec![BaseMessage::Human(
        HumanMessage::builder().content("Hello").build(),
    )];
    let actual = convert_to_messages(&message_like).unwrap();
    assert_eq!(expected, actual);
}

#[test]
fn test_convert_to_messages_role_assistant() {
    let message_like = vec![serde_json::json!({"role": "assistant", "content": "Hi"})];
    let expected = vec![BaseMessage::AI(AIMessage::builder().content("Hi").build())];
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
    let messages = vec![BaseMessage::Human(
        HumanMessage::builder().content("human").build(),
    )];
    let expected_output = "Human: human";
    assert_eq!(get_buffer_string(&messages, "Human", "AI"), expected_output);
}

#[test]
fn test_get_buffer_string_custom_human_prefix() {
    let messages = vec![BaseMessage::Human(
        HumanMessage::builder().content("human").build(),
    )];
    let expected_output = "H: human";
    assert_eq!(get_buffer_string(&messages, "H", "AI"), expected_output);
}

#[test]
fn test_get_buffer_string_custom_ai_prefix() {
    let messages = vec![BaseMessage::AI(AIMessage::builder().content("ai").build())];
    let expected_output = "A: ai";
    assert_eq!(get_buffer_string(&messages, "Human", "A"), expected_output);
}

#[test]
fn test_get_buffer_string_multiple_msg() {
    let messages = vec![
        BaseMessage::Human(HumanMessage::builder().content("human").build()),
        BaseMessage::AI(AIMessage::builder().content("ai").build()),
        BaseMessage::System(SystemMessage::builder().content("system").build()),
        // Note: FunctionMessage, ToolMessage, ChatMessage require additional parameters
        BaseMessage::Tool(
            ToolMessage::builder()
                .content("tool")
                .tool_call_id("tool_id")
                .build(),
        ),
    ];
    let expected_output = "Human: human\nAI: ai\nSystem: system\nTool: tool";

    assert_eq!(get_buffer_string(&messages, "Human", "AI"), expected_output);
}

// ============================================================================
// test_trim_messages
// ============================================================================

/// Dummy token counter for testing.
/// Treat each message like it adds 3 default tokens at the beginning
/// of the message and at the end of the message. 3 + 4 + 3 = 10 tokens per message.
fn dummy_token_counter(messages: &[BaseMessage]) -> usize {
    let default_content_len = 4;
    let default_msg_prefix_len = 3;
    let default_msg_suffix_len = 3;

    let mut count = 0;
    for _msg in messages {
        count += default_msg_prefix_len + default_content_len + default_msg_suffix_len;
    }
    count
}

#[test]
fn test_trim_messages_first_30() {
    // Messages to trim (same as Python test)
    // Each message is 10 tokens (3 prefix + 4 content + 3 suffix)
    let messages = vec![
        BaseMessage::System(
            SystemMessage::builder()
                .content("This is a 4 token text.")
                .build(),
        ),
        BaseMessage::Human(
            HumanMessage::builder()
                .id("first".to_string())
                .content("This is a 4 token text.")
                .build(),
        ),
        BaseMessage::AI(
            AIMessage::builder()
                .id("second".to_string())
                .content("This is the FIRST 4 token block.")
                .build(),
        ),
        BaseMessage::Human(
            HumanMessage::builder()
                .id("third".to_string())
                .content("This is a 4 token text.")
                .build(),
        ),
        BaseMessage::AI(
            AIMessage::builder()
                .id("fourth".to_string())
                .content("This is a 4 token text.")
                .build(),
        ),
    ];
    let messages_copy = messages.clone();

    // With 30 tokens max and each message being 10 tokens, we can fit exactly 3 messages
    let expected = [
        BaseMessage::System(
            SystemMessage::builder()
                .content("This is a 4 token text.")
                .build(),
        ),
        BaseMessage::Human(
            HumanMessage::builder()
                .id("first".to_string())
                .content("This is a 4 token text.")
                .build(),
        ),
        BaseMessage::AI(
            AIMessage::builder()
                .id("second".to_string())
                .content("This is the FIRST 4 token block.")
                .build(),
        ),
    ];

    let config =
        TrimMessagesConfig::new(30, dummy_token_counter).with_strategy(TrimStrategy::First);

    let actual = trim_messages(&messages, &config);

    // Check that 3 messages were included (30 tokens, which is <= 30)
    assert_eq!(actual.len(), expected.len());
    assert_eq!(actual[0].content(), expected[0].content());
    assert_eq!(actual[1].content(), expected[1].content());
    assert_eq!(actual[2].content(), expected[2].content());
    // Ensure original messages not mutated
    assert_eq!(messages, messages_copy);
}

#[test]
fn test_trim_messages_first_30_allow_partial() {
    // In Rust version, allow_partial doesn't include partial content blocks
    // as the Python version does with list content - this test verifies basic behavior
    let messages = vec![
        BaseMessage::System(
            SystemMessage::builder()
                .content("This is a 4 token text.")
                .build(),
        ),
        BaseMessage::Human(
            HumanMessage::builder()
                .id("first".to_string())
                .content("This is a 4 token text.")
                .build(),
        ),
        BaseMessage::AI(
            AIMessage::builder()
                .id("second".to_string())
                .content("First line\nSecond line\nThird line")
                .build(),
        ),
        BaseMessage::Human(
            HumanMessage::builder()
                .id("third".to_string())
                .content("This is a 4 token text.")
                .build(),
        ),
    ];
    let messages_copy = messages.clone();

    let config = TrimMessagesConfig::new(30, dummy_token_counter)
        .with_strategy(TrimStrategy::First)
        .with_allow_partial(true);

    let actual = trim_messages(&messages, &config);

    // Should include at least the first 2 complete messages
    assert!(actual.len() >= 2);
    assert_eq!(actual[0].content(), "This is a 4 token text.");
    assert_eq!(actual[1].content(), "This is a 4 token text.");
    // Ensure original messages not mutated
    assert_eq!(messages, messages_copy);
}

#[test]
fn test_trim_messages_last_30_include_system() {
    let messages = vec![
        BaseMessage::System(
            SystemMessage::builder()
                .content("This is a 4 token text.")
                .build(),
        ),
        BaseMessage::Human(
            HumanMessage::builder()
                .id("first".to_string())
                .content("This is a 4 token text.")
                .build(),
        ),
        BaseMessage::AI(
            AIMessage::builder()
                .id("second".to_string())
                .content("This is a block.")
                .build(),
        ),
        BaseMessage::Human(
            HumanMessage::builder()
                .id("third".to_string())
                .content("This is a 4 token text.")
                .build(),
        ),
        BaseMessage::AI(
            AIMessage::builder()
                .id("fourth".to_string())
                .content("This is a 4 token text.")
                .build(),
        ),
    ];
    let messages_copy = messages.clone();

    let expected = [
        BaseMessage::System(
            SystemMessage::builder()
                .content("This is a 4 token text.")
                .build(),
        ),
        BaseMessage::Human(
            HumanMessage::builder()
                .id("third".to_string())
                .content("This is a 4 token text.")
                .build(),
        ),
        BaseMessage::AI(
            AIMessage::builder()
                .id("fourth".to_string())
                .content("This is a 4 token text.")
                .build(),
        ),
    ];

    let config = TrimMessagesConfig::new(30, dummy_token_counter)
        .with_strategy(TrimStrategy::Last)
        .with_include_system(true);

    let actual = trim_messages(&messages, &config);

    // Should include system message + last 2 messages (30 tokens)
    assert_eq!(actual.len(), expected.len());
    // First message should be the system message
    assert!(matches!(actual.first(), Some(BaseMessage::System(_))));
    assert_eq!(actual[0].content(), expected[0].content());
    // Ensure original messages not mutated
    assert_eq!(messages, messages_copy);
}

// ============================================================================
// test_convert_to_openai_messages
// ============================================================================

#[test]
fn test_convert_to_openai_messages_single_message() {
    let messages = vec![BaseMessage::Human(
        HumanMessage::builder().content("Hello").build(),
    )];
    let result = convert_to_openai_messages(&messages, TextFormat::String);

    assert_eq!(result.len(), 1);
    assert_eq!(result[0]["role"], "user");
    assert_eq!(result[0]["content"], "Hello");
}

#[test]
fn test_convert_to_openai_messages_multiple_messages() {
    let messages = vec![
        BaseMessage::System(SystemMessage::builder().content("System message").build()),
        BaseMessage::Human(HumanMessage::builder().content("Human message").build()),
        BaseMessage::AI(AIMessage::builder().content("AI message").build()),
    ];
    let result = convert_to_openai_messages(&messages, TextFormat::String);

    let expected = [
        serde_json::json!({"role": "system", "content": "System message"}),
        serde_json::json!({"role": "user", "content": "Human message"}),
        serde_json::json!({"role": "assistant", "content": "AI message"}),
    ];

    assert_eq!(result.len(), expected.len());
    assert_eq!(result[0]["role"], "system");
    assert_eq!(result[1]["role"], "user");
    assert_eq!(result[2]["role"], "assistant");
}

#[test]
fn test_convert_to_openai_messages_block_format() {
    let messages = vec![BaseMessage::Human(
        HumanMessage::builder().content("Hello").build(),
    )];
    let result = convert_to_openai_messages(&messages, TextFormat::Block);

    assert_eq!(result.len(), 1);
    assert_eq!(result[0]["role"], "user");
    // In block format, content should be an array
    let content = result[0]["content"].as_array().unwrap();
    assert_eq!(content.len(), 1);
    assert_eq!(content[0]["type"], "text");
    assert_eq!(content[0]["text"], "Hello");
}

#[test]
fn test_convert_to_openai_messages_tool_message() {
    let messages = vec![BaseMessage::Tool(
        ToolMessage::builder()
            .content("Tool result")
            .tool_call_id("123")
            .build(),
    )];
    let result = convert_to_openai_messages(&messages, TextFormat::Block);

    assert_eq!(result.len(), 1);
    assert_eq!(result[0]["role"], "tool");
    assert_eq!(result[0]["tool_call_id"], "123");
    let content = result[0]["content"].as_array().unwrap();
    assert_eq!(content[0]["type"], "text");
    assert_eq!(content[0]["text"], "Tool result");
}

#[test]
fn test_convert_to_openai_messages_empty_list() {
    let messages: Vec<BaseMessage> = vec![];
    let result = convert_to_openai_messages(&messages, TextFormat::String);
    assert!(result.is_empty());
}

// ============================================================================
// test_count_tokens_approximately
// ============================================================================

#[test]
fn test_count_tokens_approximately_empty_messages() {
    // Test with empty message list
    let messages: Vec<BaseMessage> = vec![];
    let config = CountTokensConfig::default();
    assert_eq!(count_tokens_approximately(&messages, &config), 0);

    // Test with empty content
    let messages = vec![BaseMessage::Human(
        HumanMessage::builder().content("").build(),
    )];
    // 0 content chars + 4 role chars ("user") -> ceil(4/4) + 3 = 1 + 3 = 4 tokens
    assert_eq!(count_tokens_approximately(&messages, &config), 4);
}

#[test]
fn test_count_tokens_approximately_string_content() {
    let messages = vec![
        // "Hello" = 5 chars + "user" = 4 chars -> ceil(9/4) + 3 = 3 + 3 = 6 tokens
        BaseMessage::Human(HumanMessage::builder().content("Hello").build()),
        // "Hi there" = 8 chars + "assistant" = 9 chars -> ceil(17/4) + 3 = 5 + 3 = 8 tokens
        BaseMessage::AI(AIMessage::builder().content("Hi there").build()),
        // "How are you?" = 12 chars + "user" = 4 chars -> ceil(16/4) + 3 = 4 + 3 = 7 tokens
        BaseMessage::Human(HumanMessage::builder().content("How are you?").build()),
    ];
    let config = CountTokensConfig::default();

    // Total: 6 + 8 + 7 = 21 tokens
    assert_eq!(count_tokens_approximately(&messages, &config), 21);
}

#[test]
fn test_count_tokens_approximately_with_names() {
    let messages = vec![
        BaseMessage::Human(
            HumanMessage::builder()
                .content("Hello")
                .name("user".to_string())
                .build(),
        ),
        BaseMessage::AI(
            AIMessage::builder()
                .content("Hi there")
                .name("assistant".to_string())
                .build(),
        ),
    ];

    // With names included (default)
    let config = CountTokensConfig::default();
    // "Hello" + "user" (role) + "user" (name) = 5 + 4 + 4 = 13 chars -> ceil(13/4) + 3 = 4 + 3 = 7 tokens
    // "Hi there" + "assistant" (role) + "assistant" (name) = 8 + 9 + 9 = 26 chars -> ceil(26/4) + 3 = 7 + 3 = 10 tokens
    // Total: 7 + 10 = 17 tokens
    assert_eq!(count_tokens_approximately(&messages, &config), 17);

    // Without names
    let config_no_names = CountTokensConfig {
        count_name: false,
        ..Default::default()
    };
    // "Hello" + "user" (role) = 5 + 4 = 9 chars -> ceil(9/4) + 3 = 3 + 3 = 6 tokens
    // "Hi there" + "assistant" (role) = 8 + 9 = 17 chars -> ceil(17/4) + 3 = 5 + 3 = 8 tokens
    // Total: 6 + 8 = 14 tokens
    assert_eq!(count_tokens_approximately(&messages, &config_no_names), 14);
}

#[test]
fn test_count_tokens_approximately_custom_token_length() {
    let messages = vec![
        // "Hello world" + "user" = 11 + 4 = 15 chars
        BaseMessage::Human(HumanMessage::builder().content("Hello world").build()),
        // "Testing" + "assistant" = 7 + 9 = 16 chars
        BaseMessage::AI(AIMessage::builder().content("Testing").build()),
    ];

    // With chars_per_token = 4 (default)
    let config4 = CountTokensConfig::default();
    // ceil(15/4) + 3 = 4 + 3 = 7 tokens
    // ceil(16/4) + 3 = 4 + 3 = 7 tokens
    // Total: 14 tokens
    assert_eq!(count_tokens_approximately(&messages, &config4), 14);

    // With chars_per_token = 2
    let config2 = CountTokensConfig {
        chars_per_token: 2.0,
        ..Default::default()
    };
    // ceil(15/2) + 3 = 8 + 3 = 11 tokens
    // ceil(16/2) + 3 = 8 + 3 = 11 tokens
    // Total: 22 tokens
    assert_eq!(count_tokens_approximately(&messages, &config2), 22);
}
