//! Utility functions for Ollama provider.
//!
//! Matches Python `langchain_ollama._utils`.

use std::collections::HashMap;

use base64::Engine;

use crate::error::{Error, Result};

/// Validate that a model exists in the local Ollama instance.
///
/// Matches Python `validate_model()` which calls `client.list()` and checks
/// for an exact match or tag-prefixed match.
pub async fn validate_model(
    client: &reqwest::Client,
    base_url: &str,
    model_name: &str,
) -> Result<()> {
    let response = client
        .get(format!("{}/api/tags", base_url))
        .send()
        .await
        .map_err(|e| {
            Error::Other(format!(
                "Failed to connect to Ollama. Please check that Ollama is downloaded, \
                 running and accessible. https://ollama.com/download. Error: {}",
                e
            ))
        })?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(Error::Other(format!(
            "Received an error from the Ollama API. \
             Please check your Ollama server logs. {}",
            error_text
        )));
    }

    let body: serde_json::Value = response.json().await.map_err(|e| {
        Error::Json(serde_json::Error::io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            e.to_string(),
        )))
    })?;

    let model_names: Vec<&str> = body
        .get("models")
        .and_then(|m| m.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|m| m.get("model").and_then(|n| n.as_str()))
                .collect()
        })
        .unwrap_or_default();

    let found = model_names
        .iter()
        .any(|m| *m == model_name || m.starts_with(&format!("{}:", model_name)));

    if !found {
        let available = model_names.join(", ");
        return Err(Error::Other(format!(
            "Model `{}` not found in Ollama. Please pull the model \
             (using `ollama pull {}`) or specify a valid model name. \
             Available local models: {}",
            model_name, model_name, available
        )));
    }

    Ok(())
}

/// Parse URL and extract `userinfo` credentials for headers.
///
/// Handles URLs of the form: `https://user:password@host:port/path`
///
/// Returns `(cleaned_url, headers)` where:
/// - `cleaned_url` is the URL without authentication credentials if any were
///   found. Otherwise, returns the original URL.
/// - `headers` contains Authorization header if credentials were found.
///
/// Matches Python `parse_url_with_auth()`.
pub fn parse_url_with_auth(url: Option<&str>) -> (Option<String>, Option<HashMap<String, String>>) {
    let Some(url) = url else {
        return (None, None);
    };

    let parsed = match url::Url::parse(url) {
        Ok(u) => u,
        Err(_) => return (None, None),
    };

    if parsed.scheme().is_empty() || parsed.host_str().is_none() {
        return (None, None);
    }

    let username = parsed.username();
    if username.is_empty() {
        return (Some(url.to_string()), None);
    }

    let password = parsed.password().unwrap_or("");

    // Decode percent-encoding
    let username = percent_encoding::percent_decode_str(username)
        .decode_utf8()
        .map(|s| s.into_owned())
        .unwrap_or_else(|_| username.to_string());
    let password = percent_encoding::percent_decode_str(password)
        .decode_utf8()
        .map(|s| s.into_owned())
        .unwrap_or_else(|_| password.to_string());

    let credentials = format!("{}:{}", username, password);
    let encoded_credentials = base64::engine::general_purpose::STANDARD.encode(credentials);
    let mut headers = HashMap::new();
    headers.insert(
        "Authorization".to_string(),
        format!("Basic {}", encoded_credentials),
    );

    // Rebuild URL without credentials
    let host = parsed.host_str().unwrap_or("");
    let mut cleaned_url = format!("{}://{}", parsed.scheme(), host);
    if let Some(port) = parsed.port() {
        cleaned_url.push_str(&format!(":{}", port));
    }
    if !parsed.path().is_empty() && parsed.path() != "/" {
        cleaned_url.push_str(parsed.path());
    }
    if let Some(query) = parsed.query() {
        cleaned_url.push_str(&format!("?{}", query));
    }
    if let Some(fragment) = parsed.fragment() {
        cleaned_url.push_str(&format!("#{}", fragment));
    }

    (Some(cleaned_url), Some(headers))
}

/// Merge authentication headers into client kwargs headers in-place.
///
/// Matches Python `merge_auth_headers()`.
pub fn merge_auth_headers(
    headers: &mut HashMap<String, String>,
    auth_headers: Option<HashMap<String, String>>,
) {
    if let Some(auth) = auth_headers {
        headers.extend(auth);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_url_with_auth_none_input() {
        let (url, headers) = parse_url_with_auth(None);
        assert!(url.is_none());
        assert!(headers.is_none());
    }

    #[test]
    fn test_parse_url_with_auth_no_credentials() {
        let url = "https://ollama.example.com:11434/path?query=param";
        let (cleaned_url, headers) = parse_url_with_auth(Some(url));
        assert_eq!(cleaned_url.as_deref(), Some(url));
        assert!(headers.is_none());
    }

    #[test]
    fn test_parse_url_with_auth_with_credentials() {
        let url = "https://user:password@ollama.example.com:11434";
        let (cleaned_url, headers) = parse_url_with_auth(Some(url));

        assert_eq!(
            cleaned_url.as_deref(),
            Some("https://ollama.example.com:11434")
        );
        let expected_credentials =
            base64::engine::general_purpose::STANDARD.encode("user:password");
        let headers = headers.unwrap();
        assert_eq!(
            headers.get("Authorization"),
            Some(&format!("Basic {}", expected_credentials))
        );
    }

    #[test]
    fn test_parse_url_with_auth_with_path_and_query() {
        let url = "https://user:pass@ollama.example.com:11434/api/v1?timeout=30";
        let (cleaned_url, headers) = parse_url_with_auth(Some(url));

        assert_eq!(
            cleaned_url.as_deref(),
            Some("https://ollama.example.com:11434/api/v1?timeout=30")
        );
        let expected_credentials = base64::engine::general_purpose::STANDARD.encode("user:pass");
        let headers = headers.unwrap();
        assert_eq!(
            headers.get("Authorization"),
            Some(&format!("Basic {}", expected_credentials))
        );
    }

    #[test]
    fn test_parse_url_with_auth_special_characters() {
        let url = "https://user%40domain:p%40ssw0rd@ollama.example.com:11434";
        let (cleaned_url, headers) = parse_url_with_auth(Some(url));

        assert_eq!(
            cleaned_url.as_deref(),
            Some("https://ollama.example.com:11434")
        );
        let expected_credentials =
            base64::engine::general_purpose::STANDARD.encode("user@domain:p@ssw0rd");
        let headers = headers.unwrap();
        assert_eq!(
            headers.get("Authorization"),
            Some(&format!("Basic {}", expected_credentials))
        );
    }

    #[test]
    fn test_parse_url_with_auth_only_username() {
        let url = "https://user@ollama.example.com:11434";
        let (cleaned_url, headers) = parse_url_with_auth(Some(url));

        assert_eq!(
            cleaned_url.as_deref(),
            Some("https://ollama.example.com:11434")
        );
        let expected_credentials = base64::engine::general_purpose::STANDARD.encode("user:");
        let headers = headers.unwrap();
        assert_eq!(
            headers.get("Authorization"),
            Some(&format!("Basic {}", expected_credentials))
        );
    }

    #[test]
    fn test_parse_url_with_auth_malformed_url() {
        let (url, headers) = parse_url_with_auth(Some("not-a-valid-url"));
        assert!(url.is_none());
        assert!(headers.is_none());
    }

    #[test]
    fn test_parse_url_with_auth_no_port() {
        let url = "https://user:password@ollama.example.com";
        let (cleaned_url, headers) = parse_url_with_auth(Some(url));

        assert_eq!(cleaned_url.as_deref(), Some("https://ollama.example.com"));
        let expected_credentials =
            base64::engine::general_purpose::STANDARD.encode("user:password");
        let headers = headers.unwrap();
        assert_eq!(
            headers.get("Authorization"),
            Some(&format!("Basic {}", expected_credentials))
        );
    }

    #[test]
    fn test_merge_auth_headers() {
        let mut headers = HashMap::new();
        headers.insert("User-Agent".to_string(), "test-agent".to_string());

        let mut auth_headers = HashMap::new();
        auth_headers.insert("Authorization".to_string(), "Basic abc123".to_string());

        merge_auth_headers(&mut headers, Some(auth_headers));

        assert_eq!(headers.get("User-Agent"), Some(&"test-agent".to_string()));
        assert_eq!(
            headers.get("Authorization"),
            Some(&"Basic abc123".to_string())
        );
    }

    #[test]
    fn test_merge_auth_headers_none() {
        let mut headers = HashMap::new();
        headers.insert("User-Agent".to_string(), "test-agent".to_string());

        merge_auth_headers(&mut headers, None);

        assert_eq!(headers.len(), 1);
        assert_eq!(headers.get("User-Agent"), Some(&"test-agent".to_string()));
    }
}
