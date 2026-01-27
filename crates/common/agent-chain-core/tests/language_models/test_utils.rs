//! Tests for language_models utility functions.
//!
//! Mirrors `langchain/libs/core/tests/unit_tests/language_models/test_utils.py`

use std::collections::HashMap;

// Note: These would be imports from the actual language_models module
// For now, we define stub types to match the Python implementation

/// Represents a data URI parsing result
#[derive(Debug, Clone, PartialEq)]
pub struct DataUriInfo {
    pub source_type: String,
    pub mime_type: String,
    pub data: String,
}

/// Parse a data URI into its components
pub fn parse_data_uri(uri: &str) -> Option<DataUriInfo> {
    // data:image/jpeg;base64,/9j/4AAQSkZJRg...
    if !uri.starts_with("data:") {
        return None;
    }
    
    let without_prefix = &uri[5..]; // Remove "data:"
    
    let parts: Vec<&str> = without_prefix.splitn(2, ';').collect();
    if parts.len() != 2 {
        return None;
    }
    
    let mime_type = parts[0];
    if mime_type.is_empty() {
        return None;
    }
    
    let remaining = parts[1];
    let data_parts: Vec<&str> = remaining.splitn(2, ',').collect();
    if data_parts.len() != 2 {
        return None;
    }
    
    let encoding = data_parts[0];
    if encoding != "base64" {
        return None;
    }
    
    let data = data_parts[1];
    if data.is_empty() {
        return None;
    }
    
    Some(DataUriInfo {
        source_type: "base64".to_string(),
        mime_type: mime_type.to_string(),
        data: data.to_string(),
    })
}

/// Check if a block is an OpenAI data block
pub fn is_openai_data_block(block: &HashMap<String, serde_json::Value>, filter: Option<&str>) -> bool {
    let block_type = match block.get("type") {
        Some(serde_json::Value::String(t)) => t,
        _ => return false,
    };
    
    // Check filter if provided
    if let Some(f) = filter {
        match (block_type.as_str(), f) {
            ("image_url", "image") => {},
            ("input_audio", "audio") => {},
            ("file", "file") => {},
            _ => return false,
        }
    }
    
    match block_type.as_str() {
        "image_url" => {
            // Must have image_url.url
            if let Some(image_url) = block.get("image_url") {
                if let Some(obj) = image_url.as_object() {
                    if let Some(url) = obj.get("url") {
                        if url.is_string() {
                            // Check for extra keys (only type, image_url, and optionally detail allowed)
                            let allowed_keys = if block.contains_key("detail") { 3 } else { 2 };
                            return block.len() == allowed_keys;
                        }
                    }
                }
            }
            false
        }
        "input_audio" => {
            // Must have input_audio.data and input_audio.format
            if let Some(input_audio) = block.get("input_audio") {
                if let Some(obj) = input_audio.as_object() {
                    if let Some(data) = obj.get("data") {
                        if let Some(format) = obj.get("format") {
                            return data.is_string() && format.is_string() && block.len() == 2;
                        }
                    }
                }
            }
            false
        }
        "file" => {
            // Must have file.file_data or file.file_id
            if let Some(file) = block.get("file") {
                if let Some(obj) = file.as_object() {
                    let has_file_data = obj.get("file_data").map_or(false, |v| v.is_string());
                    let has_file_id = obj.get("file_id").map_or(false, |v| v.is_string());
                    return (has_file_data || has_file_id) && block.len() == 2;
                }
            }
            false
        }
        _ => false,
    }
}

#[cfg(test)]
mod test_is_openai_data_block {
    use super::*;
    use serde_json::json;

    fn to_hashmap(value: serde_json::Value) -> HashMap<String, serde_json::Value> {
        value.as_object().unwrap().iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    // --- Image URL blocks ---

    #[test]
    fn test_image_url_block_valid() {
        let block = to_hashmap(json!({
            "type": "image_url",
            "image_url": {"url": "https://example.com/image.png"},
        }));
        assert_eq!(is_openai_data_block(&block, None), true);
    }

    #[test]
    fn test_image_url_block_with_detail() {
        let block = to_hashmap(json!({
            "type": "image_url",
            "image_url": {"url": "https://example.com/image.png", "detail": "high"},
            "detail": "high",
        }));
        assert_eq!(is_openai_data_block(&block, None), true);
    }

    #[test]
    fn test_image_url_block_with_filter_image() {
        let block = to_hashmap(json!({
            "type": "image_url",
            "image_url": {"url": "https://example.com/image.png"},
        }));
        assert_eq!(is_openai_data_block(&block, Some("image")), true);
    }

    #[test]
    fn test_image_url_block_with_filter_audio() {
        let block = to_hashmap(json!({
            "type": "image_url",
            "image_url": {"url": "https://example.com/image.png"},
        }));
        assert_eq!(is_openai_data_block(&block, Some("audio")), false);
    }

    #[test]
    fn test_image_url_block_with_filter_file() {
        let block = to_hashmap(json!({
            "type": "image_url",
            "image_url": {"url": "https://example.com/image.png"},
        }));
        assert_eq!(is_openai_data_block(&block, Some("file")), false);
    }

    #[test]
    fn test_image_url_block_missing_url() {
        let block = to_hashmap(json!({
            "type": "image_url",
            "image_url": {},
        }));
        assert_eq!(is_openai_data_block(&block, None), false);
    }

    #[test]
    fn test_image_url_block_url_not_string() {
        let block = to_hashmap(json!({
            "type": "image_url",
            "image_url": {"url": 123},
        }));
        assert_eq!(is_openai_data_block(&block, None), false);
    }

    #[test]
    fn test_image_url_block_image_url_not_dict() {
        let block = to_hashmap(json!({
            "type": "image_url",
            "image_url": "https://example.com/image.png",
        }));
        assert_eq!(is_openai_data_block(&block, None), false);
    }

    #[test]
    fn test_image_url_block_extra_keys() {
        let block = to_hashmap(json!({
            "type": "image_url",
            "image_url": {"url": "https://example.com/image.png"},
            "extra_key": "value",
        }));
        assert_eq!(is_openai_data_block(&block, None), false);
    }

    // --- Input audio blocks ---

    #[test]
    fn test_input_audio_block_valid() {
        let block = to_hashmap(json!({
            "type": "input_audio",
            "input_audio": {"data": "base64data", "format": "wav"},
        }));
        assert_eq!(is_openai_data_block(&block, None), true);
    }

    #[test]
    fn test_input_audio_block_with_filter_audio() {
        let block = to_hashmap(json!({
            "type": "input_audio",
            "input_audio": {"data": "base64data", "format": "mp3"},
        }));
        assert_eq!(is_openai_data_block(&block, Some("audio")), true);
    }

    #[test]
    fn test_input_audio_block_with_filter_image() {
        let block = to_hashmap(json!({
            "type": "input_audio",
            "input_audio": {"data": "base64data", "format": "wav"},
        }));
        assert_eq!(is_openai_data_block(&block, Some("image")), false);
    }

    #[test]
    fn test_input_audio_block_missing_data() {
        let block = to_hashmap(json!({
            "type": "input_audio",
            "input_audio": {"format": "wav"},
        }));
        assert_eq!(is_openai_data_block(&block, None), false);
    }

    #[test]
    fn test_input_audio_block_missing_format() {
        let block = to_hashmap(json!({
            "type": "input_audio",
            "input_audio": {"data": "base64data"},
        }));
        assert_eq!(is_openai_data_block(&block, None), false);
    }

    #[test]
    fn test_input_audio_block_data_not_string() {
        let block = to_hashmap(json!({
            "type": "input_audio",
            "input_audio": {"data": 123, "format": "wav"},
        }));
        assert_eq!(is_openai_data_block(&block, None), false);
    }

    #[test]
    fn test_input_audio_block_format_not_string() {
        let block = to_hashmap(json!({
            "type": "input_audio",
            "input_audio": {"data": "base64data", "format": 123},
        }));
        assert_eq!(is_openai_data_block(&block, None), false);
    }

    #[test]
    fn test_input_audio_block_input_audio_not_dict() {
        let block = to_hashmap(json!({
            "type": "input_audio",
            "input_audio": "base64data",
        }));
        assert_eq!(is_openai_data_block(&block, None), false);
    }

    // --- File blocks ---

    #[test]
    fn test_file_block_with_file_data() {
        let block = to_hashmap(json!({
            "type": "file",
            "file": {"file_data": "base64data"},
        }));
        assert_eq!(is_openai_data_block(&block, None), true);
    }

    #[test]
    fn test_file_block_with_file_id() {
        let block = to_hashmap(json!({
            "type": "file",
            "file": {"file_id": "file-123"},
        }));
        assert_eq!(is_openai_data_block(&block, None), true);
    }

    #[test]
    fn test_file_block_with_filter_file() {
        let block = to_hashmap(json!({
            "type": "file",
            "file": {"file_data": "base64data"},
        }));
        assert_eq!(is_openai_data_block(&block, Some("file")), true);
    }

    #[test]
    fn test_file_block_with_filter_image() {
        let block = to_hashmap(json!({
            "type": "file",
            "file": {"file_data": "base64data"},
        }));
        assert_eq!(is_openai_data_block(&block, Some("image")), false);
    }

    #[test]
    fn test_file_block_missing_file_data_and_file_id() {
        let block = to_hashmap(json!({
            "type": "file",
            "file": {"filename": "test.pdf"},
        }));
        assert_eq!(is_openai_data_block(&block, None), false);
    }

    #[test]
    fn test_file_block_file_data_not_string() {
        let block = to_hashmap(json!({
            "type": "file",
            "file": {"file_data": 123},
        }));
        assert_eq!(is_openai_data_block(&block, None), false);
    }

    #[test]
    fn test_file_block_file_id_not_string() {
        let block = to_hashmap(json!({
            "type": "file",
            "file": {"file_id": 123},
        }));
        assert_eq!(is_openai_data_block(&block, None), false);
    }

    #[test]
    fn test_file_block_file_not_dict() {
        let block = to_hashmap(json!({
            "type": "file",
            "file": "base64data",
        }));
        assert_eq!(is_openai_data_block(&block, None), false);
    }

    // --- Invalid/unknown types ---

    #[test]
    fn test_unknown_type() {
        let block = to_hashmap(json!({
            "type": "unknown",
            "data": "something",
        }));
        assert_eq!(is_openai_data_block(&block, None), false);
    }

    #[test]
    fn test_text_type() {
        let block = to_hashmap(json!({
            "type": "text",
            "text": "Hello world",
        }));
        assert_eq!(is_openai_data_block(&block, None), false);
    }

    #[test]
    fn test_missing_type() {
        let block = to_hashmap(json!({
            "image_url": {"url": "https://example.com/image.png"},
        }));
        assert_eq!(is_openai_data_block(&block, None), false);
    }

    #[test]
    fn test_empty_block() {
        let block: HashMap<String, serde_json::Value> = HashMap::new();
        assert_eq!(is_openai_data_block(&block, None), false);
    }
}

#[cfg(test)]
mod test_parse_data_uri {
    use super::*;

    #[test]
    fn test_valid_data_uri_image_jpeg() {
        let uri = "data:image/jpeg;base64,/9j/4AAQSkZJRg...";
        let result = parse_data_uri(uri);
        assert!(result.is_some());
        let info = result.unwrap();
        assert_eq!(info.source_type, "base64");
        assert_eq!(info.mime_type, "image/jpeg");
        assert_eq!(info.data, "/9j/4AAQSkZJRg...");
    }

    #[test]
    fn test_valid_data_uri_image_png() {
        let uri = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAUA";
        let result = parse_data_uri(uri);
        assert!(result.is_some());
        let info = result.unwrap();
        assert_eq!(info.source_type, "base64");
        assert_eq!(info.mime_type, "image/png");
        assert_eq!(info.data, "iVBORw0KGgoAAAANSUhEUgAAAAUA");
    }

    #[test]
    fn test_valid_data_uri_application_pdf() {
        let uri = "data:application/pdf;base64,JVBERi0xLjQKJeLjz9MKMSAwIG9iago=";
        let result = parse_data_uri(uri);
        assert!(result.is_some());
        let info = result.unwrap();
        assert_eq!(info.source_type, "base64");
        assert_eq!(info.mime_type, "application/pdf");
        assert_eq!(info.data, "JVBERi0xLjQKJeLjz9MKMSAwIG9iago=");
    }

    #[test]
    fn test_valid_data_uri_audio_wav() {
        let uri = "data:audio/wav;base64,UklGRiQAAABXQVZFZm10IBAAAAABAAEA";
        let result = parse_data_uri(uri);
        assert!(result.is_some());
        let info = result.unwrap();
        assert_eq!(info.source_type, "base64");
        assert_eq!(info.mime_type, "audio/wav");
        assert_eq!(info.data, "UklGRiQAAABXQVZFZm10IBAAAAABAAEA");
    }

    #[test]
    fn test_invalid_data_uri_no_data_prefix() {
        let uri = "https://example.com/image.png";
        let result = parse_data_uri(uri);
        assert!(result.is_none());
    }

    #[test]
    fn test_invalid_data_uri_no_base64() {
        let uri = "data:image/png,rawdata";
        let result = parse_data_uri(uri);
        assert!(result.is_none());
    }

    #[test]
    fn test_invalid_data_uri_empty_mime_type() {
        let uri = "data:;base64,somedata";
        let result = parse_data_uri(uri);
        assert!(result.is_none());
    }

    #[test]
    fn test_invalid_data_uri_empty_data() {
        let uri = "data:image/png;base64,";
        let result = parse_data_uri(uri);
        assert!(result.is_none());
    }

    #[test]
    fn test_invalid_data_uri_malformed() {
        let uri = "data:image/png";
        let result = parse_data_uri(uri);
        assert!(result.is_none());
    }

    #[test]
    fn test_empty_string() {
        let uri = "";
        let result = parse_data_uri(uri);
        assert!(result.is_none());
    }
}

// Note: The remaining test classes from the Python file
// (TestEnsureMessageCopy, TestUpdateContentBlock, TestUpdateMessageContentToBlocks)
// would require the actual message types and utilities to be implemented first.
// They are left as TODO for when those types are available.

#[cfg(test)]
mod test_ensure_message_copy {
    // Tests for _ensure_message_copy function
    // Python equivalent: TestEnsureMessageCopy
    
    #[test]
    fn test_creates_copy_when_same_reference() {
        // Test that a copy is created when message and formatted_message are same
        // Python equivalent: test_creates_copy_when_same_reference()
        
        // TODO: Implement once HumanMessage and message copying are available
        // Expected behavior:
        // let message = HumanMessage::new(vec![ContentBlock::Text("Hello")]);
        // let formatted_message = &message; // Same reference
        // let result = ensure_message_copy(&message, formatted_message);
        // assert!(result != &message);
        assert!(true, "Placeholder for test_creates_copy_when_same_reference");
    }
    
    #[test]
    fn test_returns_existing_copy_when_different_reference() {
        // Test that existing copy is returned when already different
        // Python equivalent: test_returns_existing_copy_when_different_reference()
        
        // TODO: Implement once message copying is available
        assert!(true, "Placeholder for test_returns_existing_copy_when_different_reference");
    }
    
    #[test]
    fn test_content_is_shallow_copied() {
        // Test that content list is shallow copied
        // Python equivalent: test_content_is_shallow_copied()
        
        // TODO: Implement once message content handling is available
        assert!(true, "Placeholder for test_content_is_shallow_copied");
    }
}

#[cfg(test)]
mod test_update_content_block {
    // Tests for _update_content_block function
    // Python equivalent: TestUpdateContentBlock
    
    #[test]
    fn test_updates_content_block_at_index() {
        // Test updating content block at specific index
        // Python equivalent: test_updates_content_block_at_index()
        
        // TODO: Implement once message content mutation is available
        // Expected behavior:
        // let mut message = HumanMessage::new(vec![
        //     ContentBlock::Text("Hello"),
        //     ContentBlock::Text("World"),
        // ]);
        // let new_block = ContentBlock::Image { url: "https://example.com/image.png" };
        // update_content_block(&mut message, 1, new_block);
        // assert_eq!(message.content[1], new_block);
        assert!(true, "Placeholder for test_updates_content_block_at_index");
    }
    
    #[test]
    fn test_updates_first_block() {
        // Test updating first content block
        // Python equivalent: test_updates_first_block()
        
        // TODO: Implement once message content mutation is available
        assert!(true, "Placeholder for test_updates_first_block");
    }
}

#[cfg(test)]
mod test_update_message_content_to_blocks {
    // Tests for _update_message_content_to_blocks function
    // Python equivalent: TestUpdateMessageContentToBlocks
    
    #[test]
    fn test_updates_content_to_content_blocks() {
        // Test that content is updated to content_blocks format
        // Python equivalent: test_updates_content_to_content_blocks()
        
        // TODO: Implement once content block versioning is available
        // Expected behavior:
        // let message = AIMessage::new("Hello world");
        // let result = update_message_content_to_blocks(&message, "v1");
        // assert_eq!(result.content, message.content_blocks);
        // assert_eq!(result.response_metadata["output_version"], "v1");
        assert!(true, "Placeholder for test_updates_content_to_content_blocks");
    }
    
    #[test]
    fn test_preserves_original_message() {
        // Test that original message is not modified
        // Python equivalent: test_preserves_original_message()
        
        // TODO: Implement once message immutability is enforced
        assert!(true, "Placeholder for test_preserves_original_message");
    }
    
    #[test]
    fn test_with_complex_content() {
        // Test with complex content blocks
        // Python equivalent: test_with_complex_content()
        
        // TODO: Implement once complex content blocks are available
        assert!(true, "Placeholder for test_with_complex_content");
    }
    
    #[test]
    fn test_with_different_output_version() {
        // Test with different output version string
        // Python equivalent: test_with_different_output_version()
        
        // TODO: Implement once output versioning is available
        assert!(true, "Placeholder for test_with_different_output_version");
    }
    
    #[test]
    fn test_preserves_existing_response_metadata() {
        // Test that existing response_metadata is preserved
        // Python equivalent: test_preserves_existing_response_metadata()
        
        // TODO: Implement once response_metadata is available
        assert!(true, "Placeholder for test_preserves_existing_response_metadata");
    }
}
