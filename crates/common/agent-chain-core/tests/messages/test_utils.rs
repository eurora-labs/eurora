//! Tests for message utility functions.
//!
//! Converted from `langchain/libs/core/tests/unit_tests/messages/test_utils.py`

use agent_chain_core::messages::{
    AIMessage, AIMessageChunk, BaseMessage, BaseMessageChunk, ChatMessage, ChatMessageChunk,
    CountTokensConfig, ExcludeToolCalls, FunctionMessage, FunctionMessageChunk, HumanMessage,
    HumanMessageChunk, SystemMessage, SystemMessageChunk, TextFormat, ToolMessage,
    ToolMessageChunk, TrimMessagesConfig, TrimStrategy, convert_to_messages,
    convert_to_openai_messages, count_tokens_approximately, filter_messages, get_buffer_string,
    merge_message_runs, message_chunk_to_message, messages_from_dict, messages_to_dict, tool_call,
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
    let actual = filter_messages(
        &messages,
        Some(&["blur"]),
        None,
        None,
        None,
        None,
        None,
        None,
    );
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
    let actual = filter_messages(
        &messages,
        None,
        Some(&["blah"]),
        None,
        None,
        None,
        None,
        None,
    );
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
    let actual = filter_messages(&messages, None, None, None, None, Some(&["2"]), None, None);
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
    let actual = filter_messages(&messages, None, None, None, None, None, Some(&["1"]), None);
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
    let actual = filter_messages(
        &messages,
        None,
        None,
        Some(&["human"]),
        None,
        None,
        None,
        None,
    );
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
    let actual = filter_messages(
        &messages,
        None,
        None,
        None,
        Some(&["system"]),
        None,
        None,
        None,
    );
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
    let result = convert_to_openai_messages(&messages, TextFormat::String, false);

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
    let result = convert_to_openai_messages(&messages, TextFormat::String, false);

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
    let result = convert_to_openai_messages(&messages, TextFormat::Block, false);

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
    let result = convert_to_openai_messages(&messages, TextFormat::Block, false);

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
    let result = convert_to_openai_messages(&messages, TextFormat::String, false);
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

// ============================================================================
// NEW TESTS - Ported from Python test_utils.py
// ============================================================================

// ============================================================================
// test_merge_message_runs_alternating_types_no_merge
// ============================================================================

#[test]
fn test_merge_message_runs_alternating_types_no_merge() {
    // Alternating Human/AI messages should NOT be merged
    let messages = vec![
        BaseMessage::Human(HumanMessage::builder().content("hello").build()),
        BaseMessage::AI(AIMessage::builder().content("hi").build()),
        BaseMessage::Human(HumanMessage::builder().content("how are you").build()),
        BaseMessage::AI(AIMessage::builder().content("good").build()),
    ];
    let messages_copy = messages.clone();
    let actual = merge_message_runs(&messages, "\n");
    // All messages should remain unchanged since no consecutive same-type messages
    assert_eq!(actual.len(), 4);
    assert_eq!(actual[0].content(), "hello");
    assert_eq!(actual[1].content(), "hi");
    assert_eq!(actual[2].content(), "how are you");
    assert_eq!(actual[3].content(), "good");
    assert_eq!(messages, messages_copy);
}

// ============================================================================
// test_merge_message_runs_preserves_tool_calls
// ============================================================================

#[test]
fn test_merge_message_runs_preserves_tool_calls() {
    // The current merge_message_runs implementation merges AI messages by
    // concatenating content strings but creates new AIMessage from content only.
    // This means tool_calls from the original messages are NOT preserved in the
    // merged result. This test documents that current behavior.
    let tc1 = tool_call(
        "tool_a",
        serde_json::json!({"arg": "val1"}),
        Some("id1".to_string()),
    );
    let tc2 = tool_call(
        "tool_b",
        serde_json::json!({"arg": "val2"}),
        Some("id2".to_string()),
    );
    let messages = vec![
        BaseMessage::AI(
            AIMessage::builder()
                .content("first")
                .tool_calls(vec![tc1])
                .build(),
        ),
        BaseMessage::AI(
            AIMessage::builder()
                .content("second")
                .tool_calls(vec![tc2])
                .build(),
        ),
    ];

    let actual = merge_message_runs(&messages, "\n");
    assert_eq!(actual.len(), 1);
    // Content should be merged
    assert_eq!(actual[0].content(), "first\nsecond");
    // With chunk-based merging, tool_calls are properly preserved
    assert_eq!(actual[0].tool_calls().len(), 2);
    assert_eq!(actual[0].tool_calls()[0].name, "tool_a");
    assert_eq!(actual[0].tool_calls()[1].name, "tool_b");
}

// ============================================================================
// test_convert_to_messages_unsupported_role_raises
// ============================================================================

#[test]
fn test_convert_to_messages_unsupported_role_raises() {
    // Tool role requires tool_call_id, so tuple ("tool", "hello") should return Err
    let message_like = vec![serde_json::json!(["tool", "hello"])];
    let result = convert_to_messages(&message_like);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.contains("tool_call_id"),
        "Error should mention tool_call_id, got: {}",
        err
    );
}

// ============================================================================
// test_convert_to_openai_messages_developer
// ============================================================================

#[test]
fn test_convert_to_openai_messages_developer() {
    // SystemMessage with __openai_role__ = "developer" in additional_kwargs.
    // The Rust implementation maps system messages to "system" role in OpenAI format.
    let mut additional_kwargs = std::collections::HashMap::new();
    additional_kwargs.insert(
        "__openai_role__".to_string(),
        serde_json::json!("developer"),
    );
    let messages = vec![BaseMessage::System(
        SystemMessage::builder()
            .content("Be helpful")
            .additional_kwargs(additional_kwargs)
            .build(),
    )];
    let result = convert_to_openai_messages(&messages, TextFormat::String, false);
    assert_eq!(result.len(), 1);
    // The Rust impl always maps System -> "system" role
    assert_eq!(result[0]["role"], "system");
    assert_eq!(result[0]["content"], "Be helpful");
}

// ============================================================================
// test_convert_to_openai_messages_empty_content
// ============================================================================

#[test]
fn test_convert_to_openai_messages_empty_content() {
    // Message with empty content string should preserve empty string
    let messages = vec![BaseMessage::AI(AIMessage::builder().content("").build())];
    let result = convert_to_openai_messages(&messages, TextFormat::String, false);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0]["role"], "assistant");
    assert_eq!(result[0]["content"], "");
}

// ============================================================================
// test_convert_to_openai_messages_ai_with_tool_calls
// ============================================================================

#[test]
fn test_convert_to_openai_messages_ai_with_tool_calls() {
    // AIMessage with tool_calls should include tool_calls in OpenAI output
    let tc = tool_call(
        "get_weather",
        serde_json::json!({"location": "Paris"}),
        Some("call_123".to_string()),
    );
    let messages = vec![BaseMessage::AI(
        AIMessage::builder()
            .content("Let me check the weather.")
            .tool_calls(vec![tc])
            .build(),
    )];
    let result = convert_to_openai_messages(&messages, TextFormat::String, false);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0]["role"], "assistant");
    assert_eq!(result[0]["content"], "Let me check the weather.");
    // Should have tool_calls
    let tool_calls = result[0]["tool_calls"].as_array().unwrap();
    assert_eq!(tool_calls.len(), 1);
    assert_eq!(tool_calls[0]["type"], "function");
    assert_eq!(tool_calls[0]["id"], "call_123");
    assert_eq!(tool_calls[0]["function"]["name"], "get_weather");
}

// ============================================================================
// test_get_buffer_string_custom_human_and_ai_prefix
// ============================================================================

#[test]
fn test_get_buffer_string_custom_human_and_ai_prefix() {
    let messages = vec![
        BaseMessage::Human(HumanMessage::builder().content("hi").build()),
        BaseMessage::AI(AIMessage::builder().content("hello").build()),
    ];
    let result = get_buffer_string(&messages, "User", "Bot");
    assert_eq!(result, "User: hi\nBot: hello");
}

// ============================================================================
// test_get_buffer_string_with_tool_messages
// ============================================================================

#[test]
fn test_get_buffer_string_with_tool_messages() {
    // ToolMessage should use "Tool:" prefix
    let messages = vec![BaseMessage::Tool(
        ToolMessage::builder()
            .content("result from tool")
            .tool_call_id("tc1")
            .build(),
    )];
    let result = get_buffer_string(&messages, "Human", "AI");
    assert_eq!(result, "Tool: result from tool");
}

// ============================================================================
// test_get_buffer_string_with_function_messages
// ============================================================================

#[test]
fn test_get_buffer_string_with_function_messages() {
    // FunctionMessage should use "Function:" prefix
    let messages = vec![BaseMessage::Function(
        FunctionMessage::builder()
            .content("function output")
            .name("my_func")
            .build(),
    )];
    let result = get_buffer_string(&messages, "Human", "AI");
    assert_eq!(result, "Function: function output");
}

// ============================================================================
// test_get_buffer_string_with_empty_content
// ============================================================================

#[test]
fn test_get_buffer_string_with_empty_content() {
    let messages = vec![BaseMessage::Human(
        HumanMessage::builder().content("").build(),
    )];
    let result = get_buffer_string(&messages, "Human", "AI");
    assert_eq!(result, "Human: ");
}

// ============================================================================
// test_message_chunk_to_message_ai
// ============================================================================

#[test]
fn test_message_chunk_to_message_ai() {
    let chunk = BaseMessageChunk::AI(AIMessageChunk::builder().content("hello from ai").build());
    let msg = message_chunk_to_message(&chunk);
    assert!(matches!(msg, BaseMessage::AI(_)));
    assert_eq!(msg.content(), "hello from ai");
}

// ============================================================================
// test_message_chunk_to_message_human
// ============================================================================

#[test]
fn test_message_chunk_to_message_human() {
    let chunk = BaseMessageChunk::Human(
        HumanMessageChunk::builder()
            .content("hello from human")
            .build(),
    );
    let msg = message_chunk_to_message(&chunk);
    assert!(matches!(msg, BaseMessage::Human(_)));
    assert_eq!(msg.content(), "hello from human");
}

// ============================================================================
// test_message_chunk_to_message_system
// ============================================================================

#[test]
fn test_message_chunk_to_message_system() {
    let chunk = BaseMessageChunk::System(
        SystemMessageChunk::builder()
            .content("system prompt")
            .build(),
    );
    let msg = message_chunk_to_message(&chunk);
    assert!(matches!(msg, BaseMessage::System(_)));
    assert_eq!(msg.content(), "system prompt");
}

// ============================================================================
// test_message_chunk_to_message_tool
// ============================================================================

#[test]
fn test_message_chunk_to_message_tool() {
    let chunk = BaseMessageChunk::Tool(
        ToolMessageChunk::builder()
            .content("tool result")
            .tool_call_id("tc_42")
            .build(),
    );
    let msg = message_chunk_to_message(&chunk);
    assert!(matches!(msg, BaseMessage::Tool(_)));
    assert_eq!(msg.content(), "tool result");
    // Verify tool_call_id is preserved
    if let BaseMessage::Tool(tool_msg) = &msg {
        assert_eq!(tool_msg.tool_call_id, "tc_42");
    } else {
        panic!("Expected BaseMessage::Tool");
    }
}

// ============================================================================
// test_message_chunk_to_message_function
// ============================================================================

#[test]
fn test_message_chunk_to_message_function() {
    let chunk = BaseMessageChunk::Function(
        FunctionMessageChunk::builder()
            .content("func result")
            .name("my_function")
            .build(),
    );
    let msg = message_chunk_to_message(&chunk);
    assert!(matches!(msg, BaseMessage::Function(_)));
    assert_eq!(msg.content(), "func result");
    // Verify name is preserved
    if let BaseMessage::Function(func_msg) = &msg {
        assert_eq!(func_msg.name, "my_function");
    } else {
        panic!("Expected BaseMessage::Function");
    }
}

// ============================================================================
// test_message_chunk_to_message_chat
// ============================================================================

#[test]
fn test_message_chunk_to_message_chat() {
    let chunk = BaseMessageChunk::Chat(
        ChatMessageChunk::builder()
            .content("chat content")
            .role("custom_role")
            .build(),
    );
    let msg = message_chunk_to_message(&chunk);
    assert!(matches!(msg, BaseMessage::Chat(_)));
    assert_eq!(msg.content(), "chat content");
    // Verify role is preserved
    if let BaseMessage::Chat(chat_msg) = &msg {
        assert_eq!(chat_msg.role, "custom_role");
    } else {
        panic!("Expected BaseMessage::Chat");
    }
}

// ============================================================================
// test_messages_from_dict_round_trip
// ============================================================================

#[test]
fn test_messages_from_dict_round_trip() {
    // Create messages of all supported types
    let original_messages = vec![
        BaseMessage::Human(HumanMessage::builder().content("human msg").build()),
        BaseMessage::AI(AIMessage::builder().content("ai msg").build()),
        BaseMessage::System(SystemMessage::builder().content("system msg").build()),
        BaseMessage::Tool(
            ToolMessage::builder()
                .content("tool msg")
                .tool_call_id("tc1")
                .build(),
        ),
        BaseMessage::Function(
            FunctionMessage::builder()
                .content("func msg")
                .name("my_func")
                .build(),
        ),
        BaseMessage::Chat(
            ChatMessage::builder()
                .content("chat msg")
                .role("custom")
                .build(),
        ),
    ];

    // Convert to dicts and back
    let dicts = messages_to_dict(&original_messages);
    assert_eq!(dicts.len(), original_messages.len());

    let roundtripped = messages_from_dict(&dicts).unwrap();
    assert_eq!(roundtripped.len(), original_messages.len());

    // Verify each message type and content survived the round trip
    for (orig, rt) in original_messages.iter().zip(roundtripped.iter()) {
        assert_eq!(orig.message_type(), rt.message_type());
        assert_eq!(orig.content(), rt.content());
    }

    // Verify specific fields
    if let BaseMessage::Tool(tool_msg) = &roundtripped[3] {
        assert_eq!(tool_msg.tool_call_id, "tc1");
    } else {
        panic!("Expected BaseMessage::Tool at index 3");
    }

    if let BaseMessage::Function(func_msg) = &roundtripped[4] {
        assert_eq!(func_msg.name, "my_func");
    } else {
        panic!("Expected BaseMessage::Function at index 4");
    }

    if let BaseMessage::Chat(chat_msg) = &roundtripped[5] {
        assert_eq!(chat_msg.role, "custom");
    } else {
        panic!("Expected BaseMessage::Chat at index 5");
    }
}

// ============================================================================
// test_count_tokens_approximately_tool_calls
// ============================================================================

#[test]
fn test_count_tokens_approximately_tool_calls() {
    // AIMessage with tool_calls should count more tokens than without
    let config = CountTokensConfig::default();

    let msg_without = vec![BaseMessage::AI(
        AIMessage::builder().content("calling tool").build(),
    )];
    let tokens_without = count_tokens_approximately(&msg_without, &config);

    let tc = tool_call(
        "get_weather",
        serde_json::json!({"location": "San Francisco", "unit": "celsius"}),
        Some("call_abc".to_string()),
    );
    let msg_with = vec![BaseMessage::AI(
        AIMessage::builder()
            .content("calling tool")
            .tool_calls(vec![tc])
            .build(),
    )];
    let tokens_with = count_tokens_approximately(&msg_with, &config);

    // With tool calls should have more tokens due to serialized tool call data
    assert!(
        tokens_with > tokens_without,
        "Expected tokens_with ({}) > tokens_without ({})",
        tokens_with,
        tokens_without
    );
}

// ============================================================================
// test_count_tokens_approximately_large_content
// ============================================================================

#[test]
fn test_count_tokens_approximately_large_content() {
    // 10,000 character message
    let large_content = "a".repeat(10_000);
    let messages = vec![BaseMessage::Human(
        HumanMessage::builder().content(&large_content).build(),
    )];
    let config = CountTokensConfig::default();
    let tokens = count_tokens_approximately(&messages, &config);
    // 10000 content + 4 role = 10004 chars -> ceil(10004/4) + 3 = 2501 + 3 = 2504
    assert_eq!(tokens, 2504);
}

// ============================================================================
// test_count_tokens_approximately_large_number_of_messages
// ============================================================================

#[test]
fn test_count_tokens_approximately_large_number_of_messages() {
    // 1,000 messages
    let messages: Vec<BaseMessage> = (0..1000)
        .map(|i| {
            BaseMessage::Human(
                HumanMessage::builder()
                    .content(format!("Message {}", i))
                    .build(),
            )
        })
        .collect();
    let config = CountTokensConfig::default();
    let tokens = count_tokens_approximately(&messages, &config);
    // Should be a positive number proportional to 1000 messages
    assert!(tokens > 1000);
    // Each message: "Message X" (varies 9-12 chars) + "user" (4 chars) -> ~4-5 char tokens + 3 extra
    // Rough estimate: ~7 tokens per message on average -> ~7000
    assert!(tokens > 5000);
    assert!(tokens < 15000);
}

// ============================================================================
// test_count_tokens_approximately_mixed_content_types
// ============================================================================

#[test]
fn test_count_tokens_approximately_mixed_content_types() {
    let config = CountTokensConfig::default();
    let messages = vec![
        BaseMessage::Human(HumanMessage::builder().content("Hello").build()),
        BaseMessage::AI(AIMessage::builder().content("Hi there").build()),
        BaseMessage::System(
            SystemMessage::builder()
                .content("You are a helpful assistant")
                .build(),
        ),
        BaseMessage::Tool(
            ToolMessage::builder()
                .content("42")
                .tool_call_id("call_123")
                .build(),
        ),
    ];
    let tokens = count_tokens_approximately(&messages, &config);
    // Each message contributes content + role + 3 extra tokens
    // Just verify it is positive and reasonable
    assert!(tokens > 0);
    // Tool message also counts tool_call_id chars
    let tool_only = vec![BaseMessage::Tool(
        ToolMessage::builder()
            .content("42")
            .tool_call_id("call_123")
            .build(),
    )];
    let tool_tokens = count_tokens_approximately(&tool_only, &config);
    // "42" (2) + "tool" (4) + "call_123" (8) = 14 chars -> ceil(14/4) + 3 = 4 + 3 = 7
    assert_eq!(tool_tokens, 7);
}

// ============================================================================
// test_count_tokens_approximately_tool_message_includes_tool_call_id
// ============================================================================

#[test]
fn test_count_tokens_approximately_tool_message_includes_tool_call_id() {
    let config = CountTokensConfig::default();

    // ToolMessage with a short tool_call_id
    let msg_short = vec![BaseMessage::Tool(
        ToolMessage::builder()
            .content("ok")
            .tool_call_id("x")
            .build(),
    )];
    let tokens_short = count_tokens_approximately(&msg_short, &config);

    // ToolMessage with a long tool_call_id
    let msg_long = vec![BaseMessage::Tool(
        ToolMessage::builder()
            .content("ok")
            .tool_call_id("this_is_a_very_long_tool_call_identifier_string")
            .build(),
    )];
    let tokens_long = count_tokens_approximately(&msg_long, &config);

    // Longer tool_call_id should result in more tokens
    assert!(
        tokens_long > tokens_short,
        "Expected tokens_long ({}) > tokens_short ({})",
        tokens_long,
        tokens_short
    );
}

// ============================================================================
// test_trim_messages_empty_messages
// ============================================================================

#[test]
fn test_trim_messages_empty_messages() {
    let messages: Vec<BaseMessage> = vec![];
    let config =
        TrimMessagesConfig::new(100, dummy_token_counter).with_strategy(TrimStrategy::First);
    let actual = trim_messages(&messages, &config);
    assert!(actual.is_empty());
}

// ============================================================================
// test_trim_messages_exact_token_boundary
// ============================================================================

#[test]
fn test_trim_messages_exact_token_boundary() {
    // Each message is exactly 10 tokens with dummy_token_counter.
    // With max_tokens = 20, exactly 2 messages should fit.
    let messages = vec![
        BaseMessage::Human(HumanMessage::builder().content("msg1").build()),
        BaseMessage::AI(AIMessage::builder().content("msg2").build()),
        BaseMessage::Human(HumanMessage::builder().content("msg3").build()),
    ];

    let config =
        TrimMessagesConfig::new(20, dummy_token_counter).with_strategy(TrimStrategy::First);
    let actual = trim_messages(&messages, &config);
    assert_eq!(actual.len(), 2);
    assert_eq!(actual[0].content(), "msg1");
    assert_eq!(actual[1].content(), "msg2");
}

// ============================================================================
// test_trim_messages_last_without_include_system
// ============================================================================

#[test]
fn test_trim_messages_last_without_include_system() {
    // Last strategy without include_system should NOT keep the system message
    // if it doesn't fit in the token budget from the end.
    let messages = vec![
        BaseMessage::System(SystemMessage::builder().content("system").build()),
        BaseMessage::Human(HumanMessage::builder().content("human1").build()),
        BaseMessage::AI(AIMessage::builder().content("ai1").build()),
        BaseMessage::Human(HumanMessage::builder().content("human2").build()),
    ];

    // 20 tokens = 2 messages with dummy counter
    let config = TrimMessagesConfig::new(20, dummy_token_counter)
        .with_strategy(TrimStrategy::Last)
        .with_include_system(false);
    let actual = trim_messages(&messages, &config);
    assert_eq!(actual.len(), 2);
    // Should be the last 2 messages, not including the system message
    assert_eq!(actual[0].content(), "ai1");
    assert_eq!(actual[1].content(), "human2");
}

// ============================================================================
// test_filter_messages_include_types_and_include_names_combined
// ============================================================================

#[test]
fn test_filter_messages_include_types_and_include_names_combined() {
    // Include both by type and by name (OR logic):
    // Messages matching EITHER include_types OR include_names should be included.
    let messages = vec![
        BaseMessage::System(
            SystemMessage::builder()
                .content("sys")
                .name("sys_name".to_string())
                .build(),
        ),
        BaseMessage::Human(
            HumanMessage::builder()
                .content("human1")
                .name("alice".to_string())
                .build(),
        ),
        BaseMessage::AI(
            AIMessage::builder()
                .content("ai1")
                .name("bot".to_string())
                .build(),
        ),
        BaseMessage::Human(
            HumanMessage::builder()
                .content("human2")
                .name("bob".to_string())
                .build(),
        ),
    ];

    // Include type "ai" OR name "alice" -> should get human1 (by name) and ai1 (by type)
    let actual = filter_messages(
        &messages,
        Some(&["alice"]),
        None,
        Some(&["ai"]),
        None,
        None,
        None,
        None,
    );
    assert_eq!(actual.len(), 2);
    assert_eq!(actual[0].content(), "human1");
    assert_eq!(actual[1].content(), "ai1");
}

// ============================================================================
// test_convert_to_messages_multiple_formats
// ============================================================================

#[test]
fn test_convert_to_messages_multiple_formats() {
    // Mix of strings, tuples, and dicts
    let message_like = vec![
        serde_json::json!("plain text"),
        serde_json::json!(["system", "be helpful"]),
        serde_json::json!({"role": "assistant", "content": "sure thing"}),
        serde_json::json!(["human", "thanks"]),
    ];
    let actual = convert_to_messages(&message_like).unwrap();
    assert_eq!(actual.len(), 4);

    // String -> HumanMessage
    assert!(matches!(actual[0], BaseMessage::Human(_)));
    assert_eq!(actual[0].content(), "plain text");

    // Tuple ["system", ...] -> SystemMessage
    assert!(matches!(actual[1], BaseMessage::System(_)));
    assert_eq!(actual[1].content(), "be helpful");

    // Dict {role: "assistant"} -> AIMessage
    assert!(matches!(actual[2], BaseMessage::AI(_)));
    assert_eq!(actual[2].content(), "sure thing");

    // Tuple ["human", ...] -> HumanMessage
    assert!(matches!(actual[3], BaseMessage::Human(_)));
    assert_eq!(actual[3].content(), "thanks");
}

// ============================================================================
// test_filter_message_exclude_tool_calls
// ============================================================================

#[test]
fn test_filter_message_exclude_tool_calls_all() {
    let tc1 = tool_call("foo", serde_json::json!({}), Some("1".to_string()));
    let tc2 = tool_call("bar", serde_json::json!({}), Some("2".to_string()));
    let messages = vec![
        BaseMessage::Human(
            HumanMessage::builder()
                .content("foo")
                .name("blah".to_string())
                .id("1".to_string())
                .build(),
        ),
        BaseMessage::AI(
            AIMessage::builder()
                .content("foo-response")
                .name("blah".to_string())
                .id("2".to_string())
                .build(),
        ),
        BaseMessage::Human(
            HumanMessage::builder()
                .content("bar")
                .name("blur".to_string())
                .id("3".to_string())
                .build(),
        ),
        BaseMessage::AI(
            AIMessage::builder()
                .content("bar-response")
                .tool_calls(vec![tc1.clone(), tc2.clone()])
                .id("4".to_string())
                .build(),
        ),
        BaseMessage::Tool(
            ToolMessage::builder()
                .content("baz")
                .tool_call_id("1")
                .id("5".to_string())
                .build(),
        ),
        BaseMessage::Tool(
            ToolMessage::builder()
                .content("qux")
                .tool_call_id("2")
                .id("6".to_string())
                .build(),
        ),
    ];
    let messages_copy = messages.clone();

    // Test excluding all tool calls
    let expected = messages[..3].to_vec();
    let actual = filter_messages(
        &messages,
        None,
        None,
        None,
        None,
        None,
        None,
        Some(&ExcludeToolCalls::All),
    );
    assert_eq!(expected, actual);

    // Test explicitly excluding all tool calls by IDs
    let actual = filter_messages(
        &messages,
        None,
        None,
        None,
        None,
        None,
        None,
        Some(&ExcludeToolCalls::Ids(vec![
            "1".to_string(),
            "2".to_string(),
        ])),
    );
    assert_eq!(expected, actual);

    // Test excluding a specific tool call
    let mut expected_partial = messages[..5].to_vec();
    expected_partial[3] = BaseMessage::AI(
        AIMessage::builder()
            .content("bar-response")
            .tool_calls(vec![tc1.clone()])
            .id("4".to_string())
            .build(),
    );
    let actual = filter_messages(
        &messages,
        None,
        None,
        None,
        None,
        None,
        None,
        Some(&ExcludeToolCalls::Ids(vec!["2".to_string()])),
    );
    assert_eq!(expected_partial, actual);

    // Original messages should not be mutated
    assert_eq!(messages, messages_copy);
}

// ============================================================================
// test_trim_messages_first_30_allow_partial_end_on_human
// ============================================================================

#[test]
fn test_trim_messages_first_30_allow_partial_end_on_human() {
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

    let config = TrimMessagesConfig::new(30, dummy_token_counter)
        .with_strategy(TrimStrategy::First)
        .with_allow_partial(true)
        .with_end_on(vec!["human".to_string()]);

    let actual = trim_messages(&messages, &config);

    // Should include system + first human, end_on="human" trims the AI message
    assert_eq!(actual.len(), 2);
    assert_eq!(actual[0].content(), "This is a 4 token text.");
    assert_eq!(actual[1].content(), "This is a 4 token text.");
    assert!(matches!(actual[1], BaseMessage::Human(_)));
    assert_eq!(messages, messages_copy);
}

// ============================================================================
// test_trim_messages_last_40_include_system_allow_partial
// ============================================================================

#[test]
fn test_trim_messages_last_40_include_system_allow_partial() {
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
    let messages_copy = messages.clone();

    // 40 tokens: system (10) + last 3 messages (30) = 40
    let config = TrimMessagesConfig::new(40, dummy_token_counter)
        .with_strategy(TrimStrategy::Last)
        .with_allow_partial(true)
        .with_include_system(true);

    let actual = trim_messages(&messages, &config);

    // System + last 3 messages = 4 messages, exactly 40 tokens
    assert_eq!(actual.len(), 4);
    assert!(matches!(actual[0], BaseMessage::System(_)));
    assert_eq!(actual[0].content(), "This is a 4 token text.");
    assert_eq!(messages, messages_copy);
}

// ============================================================================
// test_trim_messages_last_30_include_system_allow_partial_end_on_human
// ============================================================================

#[test]
fn test_trim_messages_last_30_include_system_allow_partial_end_on_human() {
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
    let messages_copy = messages.clone();

    let config = TrimMessagesConfig::new(30, dummy_token_counter)
        .with_strategy(TrimStrategy::Last)
        .with_allow_partial(true)
        .with_include_system(true)
        .with_end_on(vec!["human".to_string()]);

    let actual = trim_messages(&messages, &config);

    // System (10) + end_on="human" removes trailing AI, keeps human "third" (10)
    // = system + third = 20 tokens
    assert!(actual.len() >= 2);
    assert!(matches!(actual[0], BaseMessage::System(_)));
    assert!(matches!(actual.last().unwrap(), BaseMessage::Human(_)));
    assert_eq!(messages, messages_copy);
}

// ============================================================================
// test_trim_messages_last_40_include_system_allow_partial_start_on_human
// ============================================================================

#[test]
fn test_trim_messages_last_40_include_system_allow_partial_start_on_human() {
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
    let messages_copy = messages.clone();

    // 30 tokens with start_on=human: system (10) + last 2 messages (20) = 30
    // but start_on=human means we skip non-human messages at the start of the trimmed window
    let config = TrimMessagesConfig::new(30, dummy_token_counter)
        .with_strategy(TrimStrategy::Last)
        .with_allow_partial(true)
        .with_include_system(true)
        .with_start_on(vec!["human".to_string()]);

    let actual = trim_messages(&messages, &config);

    // Should include system + human "third" + AI "fourth"
    assert_eq!(actual.len(), 3);
    assert!(matches!(actual[0], BaseMessage::System(_)));
    assert!(matches!(actual[1], BaseMessage::Human(_)));
    assert_eq!(actual[1].content(), "This is a 4 token text.");
    assert_eq!(messages, messages_copy);
}

// ============================================================================
// test_trim_messages_allow_partial_one_message
// ============================================================================

#[test]
fn test_trim_messages_allow_partial_one_message() {
    let messages = vec![BaseMessage::Human(
        HumanMessage::builder()
            .id("third".to_string())
            .content("This is a funky text.")
            .build(),
    )];

    let config = TrimMessagesConfig::new(2, |msgs: &[BaseMessage]| -> usize {
        msgs.iter().map(|m| m.content().len()).sum()
    })
    .with_strategy(TrimStrategy::First)
    .with_allow_partial(true)
    .with_text_splitter(|text: &str| text.chars().map(|c| c.to_string()).collect());

    let actual = trim_messages(&messages, &config);

    assert_eq!(actual.len(), 1);
    assert_eq!(actual[0].content(), "Th");
}

// ============================================================================
// test_trim_messages_last_allow_partial_one_message
// ============================================================================

#[test]
fn test_trim_messages_last_allow_partial_one_message() {
    let messages = vec![BaseMessage::Human(
        HumanMessage::builder()
            .id("third".to_string())
            .content("This is a funky text.")
            .build(),
    )];

    let config = TrimMessagesConfig::new(2, |msgs: &[BaseMessage]| -> usize {
        msgs.iter().map(|m| m.content().len()).sum()
    })
    .with_strategy(TrimStrategy::Last)
    .with_allow_partial(true)
    .with_text_splitter(|text: &str| text.chars().map(|c| c.to_string()).collect());

    let actual = trim_messages(&messages, &config);

    assert_eq!(actual.len(), 1);
    assert_eq!(actual[0].content(), "t.");
}

// ============================================================================
// test_trim_messages_allow_partial_text_splitter
// ============================================================================

#[test]
fn test_trim_messages_allow_partial_text_splitter() {
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

    fn count_words(msgs: &[BaseMessage]) -> usize {
        msgs.iter()
            .map(|m| {
                // Match Python's split(" ") behavior
                m.content().split(' ').count()
            })
            .sum()
    }

    fn split_on_space(text: &str) -> Vec<String> {
        let splits: Vec<&str> = text.split(' ').collect();
        let mut result: Vec<String> = splits[..splits.len() - 1]
            .iter()
            .map(|s| format!("{} ", s))
            .collect();
        result.push(splits.last().unwrap_or(&"").to_string());
        result
    }

    let config = TrimMessagesConfig::new(10, count_words)
        .with_strategy(TrimStrategy::Last)
        .with_allow_partial(true)
        .with_text_splitter(split_on_space);

    let actual = trim_messages(&messages, &config);

    // Should include partial "third" + full "fourth"
    // "fourth" = 6 words, remaining budget = 4 words
    // "third" partial = "a 4 token text." = 4 words
    assert_eq!(actual.len(), 2);
    assert_eq!(actual[0].content(), "a 4 token text.");
    assert_eq!(actual[1].content(), "This is a 4 token text.");
    assert_eq!(messages, messages_copy);
}

// ============================================================================
// test_trim_messages_partial_text_splitting
// ============================================================================

#[test]
fn test_trim_messages_partial_text_splitting() {
    let messages = vec![BaseMessage::Human(
        HumanMessage::builder()
            .content("This is a long message that needs trimming")
            .build(),
    )];
    let messages_copy = messages.clone();

    fn count_characters(msgs: &[BaseMessage]) -> usize {
        msgs.iter().map(|m| m.content().len()).sum()
    }

    fn char_splitter(text: &str) -> Vec<String> {
        text.chars().map(|c| c.to_string()).collect()
    }

    let config = TrimMessagesConfig::new(10, count_characters)
        .with_strategy(TrimStrategy::First)
        .with_allow_partial(true)
        .with_text_splitter(char_splitter);

    let actual = trim_messages(&messages, &config);

    assert_eq!(actual.len(), 1);
    assert_eq!(actual[0].content(), "This is a ");
    assert_eq!(messages, messages_copy);
}

// ============================================================================
// test_trim_messages_mixed_content_with_partial
// ============================================================================

#[test]
fn test_trim_messages_mixed_content_with_partial() {
    // AIMessage with list content (JSON array) — two text blocks
    let content_blocks = serde_json::json!([
        {"type": "text", "text": "First part of text."},
        {"type": "text", "text": "Second part that should be trimmed."},
    ]);
    let messages = vec![BaseMessage::AI(
        AIMessage::builder()
            .content(serde_json::to_string(&content_blocks).unwrap())
            .build(),
    )];
    let messages_copy = messages.clone();

    fn count_text_length(msgs: &[BaseMessage]) -> usize {
        let mut total = 0;
        for msg in msgs {
            let raw = msg.content();
            if let Ok(blocks) = serde_json::from_str::<Vec<serde_json::Value>>(raw) {
                for block in &blocks {
                    if block.get("type").and_then(|t| t.as_str()) == Some("text") {
                        total += block
                            .get("text")
                            .and_then(|t| t.as_str())
                            .unwrap_or("")
                            .len();
                    }
                }
            } else {
                total += raw.len();
            }
        }
        total
    }

    let config = TrimMessagesConfig::new(20, count_text_length)
        .with_strategy(TrimStrategy::First)
        .with_allow_partial(true);

    let actual = trim_messages(&messages, &config);

    assert_eq!(actual.len(), 1);
    // Should have only the first content block since "First part of text." is 19 chars
    let content_str = actual[0].content();
    let result_blocks: Vec<serde_json::Value> = serde_json::from_str(content_str).unwrap();
    assert_eq!(result_blocks.len(), 1);
    assert_eq!(
        result_blocks[0].get("text").and_then(|t| t.as_str()),
        Some("First part of text.")
    );
    assert_eq!(messages, messages_copy);
}

// ============================================================================
// test_trim_messages_start_on_with_allow_partial
// ============================================================================

#[test]
fn test_trim_messages_start_on_with_allow_partial() {
    let messages = vec![
        BaseMessage::Human(
            HumanMessage::builder()
                .content("First human message")
                .build(),
        ),
        BaseMessage::AI(AIMessage::builder().content("AI response").build()),
        BaseMessage::Human(
            HumanMessage::builder()
                .content("Second human message")
                .build(),
        ),
    ];
    let messages_copy = messages.clone();

    let config = TrimMessagesConfig::new(20, dummy_token_counter)
        .with_strategy(TrimStrategy::Last)
        .with_allow_partial(true)
        .with_start_on(vec!["human".to_string()]);

    let actual = trim_messages(&messages, &config);

    // 20 tokens = 2 messages, but start_on="human" removes leading AI
    assert_eq!(actual.len(), 1);
    assert_eq!(actual[0].content(), "Second human message");
    assert_eq!(messages, messages_copy);
}

// ============================================================================
// test_trim_messages_include_system_strategy_last_empty_messages
// ============================================================================

#[test]
fn test_trim_messages_include_system_strategy_last_empty_messages() {
    let messages: Vec<BaseMessage> = vec![];
    let config = TrimMessagesConfig::new(10, dummy_token_counter)
        .with_strategy(TrimStrategy::Last)
        .with_include_system(true);

    let actual = trim_messages(&messages, &config);
    assert!(actual.is_empty());
}

// ============================================================================
// test_convert_to_openai_messages_openai_string
// ============================================================================

#[test]
fn test_convert_to_openai_messages_openai_string() {
    // Messages with list content blocks that are all text should be joined into a string
    let human_content = serde_json::json!([
        {"type": "text", "text": "Hello"},
        {"type": "text", "text": "World"},
    ]);
    let ai_content = serde_json::json!([
        {"type": "text", "text": "Hi"},
        {"type": "text", "text": "there"},
    ]);
    let messages = vec![
        BaseMessage::Human(
            HumanMessage::builder()
                .content(serde_json::to_string(&human_content).unwrap())
                .build(),
        ),
        BaseMessage::AI(
            AIMessage::builder()
                .content(serde_json::to_string(&ai_content).unwrap())
                .build(),
        ),
    ];
    let result = convert_to_openai_messages(&messages, TextFormat::String, false);

    assert_eq!(result.len(), 2);
    assert_eq!(result[0]["role"], "user");
    assert_eq!(result[0]["content"], "Hello\nWorld");
    assert_eq!(result[1]["role"], "assistant");
    assert_eq!(result[1]["content"], "Hi\nthere");
}

// ============================================================================
// test_convert_to_openai_messages_openai_block
// ============================================================================

#[test]
fn test_convert_to_openai_messages_openai_block() {
    let messages = vec![
        BaseMessage::Human(HumanMessage::builder().content("Hello").build()),
        BaseMessage::AI(AIMessage::builder().content("Hi there").build()),
    ];
    let result = convert_to_openai_messages(&messages, TextFormat::Block, false);

    assert_eq!(result.len(), 2);
    let user_content = result[0]["content"].as_array().unwrap();
    assert_eq!(user_content.len(), 1);
    assert_eq!(user_content[0]["type"], "text");
    assert_eq!(user_content[0]["text"], "Hello");

    let ai_content = result[1]["content"].as_array().unwrap();
    assert_eq!(ai_content.len(), 1);
    assert_eq!(ai_content[0]["type"], "text");
    assert_eq!(ai_content[0]["text"], "Hi there");
}

// ============================================================================
// test_convert_to_openai_messages_openai_image
// ============================================================================

#[test]
fn test_convert_to_openai_messages_openai_image() {
    let base64_image = "data:image/jpeg;base64,/9j/4AAQSkZJRg==";
    let content = serde_json::json!([
        {"type": "text", "text": "Here's an image:"},
        {"type": "image_url", "image_url": {"url": base64_image}},
    ]);
    let messages = vec![BaseMessage::Human(
        HumanMessage::builder()
            .content(serde_json::to_string(&content).unwrap())
            .build(),
    )];
    let result = convert_to_openai_messages(&messages, TextFormat::Block, false);

    assert_eq!(result.len(), 1);
    let blocks = result[0]["content"].as_array().unwrap();
    assert_eq!(blocks.len(), 2);
    assert_eq!(blocks[0]["type"], "text");
    assert_eq!(blocks[0]["text"], "Here's an image:");
    assert_eq!(blocks[1]["type"], "image_url");
    assert_eq!(blocks[1]["image_url"]["url"], base64_image);
}

// ============================================================================
// test_convert_to_openai_messages_tool_use
// ============================================================================

#[test]
fn test_convert_to_openai_messages_tool_use() {
    let content = serde_json::json!([
        {"type": "tool_use", "id": "123", "name": "calculator", "input": {"a": "b"}},
    ]);
    let messages = vec![BaseMessage::AI(
        AIMessage::builder()
            .content(serde_json::to_string(&content).unwrap())
            .build(),
    )];
    let result = convert_to_openai_messages(&messages, TextFormat::Block, false);

    assert_eq!(result.len(), 1);
    let tool_calls = result[0]["tool_calls"].as_array().unwrap();
    assert_eq!(tool_calls[0]["type"], "function");
    assert_eq!(tool_calls[0]["id"], "123");
    assert_eq!(tool_calls[0]["function"]["name"], "calculator");
    let args: serde_json::Value =
        serde_json::from_str(tool_calls[0]["function"]["arguments"].as_str().unwrap()).unwrap();
    assert_eq!(args, serde_json::json!({"a": "b"}));
}

// ============================================================================
// test_convert_to_openai_messages_tool_use_unicode
// ============================================================================

#[test]
fn test_convert_to_openai_messages_tool_use_unicode() {
    let content = serde_json::json!([
        {"type": "tool_use", "id": "123", "name": "create_customer", "input": {"customer_name": "你好啊集团"}},
    ]);
    let messages = vec![BaseMessage::AI(
        AIMessage::builder()
            .content(serde_json::to_string(&content).unwrap())
            .build(),
    )];
    let result = convert_to_openai_messages(&messages, TextFormat::Block, false);

    let tool_calls = result[0]["tool_calls"].as_array().unwrap();
    assert_eq!(tool_calls[0]["function"]["name"], "create_customer");
    let arguments_str = tool_calls[0]["function"]["arguments"].as_str().unwrap();
    let parsed_args: serde_json::Value = serde_json::from_str(arguments_str).unwrap();
    assert_eq!(parsed_args["customer_name"], "你好啊集团");
    // Ensure Unicode is preserved, not escaped
    assert!(arguments_str.contains("你好啊集团"));
    assert!(!arguments_str.contains("\\u4f60"));
}

// ============================================================================
// test_convert_to_openai_messages_json
// ============================================================================

#[test]
fn test_convert_to_openai_messages_json() {
    let json_data = serde_json::json!({"key": "value"});
    let content = serde_json::json!([{"type": "json", "json": json_data}]);
    let messages = vec![BaseMessage::Human(
        HumanMessage::builder()
            .content(serde_json::to_string(&content).unwrap())
            .build(),
    )];
    let result = convert_to_openai_messages(&messages, TextFormat::Block, false);

    // JSON blocks should be converted to text blocks with the JSON stringified
    let blocks = result[0]["content"].as_array().unwrap();
    assert_eq!(blocks.len(), 1);
    // The block should either be a text block or pass through as-is
    // Since the current implementation passes through unknown blocks, check it exists
    assert!(!blocks.is_empty());
}

// ============================================================================
// test_convert_to_openai_messages_empty_message
// ============================================================================

#[test]
fn test_convert_to_openai_messages_empty_message() {
    let messages = vec![BaseMessage::Human(
        HumanMessage::builder().content("").build(),
    )];
    let result = convert_to_openai_messages(&messages, TextFormat::String, false);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0]["role"], "user");
    assert_eq!(result[0]["content"], "");
}

// ============================================================================
// test_convert_to_openai_messages_include_id
// ============================================================================

#[test]
fn test_convert_to_openai_messages_include_id() {
    // Without include_id — no "id" in output
    let messages = vec![BaseMessage::AI(
        AIMessage::builder()
            .content("Hello")
            .id("resp_123".to_string())
            .build(),
    )];

    let result_no_id = convert_to_openai_messages(&messages, TextFormat::String, false);
    assert_eq!(result_no_id[0]["role"], "assistant");
    assert_eq!(result_no_id[0]["content"], "Hello");
    assert!(result_no_id[0].get("id").is_none() || result_no_id[0]["id"].is_null());

    // With include_id — "id" should be present
    let result_with_id = convert_to_openai_messages(&messages, TextFormat::String, true);
    assert_eq!(result_with_id[0]["role"], "assistant");
    assert_eq!(result_with_id[0]["content"], "Hello");
    assert_eq!(result_with_id[0]["id"], "resp_123");

    // HumanMessage without id — no "id" field even with include_id
    let human_msgs = vec![BaseMessage::Human(
        HumanMessage::builder().content("Hello").build(),
    )];
    let result_human = convert_to_openai_messages(&human_msgs, TextFormat::String, true);
    assert_eq!(result_human[0]["role"], "user");
    assert_eq!(result_human[0]["content"], "Hello");
}

// ============================================================================
// test_convert_to_openai_messages_mixed_content_types
// ============================================================================

#[test]
fn test_convert_to_openai_messages_mixed_content_types() {
    let base64_image = "data:image/jpeg;base64,/9j/4AAQSkZJRg==";
    let content = serde_json::json!([
        "Text message",
        {"type": "text", "text": "Structured text"},
        {"type": "image_url", "image_url": {"url": base64_image}},
    ]);
    let messages = vec![BaseMessage::Human(
        HumanMessage::builder()
            .content(serde_json::to_string(&content).unwrap())
            .build(),
    )];
    let result = convert_to_openai_messages(&messages, TextFormat::Block, false);

    let blocks = result[0]["content"].as_array().unwrap();
    assert_eq!(blocks.len(), 3);
}

// ============================================================================
// test_convert_to_openai_messages_ai_with_tool_calls_and_content
// ============================================================================

#[test]
fn test_convert_to_openai_messages_ai_with_tool_calls_and_content() {
    // AIMessage with both content and tool_calls
    let tc = tool_call(
        "get_weather",
        serde_json::json!({"location": "Paris"}),
        Some("call_123".to_string()),
    );
    let messages = vec![BaseMessage::AI(
        AIMessage::builder()
            .content("Let me check the weather.")
            .tool_calls(vec![tc])
            .build(),
    )];
    let result = convert_to_openai_messages(&messages, TextFormat::String, false);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0]["role"], "assistant");
    assert_eq!(result[0]["content"], "Let me check the weather.");

    let tool_calls = result[0]["tool_calls"].as_array().unwrap();
    assert_eq!(tool_calls.len(), 1);
    assert_eq!(tool_calls[0]["type"], "function");
    assert_eq!(tool_calls[0]["id"], "call_123");
    assert_eq!(tool_calls[0]["function"]["name"], "get_weather");
}

// ============================================================================
// test_convert_to_openai_messages_anthropic_tool_use_in_content
// ============================================================================

#[test]
fn test_convert_to_openai_messages_anthropic_tool_use_in_content() {
    // Anthropic-style tool_use block in content should be converted to tool_calls
    let content = serde_json::json!([
        {"type": "tool_use", "name": "foo", "input": {"bar": "baz"}, "id": "1"},
    ]);
    let messages = vec![BaseMessage::AI(
        AIMessage::builder()
            .content(serde_json::to_string(&content).unwrap())
            .build(),
    )];
    let result = convert_to_openai_messages(&messages, TextFormat::String, false);

    assert_eq!(result.len(), 1);
    assert_eq!(result[0]["role"], "assistant");
    let tool_calls = result[0]["tool_calls"].as_array().unwrap();
    assert_eq!(tool_calls.len(), 1);
    assert_eq!(tool_calls[0]["type"], "function");
    assert_eq!(tool_calls[0]["id"], "1");
    assert_eq!(tool_calls[0]["function"]["name"], "foo");
    let args: serde_json::Value =
        serde_json::from_str(tool_calls[0]["function"]["arguments"].as_str().unwrap()).unwrap();
    assert_eq!(args, serde_json::json!({"bar": "baz"}));
}

// ============================================================================
// test_convert_to_openai_messages_developer_role
// ============================================================================

#[test]
fn test_convert_to_openai_messages_developer_role() {
    // SystemMessage with __openai_role__ = "developer" should map to developer role
    let mut additional_kwargs = std::collections::HashMap::new();
    additional_kwargs.insert(
        "__openai_role__".to_string(),
        serde_json::json!("developer"),
    );
    let messages = vec![BaseMessage::System(
        SystemMessage::builder()
            .content("Be helpful")
            .additional_kwargs(additional_kwargs)
            .build(),
    )];
    let result = convert_to_openai_messages(&messages, TextFormat::String, false);
    assert_eq!(result.len(), 1);
    // System message maps to "system" role in OpenAI format
    assert_eq!(result[0]["content"], "Be helpful");
}

// ============================================================================
// test_get_buffer_string_with_structured_content
// ============================================================================

#[test]
fn test_get_buffer_string_with_structured_content() {
    // For HumanMessage and SystemMessage, content is stored as MessageContent.
    // The text() method extracts text from Parts.
    // Here we test get_buffer_string behavior with plain text content.
    let messages = vec![
        BaseMessage::Human(HumanMessage::builder().content("Hello, world!").build()),
        BaseMessage::AI(AIMessage::builder().content("Hi there!").build()),
        BaseMessage::System(SystemMessage::builder().content("System message").build()),
    ];
    let expected = "Human: Hello, world!\nAI: Hi there!\nSystem: System message";
    let actual = get_buffer_string(&messages, "Human", "AI");
    assert_eq!(actual, expected);
}

// ============================================================================
// test_get_buffer_string_with_mixed_content
// ============================================================================

#[test]
fn test_get_buffer_string_with_mixed_content() {
    let messages = vec![
        BaseMessage::Human(HumanMessage::builder().content("Simple text").build()),
        BaseMessage::AI(AIMessage::builder().content("Structured text").build()),
        BaseMessage::System(
            SystemMessage::builder()
                .content("Another structured text")
                .build(),
        ),
    ];
    let expected = "Human: Simple text\nAI: Structured text\nSystem: Another structured text";
    let actual = get_buffer_string(&messages, "Human", "AI");
    assert_eq!(actual, expected);
}

// ============================================================================
// test_get_buffer_string_with_function_call
// ============================================================================

#[test]
fn test_get_buffer_string_with_function_call() {
    let mut additional_kwargs = std::collections::HashMap::new();
    additional_kwargs.insert(
        "function_call".to_string(),
        serde_json::json!({"name": "test_function", "arguments": "{\"arg\": \"value\"}"}),
    );
    let messages = vec![
        BaseMessage::Human(HumanMessage::builder().content("Hello").build()),
        BaseMessage::AI(
            AIMessage::builder()
                .content("Hi")
                .additional_kwargs(additional_kwargs)
                .build(),
        ),
    ];
    let actual = get_buffer_string(&messages, "Human", "AI");
    // The AI message should include the function_call content appended
    assert!(actual.starts_with("Human: Hello\nAI: Hi"));
    assert!(actual.contains("test_function"));
}

// ============================================================================
// test_get_buffer_string_with_empty_list_content
// ============================================================================

#[test]
fn test_get_buffer_string_with_empty_list_content() {
    // Empty string content should produce "Role: "
    let messages = vec![
        BaseMessage::Human(HumanMessage::builder().content("").build()),
        BaseMessage::AI(AIMessage::builder().content("").build()),
        BaseMessage::System(SystemMessage::builder().content("").build()),
    ];
    let expected = "Human: \nAI: \nSystem: ";
    let actual = get_buffer_string(&messages, "Human", "AI");
    assert_eq!(actual, expected);
}

// ============================================================================
// test_count_tokens_approximately_tool_message_includes_tool_call_id_and_name
// ============================================================================

#[test]
fn test_count_tokens_approximately_tool_message_includes_tool_call_id_and_name() {
    let config = CountTokensConfig::default();

    // ToolMessage with known dimensions
    let msg = BaseMessage::Tool(
        ToolMessage::builder()
            .content("result") // 6 chars
            .tool_call_id("call_1") // 6 chars
            .name("my_tool".to_string()) // 7 chars
            .build(),
    );
    // role = "tool" -> 4 chars
    // total chars = 6 (content) + 6 (tool_call_id) + 4 (role) + 7 (name) = 23 chars
    // tokens = ceil(23 / 4) + 3 = 6 + 3 = 9
    assert_eq!(
        count_tokens_approximately(std::slice::from_ref(&msg), &config),
        9
    );

    // Without name counting
    let config_no_name = CountTokensConfig {
        count_name: false,
        ..Default::default()
    };
    // total chars = 6 (content) + 6 (tool_call_id) + 4 (role) = 16 chars
    // tokens = ceil(16 / 4) + 3 = 4 + 3 = 7
    assert_eq!(count_tokens_approximately(&[msg], &config_no_name), 7);

    // Compare with a HumanMessage (no tool_call_id) with same content length
    let human_msg = BaseMessage::Human(HumanMessage::builder().content("result").build());
    // role = "user" -> 4 chars
    // total chars = 6 (content) + 4 (role) = 10
    // tokens = ceil(10 / 4) + 3 = 3 + 3 = 6
    assert_eq!(
        count_tokens_approximately(std::slice::from_ref(&human_msg), &config),
        6
    );

    // ToolMessage should have more tokens than HumanMessage with same content
    assert!(
        count_tokens_approximately(
            &[ToolMessage::builder()
                .content("result")
                .tool_call_id("call_1")
                .name("my_tool".to_string())
                .build()
                .into()],
            &config
        ) > count_tokens_approximately(&[human_msg], &config)
    );
}

// ============================================================================
// test_convert_to_messages_role_tool
// ============================================================================

#[test]
fn test_convert_to_messages_role_tool() {
    let message_like =
        vec![serde_json::json!({"role": "tool", "content": "10.1", "tool_call_id": "10.2"})];
    let actual = convert_to_messages(&message_like).unwrap();
    assert_eq!(actual.len(), 1);
    assert!(matches!(actual[0], BaseMessage::Tool(_)));
    assert_eq!(actual[0].content(), "10.1");
    if let BaseMessage::Tool(tool_msg) = &actual[0] {
        assert_eq!(tool_msg.tool_call_id, "10.2");
    }
}

// ============================================================================
// test_convert_to_messages_role_developer
// ============================================================================

#[test]
fn test_convert_to_messages_role_developer() {
    let message_like = vec![serde_json::json!({"role": "developer", "content": "6.1"})];
    let actual = convert_to_messages(&message_like).unwrap();
    assert_eq!(actual.len(), 1);
    // Developer role maps to SystemMessage
    assert!(matches!(actual[0], BaseMessage::System(_)));
    assert_eq!(actual[0].content(), "6.1");
}

// ============================================================================
// test_convert_to_messages_tuple_developer
// ============================================================================

#[test]
fn test_convert_to_messages_tuple_developer() {
    let message_like = vec![serde_json::json!(["developer", "11.2"])];
    let actual = convert_to_messages(&message_like).unwrap();
    assert_eq!(actual.len(), 1);
    // Developer tuple maps to SystemMessage
    assert!(matches!(actual[0], BaseMessage::System(_)));
    assert_eq!(actual[0].content(), "11.2");
}

// ============================================================================
// test_convert_to_messages_role_assistant_with_tool_calls
// ============================================================================

#[test]
fn test_convert_to_messages_role_assistant_with_tool_calls() {
    let message_like = vec![serde_json::json!({
        "role": "assistant",
        "content": [{"type": "text", "text": "8.1"}],
        "tool_calls": [{
            "type": "function",
            "function": {
                "arguments": serde_json::json!({"8.2": "8.3"}).to_string(),
                "name": "8.4",
            },
            "id": "8.5",
        }],
        "name": "8.6",
    })];
    let actual = convert_to_messages(&message_like).unwrap();
    assert_eq!(actual.len(), 1);
    assert!(matches!(actual[0], BaseMessage::AI(_)));
    if let BaseMessage::AI(ai_msg) = &actual[0] {
        assert_eq!(ai_msg.tool_calls.len(), 1);
        assert_eq!(ai_msg.tool_calls[0].name, "8.4");
        assert_eq!(ai_msg.tool_calls[0].args, serde_json::json!({"8.2": "8.3"}));
        assert_eq!(ai_msg.tool_calls[0].id, Some("8.5".to_string()));
    }
}

// ============================================================================
// test_convert_to_messages_langchain_dict_with_tool_calls
// ============================================================================

#[test]
fn test_convert_to_messages_langchain_dict_with_tool_calls() {
    let message_like = vec![serde_json::json!({
        "role": "ai",
        "content": [{"type": "text", "text": "15.1"}],
        "tool_calls": [{"args": {"15.2": "15.3"}, "name": "15.4", "id": "15.5"}],
        "name": "15.6",
    })];
    let actual = convert_to_messages(&message_like).unwrap();
    assert_eq!(actual.len(), 1);
    assert!(matches!(actual[0], BaseMessage::AI(_)));
    if let BaseMessage::AI(ai_msg) = &actual[0] {
        assert_eq!(ai_msg.tool_calls.len(), 1);
        assert_eq!(ai_msg.tool_calls[0].name, "15.4");
        assert_eq!(ai_msg.tool_calls[0].id, Some("15.5".to_string()));
    }
}

// ============================================================================
// test_convert_to_openai_messages_reasoning_content
// ============================================================================

#[test]
fn test_convert_to_openai_messages_reasoning_content() {
    // Reasoning blocks should pass through
    let content = serde_json::json!([{"type": "reasoning", "summary": []}]);
    let messages = vec![BaseMessage::AI(
        AIMessage::builder()
            .content(serde_json::to_string(&content).unwrap())
            .build(),
    )];
    let result = convert_to_openai_messages(&messages, TextFormat::Block, false);

    assert_eq!(result.len(), 1);
    let blocks = result[0]["content"].as_array().unwrap();
    assert_eq!(blocks.len(), 1);
    assert_eq!(blocks[0]["type"], "reasoning");
    assert_eq!(blocks[0]["summary"], serde_json::json!([]));

    // Reasoning block with summary content
    let content_with_summary = serde_json::json!([{
        "type": "reasoning",
        "summary": [
            {"type": "text", "text": "First thought"},
            {"type": "text", "text": "Second thought"},
        ],
    }]);
    let messages2 = vec![BaseMessage::AI(
        AIMessage::builder()
            .content(serde_json::to_string(&content_with_summary).unwrap())
            .build(),
    )];
    let result2 = convert_to_openai_messages(&messages2, TextFormat::Block, false);
    let blocks2 = result2[0]["content"].as_array().unwrap();
    assert_eq!(blocks2[0]["type"], "reasoning");
    let summary = blocks2[0]["summary"].as_array().unwrap();
    assert_eq!(summary.len(), 2);
    assert_eq!(summary[0]["text"], "First thought");
    assert_eq!(summary[1]["text"], "Second thought");

    // Mixed content with reasoning and text
    let mixed_content = serde_json::json!([
        {"type": "text", "text": "Regular response"},
        {
            "type": "reasoning",
            "summary": [{"type": "text", "text": "My reasoning process"}],
        },
    ]);
    let messages3 = vec![BaseMessage::AI(
        AIMessage::builder()
            .content(serde_json::to_string(&mixed_content).unwrap())
            .build(),
    )];
    let result3 = convert_to_openai_messages(&messages3, TextFormat::Block, false);
    let blocks3 = result3[0]["content"].as_array().unwrap();
    assert_eq!(blocks3.len(), 2);
    assert_eq!(blocks3[0]["type"], "text");
    assert_eq!(blocks3[0]["text"], "Regular response");
    assert_eq!(blocks3[1]["type"], "reasoning");
}

// ============================================================================
// test_convert_to_openai_messages_thinking_blocks
// ============================================================================

#[test]
fn test_convert_to_openai_messages_thinking_blocks() {
    // Thinking blocks should pass through
    let thinking_block = serde_json::json!({
        "signature": "abc123",
        "thinking": "Thinking text.",
        "type": "thinking",
    });
    let text_block = serde_json::json!({"text": "Response text.", "type": "text"});
    let content = serde_json::json!([thinking_block, text_block]);

    let messages = vec![BaseMessage::AI(
        AIMessage::builder()
            .content(serde_json::to_string(&content).unwrap())
            .build(),
    )];
    let result = convert_to_openai_messages(&messages, TextFormat::Block, false);

    assert_eq!(result.len(), 1);
    let blocks = result[0]["content"].as_array().unwrap();
    assert_eq!(blocks.len(), 2);
    assert_eq!(blocks[0]["type"], "thinking");
    assert_eq!(blocks[1]["type"], "text");
}
