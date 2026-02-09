//! Test StrOutputParser.
//!
//! Ported from langchain/libs/core/tests/unit_tests/output_parsers/test_string_parser.py

use agent_chain_core::GenericFakeChatModel;
use agent_chain_core::language_models::BaseChatModel;
use agent_chain_core::messages::{
    AIMessage, AIMessageChunk, BaseMessage, BaseMessageChunk, HumanMessage,
};
use agent_chain_core::output_parsers::{
    BaseOutputParser, BaseTransformOutputParser, StrOutputParser,
};
use agent_chain_core::outputs::{ChatGeneration, Generation};
use futures::StreamExt;

#[test]
fn test_str_output_parser_parse() {
    // Test StrOutputParser.parse() returns input unchanged
    let parser = StrOutputParser::new();
    let text = "Hello, world!";
    assert_eq!(parser.parse(text).unwrap(), text);
}

#[test]
fn test_str_output_parser_parse_empty_string() {
    // Test StrOutputParser.parse() with empty string
    let parser = StrOutputParser::new();
    assert_eq!(parser.parse("").unwrap(), "");
}

#[test]
fn test_str_output_parser_parse_multiline() {
    // Test StrOutputParser.parse() with multiline text
    let parser = StrOutputParser::new();
    let text = "Line 1\nLine 2\nLine 3";
    assert_eq!(parser.parse(text).unwrap(), text);
}

#[test]
fn test_str_output_parser_parse_special_chars() {
    // Test StrOutputParser.parse() with special characters
    let parser = StrOutputParser::new();
    let text = "Special chars: !@#$%^&*()_+-=[]{}|;':\",./<>?";
    assert_eq!(parser.parse(text).unwrap(), text);
}

#[test]
fn test_str_output_parser_parse_unicode() {
    // Test StrOutputParser.parse() with unicode characters
    let parser = StrOutputParser::new();
    let text = "Unicode: 你好, こんにちは, नमस्ते, 🎉";
    assert_eq!(parser.parse(text).unwrap(), text);
}

#[test]
fn test_str_output_parser_invoke_with_message() {
    // Test StrOutputParser.invoke() with AIMessage input
    let parser = StrOutputParser::new();
    let message = AIMessage::builder().content("Hello from AI").build();
    let result = parser.invoke(BaseMessage::AI(message), None).unwrap();
    assert_eq!(result, "Hello from AI");
}

#[test]
fn test_str_output_parser_invoke_with_human_message() {
    // Test StrOutputParser.invoke() with HumanMessage input
    let parser = StrOutputParser::new();
    let message = HumanMessage::builder().content("Hello from human").build();
    let result = parser.invoke(BaseMessage::Human(message), None).unwrap();
    assert_eq!(result, "Hello from human");
}

#[tokio::test]
async fn test_str_output_parser_ainvoke_with_message() {
    // Test StrOutputParser.ainvoke() with AIMessage input
    let parser = StrOutputParser::new();
    let message = AIMessage::builder().content("Async hello from AI").build();
    let result = parser
        .ainvoke(BaseMessage::AI(message), None)
        .await
        .unwrap();
    assert_eq!(result, "Async hello from AI");
}

#[test]
fn test_str_output_parser_parse_result_with_generation() {
    // Test StrOutputParser.parse_result() with Generation
    let parser = StrOutputParser::new();
    let generation = Generation::new("Generated text");
    let result = parser.parse_result(&[generation], false).unwrap();
    assert_eq!(result, "Generated text");
}

#[test]
fn test_str_output_parser_parse_result_with_chat_generation() {
    // Test StrOutputParser.parse_result() with ChatGeneration
    let parser = StrOutputParser::new();
    let message = AIMessage::builder().content("Chat generated text").build();
    let chat_generation = ChatGeneration::new(BaseMessage::AI(message));
    // ChatGeneration has a `text` field that holds the content
    let generation = Generation::new(&chat_generation.text);
    let result = parser.parse_result(&[generation], false).unwrap();
    assert_eq!(result, "Chat generated text");
}

#[tokio::test]
async fn test_str_output_parser_transform_string_chunks() {
    // Test StrOutputParser.transform() with string chunks
    let parser = StrOutputParser::new();
    let chunks = vec!["Hello", " ", "world", "!"];

    let input_stream = futures::stream::iter(chunks.iter().map(|s| BaseMessage::from(*s)));
    let mut result_stream = parser.transform(Box::pin(input_stream));

    let mut results = Vec::new();
    while let Some(result) = result_stream.next().await {
        results.push(result.unwrap());
    }

    assert_eq!(results, chunks);
}

#[tokio::test]
async fn test_str_output_parser_transform_message_chunks() {
    // Test StrOutputParser.transform() with message chunks
    let parser = StrOutputParser::new();
    let chunks = vec![
        BaseMessageChunk::AI(AIMessageChunk::builder().content("Hello").build()),
        BaseMessageChunk::AI(AIMessageChunk::builder().content(" ").build()),
        BaseMessageChunk::AI(AIMessageChunk::builder().content("world").build()),
    ];

    let input_stream = futures::stream::iter(chunks.into_iter().map(|c| c.to_message()));
    let mut result_stream = parser.transform(Box::pin(input_stream));

    let mut results = Vec::new();
    while let Some(result) = result_stream.next().await {
        results.push(result.unwrap());
    }

    assert_eq!(results, vec!["Hello", " ", "world"]);
}

#[tokio::test]
async fn test_str_output_parser_atransform_string_chunks() {
    // Test StrOutputParser.atransform() with string chunks
    let parser = StrOutputParser::new();
    let chunks = vec!["Async", " ", "test"];

    let input_stream = futures::stream::iter(chunks.iter().map(|s| BaseMessage::from(*s)));
    let result: Vec<String> = parser
        .atransform(Box::pin(input_stream))
        .filter_map(|r| async { r.ok() })
        .collect()
        .await;

    assert_eq!(result, chunks);
}

#[tokio::test]
async fn test_str_output_parser_atransform_message_chunks() {
    // Test StrOutputParser.atransform() with message chunks
    let parser = StrOutputParser::new();
    let chunks = vec![
        BaseMessageChunk::AI(AIMessageChunk::builder().content("Async").build()),
        BaseMessageChunk::AI(AIMessageChunk::builder().content(" ").build()),
        BaseMessageChunk::AI(AIMessageChunk::builder().content("messages").build()),
    ];

    let input_stream = futures::stream::iter(chunks.into_iter().map(|c| c.to_message()));
    let result: Vec<String> = parser
        .atransform(Box::pin(input_stream))
        .filter_map(|r| async { r.ok() })
        .collect()
        .await;

    assert_eq!(result, vec!["Async", " ", "messages"]);
}

#[tokio::test]
async fn test_str_output_parser_with_model_chain() {
    // Test StrOutputParser chained with a model
    let model =
        GenericFakeChatModel::from_vec(vec![AIMessage::builder().content("Model output").build()]);
    let parser = StrOutputParser::new();

    // Simulate chaining: model | parser
    let model_output = model
        ._generate(
            vec![BaseMessage::Human(
                HumanMessage::builder().content("input").build(),
            )],
            None,
            None,
        )
        .await
        .unwrap();
    let result = parser
        .invoke(model_output.generations[0].message.clone(), None)
        .unwrap();

    assert_eq!(result, "Model output");
}

#[tokio::test]
async fn test_str_output_parser_with_model_stream() {
    // Test StrOutputParser streaming with a model
    let model = GenericFakeChatModel::from_vec(vec![
        AIMessage::builder().content("Streaming output").build(),
    ]);
    let parser = StrOutputParser::new();

    // Simulate streaming: model.stream() | parser
    let stream = model
        ._stream(
            vec![BaseMessage::Human(
                HumanMessage::builder().content("input").build(),
            )],
            None,
            None,
        )
        .unwrap();

    // The model splits by whitespace
    let chunks: Vec<String> = stream
        .filter_map(|chunk| async {
            chunk
                .ok()
                .map(|c| parser.invoke(c.message.clone(), None).unwrap())
        })
        .collect()
        .await;

    assert_eq!(chunks, vec!["Streaming", " ", "output"]);
}

#[test]
fn test_str_output_parser_with_empty_content() {
    // Test StrOutputParser with message containing empty content
    let parser = StrOutputParser::new();
    let message = AIMessage::builder().content("").build();
    let result = parser.invoke(BaseMessage::AI(message), None).unwrap();
    assert_eq!(result, "");
}

#[test]
fn test_str_output_parser_with_whitespace_only() {
    // Test StrOutputParser with whitespace-only content
    let parser = StrOutputParser::new();
    let text = "   \n\t  ";
    assert_eq!(parser.parse(text).unwrap(), text);
}

#[test]
fn test_str_output_parser_preserves_formatting() {
    // Test StrOutputParser preserves text formatting
    let parser = StrOutputParser::new();
    let text = "
    This is a formatted text
        with indentation
    and multiple lines
    ";
    assert_eq!(parser.parse(text).unwrap(), text);
}

#[test]
fn test_str_output_parser_with_code_block() {
    // Test StrOutputParser with code block content
    let parser = StrOutputParser::new();
    let text = "```python
def hello():
    print(\"Hello, world!\")
```";
    assert_eq!(parser.parse(text).unwrap(), text);
}

#[test]
fn test_str_output_parser_with_json_string() {
    // Test StrOutputParser with JSON string (should not parse it)
    let parser = StrOutputParser::new();
    let text = r#"{"key": "value", "number": 42}"#;
    // StrOutputParser should return the string as-is, not parse JSON
    assert_eq!(parser.parse(text).unwrap(), text);
}

#[test]
fn test_str_output_parser_with_xml_string() {
    // Test StrOutputParser with XML string (should not parse it)
    let parser = StrOutputParser::new();
    let text = "<root><child>value</child></root>";
    // StrOutputParser should return the string as-is, not parse XML
    assert_eq!(parser.parse(text).unwrap(), text);
}

#[test]
fn test_str_output_parser_multiple_generations() {
    // Test StrOutputParser.parse_result() uses only first generation
    let parser = StrOutputParser::new();
    let generations = vec![
        Generation::new("First generation"),
        Generation::new("Second generation"),
        Generation::new("Third generation"),
    ];
    // Should only use the first generation
    let result = parser.parse_result(&generations, false).unwrap();
    assert_eq!(result, "First generation");
}

#[tokio::test]
async fn test_str_output_parser_aparse() {
    // Test StrOutputParser.aparse() method
    let parser = StrOutputParser::new();
    let text = "Async parse test";
    let result = parser.aparse(text).await.unwrap();
    assert_eq!(result, text);
}

#[test]
fn test_str_output_parser_with_long_text() {
    // Test StrOutputParser with long text
    let parser = StrOutputParser::new();
    let text = "A".repeat(10000); // 10k characters
    assert_eq!(parser.parse(&text).unwrap(), text);
    assert_eq!(parser.parse(&text).unwrap().len(), 10000);
}

#[tokio::test]
async fn test_str_output_parser_transform_empty_iterator() {
    // Test StrOutputParser.transform() with empty iterator
    let parser = StrOutputParser::new();

    let input_stream = futures::stream::iter(Vec::<BaseMessage>::new());
    let result: Vec<String> = parser
        .transform(Box::pin(input_stream))
        .filter_map(|r| async { r.ok() })
        .collect()
        .await;

    assert_eq!(result, Vec::<String>::new());
}

#[tokio::test]
async fn test_str_output_parser_atransform_empty_iterator() {
    // Test StrOutputParser.atransform() with empty iterator
    let parser = StrOutputParser::new();

    let input_stream = futures::stream::iter(Vec::<BaseMessage>::new());
    let result: Vec<String> = parser
        .atransform(Box::pin(input_stream))
        .filter_map(|r| async { r.ok() })
        .collect()
        .await;

    assert_eq!(result, Vec::<String>::new());
}
