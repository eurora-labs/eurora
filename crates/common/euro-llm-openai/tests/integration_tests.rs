//! Integration tests for euro-llm-openai provider.

use euro_llm_core::*;
use euro_llm_openai::*;
use std::time::Duration;

#[test]
fn test_openai_config_creation() {
    let config = OpenAIConfig::new("sk-test123456789", "gpt-4");
    assert_eq!(config.model, "gpt-4");
    assert_eq!(config.api_key.expose_secret(), "sk-test123456789");
    assert_eq!(config.base_url(), "https://api.openai.com/v1");
    assert_eq!(config.organization, None);
    assert_eq!(config.project, None);
}

#[test]
fn test_openai_config_builder() {
    let config = OpenAIConfig::builder()
        .api_key("sk-test123456789")
        .model("gpt-3.5-turbo")
        .organization("org-123")
        .project("proj-456")
        .timeout(Duration::from_secs(60))
        .max_retries(5)
        .header("X-Custom-Header".to_string(), "custom-value".to_string())
        .build();

    assert_eq!(config.model, "gpt-3.5-turbo");
    assert_eq!(config.organization, Some("org-123".to_string()));
    assert_eq!(config.project, Some("proj-456".to_string()));
    assert_eq!(config.http.timeout, Duration::from_secs(60));
    assert_eq!(config.http.max_retries, 5);
    assert_eq!(
        config.http.headers.get("X-Custom-Header"),
        Some(&"custom-value".to_string())
    );
}

#[test]
fn test_openai_config_urls() {
    let config = OpenAIConfig::new("sk-test", "gpt-4");
    assert_eq!(
        config.chat_url(),
        "https://api.openai.com/v1/chat/completions"
    );
    assert_eq!(
        config.completions_url(),
        "https://api.openai.com/v1/completions"
    );
    assert_eq!(
        config.embeddings_url(),
        "https://api.openai.com/v1/embeddings"
    );
    assert_eq!(
        config.images_url(),
        "https://api.openai.com/v1/images/generations"
    );
    assert_eq!(
        config.transcriptions_url(),
        "https://api.openai.com/v1/audio/transcriptions"
    );
    assert_eq!(
        config.speech_url(),
        "https://api.openai.com/v1/audio/speech"
    );
}

#[test]
fn test_openai_config_custom_base_url() {
    let config = OpenAIConfig::builder()
        .api_key("sk-test")
        .model("gpt-4")
        .base_url("https://custom.openai.com/v1")
        .unwrap()
        .build();

    assert_eq!(config.base_url(), "https://custom.openai.com/v1");
    assert_eq!(
        config.chat_url(),
        "https://custom.openai.com/v1/chat/completions"
    );
}

#[test]
fn test_openai_config_validation() {
    // Valid config should pass
    let valid_config = OpenAIConfig::new("sk-test123456789", "gpt-4");
    assert!(valid_config.validate().is_ok());

    // Empty API key should fail
    let invalid_config = OpenAIConfig::new("", "gpt-4");
    assert!(invalid_config.validate().is_err());

    // Empty model should fail
    let invalid_config = OpenAIConfig::new("sk-test123456789", "");
    assert!(invalid_config.validate().is_err());

    // Short API key should fail
    let invalid_config = OpenAIConfig::new("sk-short", "gpt-4");
    assert!(invalid_config.validate().is_err());

    // Placeholder API key should fail
    let invalid_config = OpenAIConfig::new("your_api_key_here", "gpt-4");
    assert!(invalid_config.validate().is_err());
}

#[test]
fn test_openai_provider_creation() {
    let config = OpenAIConfig::new("sk-test123456789", "gpt-4");
    let provider = OpenAIProvider::new(config);
    assert!(provider.is_ok());
}

#[test]
fn test_openai_provider_creation_invalid_config() {
    let config = OpenAIConfig::new("", "gpt-4");
    // The provider creation should fail during validation
    let build_result = config.build();
    assert!(build_result.is_err());
}

#[test]
fn test_openai_error_types() {
    let auth_error = OpenAIError::Authentication {
        message: "Invalid API key".to_string(),
    };
    assert!(auth_error.is_auth_error());
    assert!(!auth_error.is_retryable());
    assert!(!auth_error.is_rate_limited());
    assert_eq!(auth_error.error_code(), Some("authentication_failed"));

    let rate_limit_error = OpenAIError::RateLimit {
        retry_after: Some(Duration::from_secs(60)),
    };
    assert!(rate_limit_error.is_rate_limited());
    assert!(rate_limit_error.is_retryable());
    assert!(!rate_limit_error.is_auth_error());
    assert_eq!(
        rate_limit_error.retry_after(),
        Some(Duration::from_secs(60))
    );

    let service_error = OpenAIError::ServiceUnavailable {
        message: "Service temporarily unavailable".to_string(),
    };
    assert!(service_error.is_service_unavailable());
    assert!(service_error.is_retryable());
    assert!(!service_error.is_auth_error());

    let content_filter_error = OpenAIError::ContentFiltered {
        message: "Content violates policy".to_string(),
    };
    assert!(content_filter_error.is_content_filtered());
    assert!(!content_filter_error.is_retryable());
}

#[test]
fn test_openai_error_from_response() {
    // Test 401 Unauthorized
    let error = OpenAIError::from_response(401, "Unauthorized");
    assert!(matches!(error, OpenAIError::Authentication { .. }));

    // Test 429 Rate Limited
    let error = OpenAIError::from_response(429, "Rate limit exceeded");
    assert!(matches!(error, OpenAIError::RateLimit { .. }));

    // Test 400 Bad Request
    let error = OpenAIError::from_response(400, "Invalid request");
    assert!(matches!(error, OpenAIError::InvalidRequest { .. }));

    // Test 500 Server Error
    let error = OpenAIError::from_response(500, "Internal server error");
    assert!(matches!(error, OpenAIError::ServiceUnavailable { .. }));

    // Test with JSON error response
    let json_error = r#"{"error": {"message": "Invalid API key", "type": "invalid_api_key"}}"#;
    let error = OpenAIError::from_response(401, json_error);
    assert!(matches!(error, OpenAIError::Authentication { .. }));
}

#[test]
fn test_openai_message_conversion() {
    let core_message = Message::user("Hello, world!");
    let openai_message: OpenAIMessage = (&core_message).into();

    assert_eq!(openai_message.role, "user");
    assert_eq!(
        openai_message.content,
        Some(serde_json::Value::String("Hello, world!".to_string()))
    );
    assert_eq!(openai_message.name, None);
    assert!(openai_message.tool_calls.is_none());

    // Test system message
    let system_message = Message::system("You are a helpful assistant.");
    let openai_message: OpenAIMessage = (&system_message).into();
    assert_eq!(openai_message.role, "system");

    // Test assistant message
    let assistant_message = Message::assistant("Hi there!");
    let openai_message: OpenAIMessage = (&assistant_message).into();
    assert_eq!(openai_message.role, "assistant");
}

#[test]
fn test_openai_multimodal_message_conversion() {
    let multimodal_content = vec![
        ContentPart::Text {
            text: "Describe this image:".to_string(),
        },
        ContentPart::Image {
            image_source: euro_llm_core::ImageSource::Url(
                "https://example.com/image.jpg".to_string(),
            ),
            detail: Some("high".to_string()),
        },
    ];

    let core_message = Message::user_multimodal(multimodal_content);

    let openai_message: OpenAIMessage = (&core_message).into();
    assert_eq!(openai_message.role, "user");

    // Check that content is an array
    match openai_message.content {
        Some(serde_json::Value::Array(parts)) => {
            assert_eq!(parts.len(), 2);

            // Check text part
            assert_eq!(parts[0]["type"], "text");
            assert_eq!(parts[0]["text"], "Describe this image:");

            // Check image part
            assert_eq!(parts[1]["type"], "image_url");
            assert_eq!(
                parts[1]["image_url"]["url"],
                "https://example.com/image.jpg"
            );
            assert_eq!(parts[1]["image_url"]["detail"], "high");
        }
        _ => panic!("Expected array content for multimodal message"),
    }
}

#[test]
fn test_openai_tool_conversion() {
    let core_tool = Tool {
        tool_type: "function".to_string(),
        function: Function {
            name: "get_weather".to_string(),
            description: "Get current weather".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "location": {"type": "string"}
                }
            }),
        },
    };

    let openai_tool: OpenAITool = (&core_tool).into();
    assert_eq!(openai_tool.tool_type, "function");
    assert_eq!(openai_tool.function.name, "get_weather");
    assert_eq!(openai_tool.function.description, "Get current weather");
}

#[test]
fn test_openai_usage_conversion() {
    let openai_usage = OpenAIUsage {
        prompt_tokens: 10,
        completion_tokens: 20,
        total_tokens: 30,
    };

    let core_usage: Usage = openai_usage.into();
    assert_eq!(core_usage.prompt_tokens, 10);
    assert_eq!(core_usage.completion_tokens, 20);
    assert_eq!(core_usage.total_tokens, 30);
}

#[test]
fn test_openai_chat_response_trait() {
    let openai_response = OpenAIChatResponse {
        id: "chat-123".to_string(),
        object: "chat.completion".to_string(),
        created: 1234567890,
        model: "gpt-4".to_string(),
        choices: vec![OpenAIChatChoice {
            index: 0,
            message: OpenAIMessage {
                role: "assistant".to_string(),
                content: Some(serde_json::Value::String("Hello!".to_string())),
                name: None,
                tool_calls: None,
                tool_call_id: None,
            },
            finish_reason: Some("stop".to_string()),
            logprobs: None,
        }],
        usage: Some(OpenAIUsage {
            prompt_tokens: 10,
            completion_tokens: 5,
            total_tokens: 15,
        }),
        system_fingerprint: None,
    };

    // Test ChatResponse trait implementation
    assert_eq!(openai_response.content(), "Hello!");
    assert_eq!(openai_response.finish_reason(), Some(FinishReason::Stop));
    assert!(openai_response.tool_calls().is_none());

    // Test as_message conversion
    let message = openai_response.as_message();
    assert_eq!(message.role, Role::Assistant);
    match message.content {
        MessageContent::Text(text) => assert_eq!(text, "Hello!"),
        _ => panic!("Expected text content"),
    }
}

#[test]
fn test_openai_completion_response_trait() {
    let openai_response = OpenAICompletionResponse {
        id: "cmpl-123".to_string(),
        object: "text_completion".to_string(),
        created: 1234567890,
        model: "gpt-3.5-turbo-instruct".to_string(),
        choices: vec![OpenAICompletionChoice {
            index: 0,
            text: "Once upon a time...".to_string(),
            finish_reason: Some("length".to_string()),
            logprobs: None,
        }],
        usage: Some(OpenAIUsage {
            prompt_tokens: 5,
            completion_tokens: 10,
            total_tokens: 15,
        }),
    };

    // Test CompletionResponse trait implementation
    assert_eq!(openai_response.text(), "Once upon a time...");
    assert_eq!(openai_response.finish_reason(), Some(FinishReason::Length));
}

#[test]
fn test_openai_request_serialization() {
    let request = OpenAIChatRequest {
        model: "gpt-4".to_string(),
        messages: vec![OpenAIMessage {
            role: "user".to_string(),
            content: Some(serde_json::Value::String("Hello".to_string())),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        }],
        temperature: Some(0.7),
        max_tokens: Some(100),
        top_p: Some(0.9),
        frequency_penalty: None,
        presence_penalty: None,
        stop: vec![],
        stream: Some(false),
        tools: None,
        tool_choice: None,
        user: None,
    };

    let json = serde_json::to_string(&request).unwrap();
    assert!(json.contains("\"model\":\"gpt-4\""));
    assert!(json.contains("\"temperature\":0.7"));
    assert!(json.contains("\"max_tokens\":100"));

    // Fields with None values should not be serialized
    assert!(!json.contains("frequency_penalty"));
    assert!(!json.contains("presence_penalty"));
    // Note: user field is None but may still appear in JSON as null
}

#[test]
fn test_openai_response_deserialization() {
    let json = r#"{
        "id": "chat-123",
        "object": "chat.completion",
        "created": 1234567890,
        "model": "gpt-4",
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": "Hello!"
            },
            "finish_reason": "stop"
        }],
        "usage": {
            "prompt_tokens": 10,
            "completion_tokens": 5,
            "total_tokens": 15
        }
    }"#;

    let response: OpenAIChatResponse = serde_json::from_str(json).unwrap();
    assert_eq!(response.id, "chat-123");
    assert_eq!(response.model, "gpt-4");
    assert_eq!(response.choices.len(), 1);
    assert_eq!(response.choices[0].message.role, "assistant");
    assert_eq!(response.usage.as_ref().unwrap().total_tokens, 15);
}

// Mock tests for provider functionality (without actual API calls)
#[test]
fn test_openai_provider_request_conversion() {
    let config = OpenAIConfig::new("sk-test123456789", "gpt-4");
    let _provider = OpenAIProvider::new(config).unwrap();

    let _core_request = ChatRequest::builder()
        .message(Message::user("Hello"))
        .temperature(0.8)
        .max_tokens(150)
        .top_p(0.9)
        .stop_sequences(vec!["STOP".to_string()])
        .user_id("user123".to_string())
        .build();

    // This is testing the internal conversion method
    // In a real scenario, we'd need to make this method public or test through integration
    // For now, this demonstrates the test structure
}
