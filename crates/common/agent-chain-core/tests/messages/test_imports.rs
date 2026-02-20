#[test]
#[allow(unused_imports)]
fn test_all_imports() {
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

    use agent_chain_core::messages::InvalidToolCall;
    use agent_chain_core::messages::ToolCall;
    use agent_chain_core::messages::ToolCallChunk;

    use agent_chain_core::messages::InputTokenDetails;
    use agent_chain_core::messages::OutputTokenDetails;
    use agent_chain_core::messages::UsageMetadata;

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

    use agent_chain_core::messages::invalid_tool_call;
    use agent_chain_core::messages::tool_call;
    use agent_chain_core::messages::tool_call_chunk;

    use agent_chain_core::messages::KNOWN_BLOCK_TYPES;

    let _ = AIMessage::builder().content("test").build();
    let _ = HumanMessage::builder().content("test").build();
    let _ = SystemMessage::builder().content("test").build();
    let _ = ChatMessage::builder().content("test").role("user").build();
    let _ = FunctionMessage::builder()
        .content("test")
        .name("func")
        .build();
    let _ = ToolMessage::builder()
        .content("test")
        .tool_call_id("call-123")
        .build();
    let _ = RemoveMessage::builder().id("msg-123").build();

    let _ = AIMessageChunk::builder().content("test").build();
    let _ = HumanMessageChunk::builder().content("test").build();
    let _ = SystemMessageChunk::builder().content("test").build();
    let _ = ChatMessageChunk::builder()
        .content("test")
        .role("user")
        .build();
    let _ = FunctionMessageChunk::builder()
        .content("test")
        .name("func")
        .build();
    let _ = ToolMessageChunk::builder()
        .content("test")
        .tool_call_id("call-123")
        .build();

    let _ = tool_call("test", serde_json::json!({}), None);
    let _ = tool_call_chunk(None, None, None, None);
    let _ = invalid_tool_call(None, None, None, None);

    let _ = UsageMetadata::new(10, 20);
    let _ = InputTokenDetails {
        audio: None,
        cache_creation: None,
        cache_read: None,
        ..Default::default()
    };
    let _ = OutputTokenDetails {
        audio: None,
        reasoning: None,
        ..Default::default()
    };

    let _ = KNOWN_BLOCK_TYPES;
}

#[test]
fn test_base_message_variants() {
    use agent_chain_core::messages::{
        AIMessage, BaseMessage, ChatMessage, FunctionMessage, HumanMessage, RemoveMessage,
        SystemMessage, ToolMessage,
    };

    let _human = BaseMessage::Human(HumanMessage::builder().content("test").build());
    let _ai = BaseMessage::AI(AIMessage::builder().content("test").build());
    let _system = BaseMessage::System(SystemMessage::builder().content("test").build());
    let _chat = BaseMessage::Chat(ChatMessage::builder().content("test").role("user").build());
    let _function = BaseMessage::Function(
        FunctionMessage::builder()
            .content("test")
            .name("func")
            .build(),
    );
    let _tool = BaseMessage::Tool(
        ToolMessage::builder()
            .content("test")
            .tool_call_id("call-123")
            .build(),
    );
    let _remove = BaseMessage::Remove(RemoveMessage::builder().id("msg-123").build());
}

#[test]
#[allow(unused_imports)]
fn test_trait_imports() {
    let msg = agent_chain_core::messages::HumanMessage::builder()
        .content("test")
        .build();
    let _ = msg.content;
    let _ = msg.message_type();
    let _ = msg.content.as_text();
}
