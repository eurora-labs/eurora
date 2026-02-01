//! Tests for message module imports.
//!
//! Converted from `langchain/libs/core/tests/unit_tests/messages/test_imports.py`

// Test that all expected public exports are available from the messages module
#[test]
#[allow(unused_imports)]
fn test_all_imports() {
    // Message types
    use agent_chain_core::messages::AIMessage;
    use agent_chain_core::messages::AIMessageChunk;
    use agent_chain_core::messages::BaseMessage;
    use agent_chain_core::messages::ChatMessage;
    use agent_chain_core::messages::ChatMessageChunk;
    use agent_chain_core::messages::FunctionMessage;
    use agent_chain_core::messages::FunctionMessageChunk;
    use agent_chain_core::messages::HumanMessage;
    use agent_chain_core::messages::HumanMessageChunk;
    use agent_chain_core::messages::RemoveMessage;
    use agent_chain_core::messages::SystemMessage;
    use agent_chain_core::messages::SystemMessageChunk;
    use agent_chain_core::messages::ToolMessage;
    use agent_chain_core::messages::ToolMessageChunk;

    // Tool call types
    use agent_chain_core::messages::InvalidToolCall;
    use agent_chain_core::messages::ToolCall;
    use agent_chain_core::messages::ToolCallChunk;

    // Usage metadata types
    use agent_chain_core::messages::InputTokenDetails;
    use agent_chain_core::messages::OutputTokenDetails;
    use agent_chain_core::messages::UsageMetadata;

    // Utility functions
    use agent_chain_core::messages::convert_to_messages;
    use agent_chain_core::messages::convert_to_openai_messages;
    use agent_chain_core::messages::filter_messages;
    use agent_chain_core::messages::get_buffer_string;
    use agent_chain_core::messages::is_data_content_block;
    use agent_chain_core::messages::merge_content;
    use agent_chain_core::messages::merge_message_runs;
    use agent_chain_core::messages::message_chunk_to_message;
    use agent_chain_core::messages::message_to_dict;
    use agent_chain_core::messages::messages_from_dict;
    use agent_chain_core::messages::messages_to_dict;
    use agent_chain_core::messages::trim_messages;

    // Factory functions
    use agent_chain_core::messages::invalid_tool_call;
    use agent_chain_core::messages::tool_call;
    use agent_chain_core::messages::tool_call_chunk;

    // Constants
    use agent_chain_core::messages::KNOWN_BLOCK_TYPES;

    // Verify they're not just imported but usable
    let _ = AIMessage::builder().content("test").build();
    let _ = HumanMessage::new("test");
    let _ = SystemMessage::new("test");
    let _ = ChatMessage::new("test", "user");
    let _ = FunctionMessage::new("test", "func");
    let _ = ToolMessage::new("test", "call-123");
    let _ = RemoveMessage::new("msg-123");

    let _ = AIMessageChunk::builder().content("test").build();
    let _ = HumanMessageChunk::new("test");
    let _ = SystemMessageChunk::new("test");
    let _ = ChatMessageChunk::new("test", "user");
    let _ = FunctionMessageChunk::new("test", "func");
    let _ = ToolMessageChunk::new("test", "call-123");

    let _ = tool_call("test", serde_json::json!({}), None);
    let _ = tool_call_chunk(None, None, None, None);
    let _ = invalid_tool_call(None, None, None, None);

    let _ = UsageMetadata::new(10, 20);
    let _ = InputTokenDetails {
        audio: None,
        cache_creation: None,
        cache_read: None,
    };
    let _ = OutputTokenDetails {
        audio: None,
        reasoning: None,
    };

    let _ = KNOWN_BLOCK_TYPES;
}

#[test]
fn test_base_message_variants() {
    use agent_chain_core::messages::{
        AIMessage, BaseMessage, ChatMessage, FunctionMessage, HumanMessage, RemoveMessage,
        SystemMessage, ToolMessage,
    };

    // Test that all BaseMessage variants are accessible
    let _human = BaseMessage::Human(HumanMessage::new("test"));
    let _ai = BaseMessage::AI(AIMessage::builder().content("test").build());
    let _system = BaseMessage::System(SystemMessage::new("test"));
    let _chat = BaseMessage::Chat(ChatMessage::new("test", "user"));
    let _function = BaseMessage::Function(FunctionMessage::new("test", "func"));
    let _tool = BaseMessage::Tool(ToolMessage::new("test", "call-123"));
    let _remove = BaseMessage::Remove(RemoveMessage::new("msg-123"));
}

#[test]
#[allow(unused_imports)]
fn test_trait_imports() {
    // Verify trait is accessible and provides expected methods
    let msg = agent_chain_core::messages::HumanMessage::new("test");
    let _ = msg.content();
    let _ = msg.message_type();
    let _ = msg.text();
}
