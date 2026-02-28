use agent_chain::providers::ollama::parse_url_with_auth;
use base64::Engine;

// =============================================================================
// Ported from unit_tests/test_auth.py — TestParseUrlWithAuth
// =============================================================================

/// Ported from `test_parse_url_with_auth_none_input`.
#[test]
fn test_parse_url_with_auth_none_input() {
    let (url, headers) = parse_url_with_auth(None);
    assert!(url.is_none());
    assert!(headers.is_none());
}

/// Ported from `test_parse_url_with_auth_no_credentials`.
#[test]
fn test_parse_url_with_auth_no_credentials() {
    let input = "https://ollama.example.com:11434/path?query=param";
    let (url, headers) = parse_url_with_auth(Some(input));
    assert_eq!(url.as_deref(), Some(input));
    assert!(headers.is_none());
}

/// Ported from `test_parse_url_with_auth_with_credentials`.
#[test]
fn test_parse_url_with_auth_with_credentials() {
    let (url, headers) =
        parse_url_with_auth(Some("https://user:password@ollama.example.com:11434"));
    assert_eq!(url.as_deref(), Some("https://ollama.example.com:11434"));
    let headers = headers.expect("should have auth headers");
    let expected_creds = base64::engine::general_purpose::STANDARD.encode(b"user:password");
    assert_eq!(
        headers.get("Authorization"),
        Some(&format!("Basic {expected_creds}"))
    );
}

/// Ported from `test_parse_url_with_auth_with_path_and_query`.
#[test]
fn test_parse_url_with_auth_with_path_and_query() {
    let (url, headers) = parse_url_with_auth(Some(
        "https://user:pass@ollama.example.com:11434/api/v1?timeout=30",
    ));
    assert_eq!(
        url.as_deref(),
        Some("https://ollama.example.com:11434/api/v1?timeout=30")
    );
    let headers = headers.expect("should have auth headers");
    let expected_creds = base64::engine::general_purpose::STANDARD.encode(b"user:pass");
    assert_eq!(
        headers.get("Authorization"),
        Some(&format!("Basic {expected_creds}"))
    );
}

/// Ported from `test_parse_url_with_auth_special_characters`.
#[test]
fn test_parse_url_with_auth_special_characters() {
    let (url, headers) = parse_url_with_auth(Some(
        "https://user%40domain:p%40ssw0rd@ollama.example.com:11434",
    ));
    assert_eq!(url.as_deref(), Some("https://ollama.example.com:11434"));
    let headers = headers.expect("should have auth headers");
    let expected_creds = base64::engine::general_purpose::STANDARD.encode(b"user@domain:p@ssw0rd");
    assert_eq!(
        headers.get("Authorization"),
        Some(&format!("Basic {expected_creds}"))
    );
}

/// Ported from `test_parse_url_with_auth_only_username`.
#[test]
fn test_parse_url_with_auth_only_username() {
    let (url, headers) = parse_url_with_auth(Some("https://user@ollama.example.com:11434"));
    assert_eq!(url.as_deref(), Some("https://ollama.example.com:11434"));
    let headers = headers.expect("should have auth headers");
    let expected_creds = base64::engine::general_purpose::STANDARD.encode(b"user:");
    assert_eq!(
        headers.get("Authorization"),
        Some(&format!("Basic {expected_creds}"))
    );
}

/// Ported from `test_parse_url_with_auth_empty_password`.
#[test]
fn test_parse_url_with_auth_empty_password() {
    let (url, headers) = parse_url_with_auth(Some("https://user:@ollama.example.com:11434"));
    assert_eq!(url.as_deref(), Some("https://ollama.example.com:11434"));
    let headers = headers.expect("should have auth headers");
    let expected_creds = base64::engine::general_purpose::STANDARD.encode(b"user:");
    assert_eq!(
        headers.get("Authorization"),
        Some(&format!("Basic {expected_creds}"))
    );
}

// =============================================================================
// Ported from unit_tests/test_auth.py — TestUrlAuthEdgeCases
// =============================================================================

/// Ported from `test_parse_url_with_auth_malformed_url`.
#[test]
fn test_parse_url_with_auth_malformed_url() {
    let (url, headers) = parse_url_with_auth(Some("not-a-valid-url"));
    assert!(url.is_none(), "Malformed URL should return None");
    assert!(headers.is_none());
}

/// Ported from `test_parse_url_with_auth_no_port`.
#[test]
fn test_parse_url_with_auth_no_port() {
    let (url, headers) = parse_url_with_auth(Some("https://user:password@ollama.example.com"));
    assert_eq!(url.as_deref(), Some("https://ollama.example.com"));
    let headers = headers.expect("should have auth headers");
    let expected_creds = base64::engine::general_purpose::STANDARD.encode(b"user:password");
    assert_eq!(
        headers.get("Authorization"),
        Some(&format!("Basic {expected_creds}"))
    );
}

/// Ported from `test_parse_url_with_auth_complex_password`.
#[test]
fn test_parse_url_with_auth_complex_password() {
    let (url, headers) =
        parse_url_with_auth(Some("https://user:pass:word@ollama.example.com:11434"));
    assert_eq!(url.as_deref(), Some("https://ollama.example.com:11434"));
    let headers = headers.expect("should have auth headers");
    let expected_creds = base64::engine::general_purpose::STANDARD.encode(b"user:pass:word");
    assert_eq!(
        headers.get("Authorization"),
        Some(&format!("Basic {expected_creds}"))
    );
}

// =============================================================================
// Ported from unit_tests/test_auth.py — TestChatOllamaUrlAuth
//
// The Python tests mock the HTTP client to check that ChatOllama passes the
// cleaned URL and auth headers to the client. In Rust, we verify that
// build_request_payload uses the correct base_url when auth is present.
// =============================================================================

/// Ported from `test_chat_ollama_url_auth_integration`.
#[test]
fn test_chat_ollama_url_with_auth_strips_credentials() {
    let llm =
        ChatOllama::new("llama3.1").base_url("https://user:password@ollama.example.com:11434");
    let base_url = llm.get_base_url();
    assert_eq!(base_url, "https://ollama.example.com:11434");
}

use agent_chain::providers::ollama::{ChatOllama, OllamaEmbeddings, OllamaLLM, merge_auth_headers};

// =============================================================================
// Ported from unit_tests/test_auth.py — TestChatOllamaUrlAuth (continued)
// =============================================================================

/// Ported from `test_chat_ollama_url_auth_with_existing_headers`.
#[test]
fn test_chat_ollama_url_auth_with_existing_headers() {
    let url_with_auth = "https://user:password@ollama.example.com:11434";

    let (cleaned_url, auth_headers) = parse_url_with_auth(Some(url_with_auth));
    assert_eq!(
        cleaned_url.as_deref(),
        Some("https://ollama.example.com:11434")
    );

    let mut existing_headers = std::collections::HashMap::new();
    existing_headers.insert("User-Agent".to_string(), "test-agent".to_string());
    existing_headers.insert("X-Custom".to_string(), "value".to_string());

    merge_auth_headers(&mut existing_headers, auth_headers);

    assert_eq!(
        existing_headers.get("User-Agent"),
        Some(&"test-agent".to_string())
    );
    assert_eq!(existing_headers.get("X-Custom"), Some(&"value".to_string()));
    assert!(existing_headers.contains_key("Authorization"));
    let expected_creds = base64::engine::general_purpose::STANDARD.encode(b"user:password");
    assert_eq!(
        existing_headers.get("Authorization"),
        Some(&format!("Basic {expected_creds}"))
    );
}

// =============================================================================
// Ported from unit_tests/test_auth.py — TestOllamaLLMUrlAuth
// =============================================================================

/// Ported from `test_ollama_llm_url_auth_integration`.
#[test]
fn test_ollama_llm_url_with_auth_strips_credentials() {
    let llm = OllamaLLM::new("llama3.1").base_url("https://user:password@ollama.example.com:11434");
    let base_url = llm.get_base_url();
    assert_eq!(base_url, "https://ollama.example.com:11434");
}

// =============================================================================
// Ported from unit_tests/test_auth.py — TestOllamaEmbeddingsUrlAuth
// =============================================================================

/// Ported from `test_ollama_embeddings_url_auth_integration`.
#[test]
fn test_ollama_embeddings_url_with_auth_strips_credentials() {
    let embeddings = OllamaEmbeddings::new("llama3.1")
        .base_url("https://user:password@ollama.example.com:11434");
    let base_url = embeddings.get_base_url();
    assert_eq!(base_url, "https://ollama.example.com:11434");
}
