//! Integration tests for euro-llm-core.

use euro_llm_core::*;

#[test]
fn test_message_creation() {
    let user_msg = Message::user("Hello, world!");
    assert_eq!(user_msg.role, Role::User);
    match &user_msg.content {
        MessageContent::Text(text) => assert_eq!(text, "Hello, world!"),
        _ => panic!("Expected text content"),
    }

    let assistant_msg = Message::assistant("Hi there!");
    assert_eq!(assistant_msg.role, Role::Assistant);

    let system_msg = Message::system("You are a helpful assistant.");
    assert_eq!(system_msg.role, Role::System);
}

#[test]
fn test_chat_request_builder() {
    let request = ChatRequest::builder()
        .message(Message::user("Test message"))
        .temperature(0.7)
        .max_tokens(100)
        .top_p(0.9)
        .stop_sequences(vec!["STOP".to_string()])
        .request_id("test-123".to_string())
        .user_id("user-456".to_string())
        .extension(
            "custom_param".to_string(),
            serde_json::json!("custom_value"),
        )
        .build();

    assert_eq!(request.messages.len(), 1);
    assert_eq!(request.parameters.temperature, Some(0.7));
    assert_eq!(request.parameters.max_tokens, Some(100));
    assert_eq!(request.parameters.top_p, Some(0.9));
    assert_eq!(request.parameters.stop_sequences, vec!["STOP"]);
    assert_eq!(request.metadata.request_id, Some("test-123".to_string()));
    assert_eq!(request.metadata.user_id, Some("user-456".to_string()));
    assert_eq!(
        request.metadata.extensions.get("custom_param"),
        Some(&serde_json::json!("custom_value"))
    );
}

#[test]
fn test_parameters_default() {
    let params = Parameters::default();
    assert_eq!(params.temperature, None);
    assert_eq!(params.max_tokens, None);
    assert_eq!(params.top_p, None);
    assert_eq!(params.top_k, None);
    assert!(params.stop_sequences.is_empty());
    assert_eq!(params.frequency_penalty, None);
    assert_eq!(params.presence_penalty, None);
}

#[test]
fn test_metadata_default() {
    let metadata = Metadata::default();
    assert!(metadata.extensions.is_empty());
    assert_eq!(metadata.request_id, None);
    assert_eq!(metadata.user_id, None);
    // created_at should be set to current time
    assert!(metadata.created_at > chrono::DateTime::UNIX_EPOCH);
}

#[test]
fn test_multimodal_content() {
    let parts = vec![
        ContentPart::Text {
            text: "Describe this image:".to_string(),
        },
        ContentPart::Image {
            image_source: ImageSource::Url("https://example.com/image.jpg".to_string()),
            detail: Some("high".to_string()),
        },
    ];

    let message = Message::user_multimodal(parts);

    match &message.content {
        MessageContent::Multimodal(parts) => {
            assert_eq!(parts.len(), 2);
            match &parts[0] {
                ContentPart::Text { text } => assert_eq!(text, "Describe this image:"),
                _ => panic!("Expected text part"),
            }
            match &parts[1] {
                ContentPart::Image { image_source, .. } => {
                    let url: String = image_source.clone().into();
                    assert_eq!(url, "https://example.com/image.jpg");
                }
                _ => panic!("Expected image part"),
            }
        }
        _ => panic!("Expected multimodal content"),
    }
}

#[test]
fn test_tool_definition() {
    let function = Function {
        name: "get_weather".to_string(),
        description: "Get current weather for a location".to_string(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "location": {
                    "type": "string",
                    "description": "City name"
                }
            },
            "required": ["location"]
        }),
    };

    let tool = Tool {
        tool_type: "function".to_string(),
        function,
    };

    assert_eq!(tool.tool_type, "function");
    assert_eq!(tool.function.name, "get_weather");
    assert_eq!(
        tool.function.description,
        "Get current weather for a location"
    );
}

#[test]
fn test_tool_call() {
    let tool_call = ToolCall {
        id: "call_123".to_string(),
        call_type: "function".to_string(),
        function: FunctionCall {
            name: "get_weather".to_string(),
            arguments: r#"{"location": "San Francisco"}"#.to_string(),
        },
    };

    assert_eq!(tool_call.id, "call_123");
    assert_eq!(tool_call.call_type, "function");
    assert_eq!(tool_call.function.name, "get_weather");
    assert_eq!(
        tool_call.function.arguments,
        r#"{"location": "San Francisco"}"#
    );
}

#[test]
fn test_usage_statistics() {
    let usage = Usage {
        prompt_tokens: 10,
        completion_tokens: 20,
        total_tokens: 30,
    };

    assert_eq!(usage.prompt_tokens, 10);
    assert_eq!(usage.completion_tokens, 20);
    assert_eq!(usage.total_tokens, 30);
}

#[test]
fn test_finish_reason_variants() {
    let reasons = vec![
        FinishReason::Stop,
        FinishReason::Length,
        FinishReason::StopSequence,
        FinishReason::ToolCalls,
        FinishReason::ContentFilter,
        FinishReason::Error,
    ];

    // Test serialization/deserialization
    for reason in reasons {
        let json = serde_json::to_string(&reason).unwrap();
        let deserialized: FinishReason = serde_json::from_str(&json).unwrap();
        assert_eq!(reason, deserialized);
    }
}

#[test]
fn test_embedding() {
    let embedding = Embedding {
        embedding: vec![0.1, 0.2, 0.3, 0.4, 0.5],
        index: 0,
    };

    assert_eq!(embedding.embedding.len(), 5);
    assert_eq!(embedding.index, 0);
    assert_eq!(embedding.embedding[0], 0.1);
    assert_eq!(embedding.embedding[4], 0.5);
}

#[test]
fn test_image_request() {
    let request = ImageRequest {
        prompt: "A beautiful sunset".to_string(),
        negative_prompt: Some("blurry, low quality".to_string()),
        n: Some(2),
        size: Some("1024x1024".to_string()),
        quality: Some("hd".to_string()),
        response_format: Some("url".to_string()),
        metadata: Metadata::default(),
    };

    assert_eq!(request.prompt, "A beautiful sunset");
    assert_eq!(
        request.negative_prompt,
        Some("blurry, low quality".to_string())
    );
    assert_eq!(request.n, Some(2));
    assert_eq!(request.size, Some("1024x1024".to_string()));
    assert_eq!(request.quality, Some("hd".to_string()));
    assert_eq!(request.response_format, Some("url".to_string()));
}

#[test]
fn test_speech_to_text_request() {
    let request = SpeechToTextRequest {
        audio: "base64_encoded_audio_data".to_string(),
        format: "mp3".to_string(),
        language: Some("en".to_string()),
        metadata: Metadata::default(),
    };

    assert_eq!(request.audio, "base64_encoded_audio_data");
    assert_eq!(request.format, "mp3");
    assert_eq!(request.language, Some("en".to_string()));
}

#[test]
fn test_text_to_speech_request() {
    let request = TextToSpeechRequest {
        text: "Hello, world!".to_string(),
        voice: "alloy".to_string(),
        format: Some("mp3".to_string()),
        speed: Some(1.0),
        metadata: Metadata::default(),
    };

    assert_eq!(request.text, "Hello, world!");
    assert_eq!(request.voice, "alloy");
    assert_eq!(request.format, Some("mp3".to_string()));
    assert_eq!(request.speed, Some(1.0));
}

#[test]
fn test_generated_image() {
    let image = GeneratedImage {
        url: Some("https://example.com/image.jpg".to_string()),
        b64_json: None,
        revised_prompt: Some("A beautiful sunset over mountains".to_string()),
    };

    assert_eq!(image.url, Some("https://example.com/image.jpg".to_string()));
    assert_eq!(image.b64_json, None);
    assert_eq!(
        image.revised_prompt,
        Some("A beautiful sunset over mountains".to_string())
    );
}

#[test]
fn test_completion_request() {
    let request = CompletionRequest {
        prompt: "Once upon a time".to_string(),
        parameters: Parameters {
            temperature: Some(0.8),
            max_tokens: Some(150),
            ..Default::default()
        },
        metadata: Metadata::default(),
    };

    assert_eq!(request.prompt, "Once upon a time");
    assert_eq!(request.parameters.temperature, Some(0.8));
    assert_eq!(request.parameters.max_tokens, Some(150));
}

// Test serialization/deserialization of all major types
#[test]
fn test_serialization_roundtrip() {
    let message = Message::user("Test message");
    let json = serde_json::to_string(&message).unwrap();
    let deserialized: Message = serde_json::from_str(&json).unwrap();
    match (&message.content, &deserialized.content) {
        (MessageContent::Text(a), MessageContent::Text(b)) => assert_eq!(a, b),
        _ => panic!("Content mismatch"),
    }

    let request = ChatRequest::builder()
        .message(message)
        .temperature(0.7)
        .build();
    let json = serde_json::to_string(&request).unwrap();
    let deserialized: ChatRequest = serde_json::from_str(&json).unwrap();
    assert_eq!(
        request.parameters.temperature,
        deserialized.parameters.temperature
    );
}
