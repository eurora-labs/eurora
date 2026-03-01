use agent_chain_core::TextSplitter;
use agent_chain_core::text_splitters::{TextSplitterConfig, TokenTextSplitter};

#[test]
fn test_token_text_splitter() {
    let config = TextSplitterConfig::new(5, 0, None, None, None, None).unwrap();
    let splitter = TokenTextSplitter::builder().config(config).build().unwrap();
    // "abcdef" repeated 5 times = 30 chars
    // With gpt2 encoding, each "abcdef" may encode to different token counts
    // The Python test uses default gpt2 encoding with chunk_size=5, chunk_overlap=0
    let output = splitter.split_text(&"abcdef".repeat(5)).unwrap();
    let expected = vec!["abcdefabcdefabc", "defabcdefabcdef"];
    assert_eq!(output, expected);
}

#[test]
fn test_token_text_splitter_overlap() {
    let config = TextSplitterConfig::new(5, 1, None, None, None, None).unwrap();
    let splitter = TokenTextSplitter::builder().config(config).build().unwrap();
    let output = splitter.split_text(&"abcdef".repeat(5)).unwrap();
    let expected = vec!["abcdefabcdefabc", "abcdefabcdefabc", "abcdef"];
    assert_eq!(output, expected);
}

#[test]
fn test_token_text_splitter_from_tiktoken() {
    let config = TextSplitterConfig::default();
    let splitter =
        TokenTextSplitter::from_tiktoken_encoder(None, Some("gpt-3.5-turbo"), config).unwrap();
    // gpt-3.5-turbo uses cl100k_base encoding
    // Verify the splitter was created successfully and can encode text
    let output = splitter.split_text("Hello, world!").unwrap();
    assert_eq!(output, vec!["Hello, world!"]);
}

#[test]
fn test_tiktoken_length_function() {
    use agent_chain_core::text_splitters::tiktoken_length_function;

    let length_fn = tiktoken_length_function(Some("cl100k_base"), None).unwrap();
    // "Hello, world!" in cl100k_base should produce a small number of tokens
    let token_count = length_fn("Hello, world!");
    assert!(token_count > 0);
    assert!(token_count < 20);
}

#[test]
fn test_resolve_tiktoken_bpe_by_encoding() {
    use agent_chain_core::text_splitters::resolve_tiktoken_bpe;

    let bpe = resolve_tiktoken_bpe(Some("cl100k_base"), None).unwrap();
    let tokens = bpe.encode_with_special_tokens("test");
    assert!(!tokens.is_empty());
}

#[test]
fn test_resolve_tiktoken_bpe_by_model() {
    use agent_chain_core::text_splitters::resolve_tiktoken_bpe;

    let bpe = resolve_tiktoken_bpe(None, Some("gpt-4")).unwrap();
    let tokens = bpe.encode_with_special_tokens("test");
    assert!(!tokens.is_empty());
}

#[test]
fn test_resolve_tiktoken_bpe_default() {
    use agent_chain_core::text_splitters::resolve_tiktoken_bpe;

    // Default should use gpt2 encoding
    let bpe = resolve_tiktoken_bpe(None, None).unwrap();
    let tokens = bpe.encode_with_special_tokens("test");
    assert!(!tokens.is_empty());
}

#[test]
fn test_resolve_tiktoken_bpe_unknown_encoding() {
    use agent_chain_core::text_splitters::resolve_tiktoken_bpe;

    let result = resolve_tiktoken_bpe(Some("nonexistent_encoding"), None);
    assert!(result.is_err());
}
