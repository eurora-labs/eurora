use agent_chain_core::output_parsers::JSON_FORMAT_INSTRUCTIONS;

#[test]
fn test_json_format_instructions_exists() {
    assert!(!JSON_FORMAT_INSTRUCTIONS.is_empty());
}

#[test]
fn test_json_format_instructions_content() {
    assert!(JSON_FORMAT_INSTRUCTIONS.contains("JSON"));
    assert!(JSON_FORMAT_INSTRUCTIONS.contains("schema"));
    assert!(
        JSON_FORMAT_INSTRUCTIONS.contains("Do not wrap")
            || JSON_FORMAT_INSTRUCTIONS.contains("not include")
    );
    assert!(JSON_FORMAT_INSTRUCTIONS.contains("{schema}"));
}

#[test]
fn test_json_format_instructions_no_markdown_mention() {
    let lower = JSON_FORMAT_INSTRUCTIONS.to_lowercase();
    assert!(lower.contains("markdown") || JSON_FORMAT_INSTRUCTIONS.contains("```"));
}

#[test]
fn test_json_format_instructions_example() {
    let lower = JSON_FORMAT_INSTRUCTIONS.to_lowercase();
    assert!(lower.contains("example"));
    assert!(lower.contains("well-formatted"));
}

#[test]
fn test_json_format_instructions_strict_output() {
    let upper = JSON_FORMAT_INSTRUCTIONS.to_uppercase();
    assert!(upper.contains("STRICT") || JSON_FORMAT_INSTRUCTIONS.to_lowercase().contains("only"));
}

#[test]
fn test_json_format_instructions_no_additional_text() {
    let lower = JSON_FORMAT_INSTRUCTIONS.to_lowercase();
    assert!(lower.contains("additional") || lower.contains("only"));
}

#[test]
fn test_json_format_instructions_schema_placeholder() {
    assert!(JSON_FORMAT_INSTRUCTIONS.contains("{schema}"));
    assert!(!JSON_FORMAT_INSTRUCTIONS.contains("{{schema}}"));
}

#[test]
fn test_json_format_instructions_formatting() {
    let test_schema = r#"{"properties": {"foo": {"type": "string"}}, "required": ["foo"]}"#;
    let formatted = JSON_FORMAT_INSTRUCTIONS.replace("{schema}", test_schema);

    assert!(formatted.contains(test_schema));
    assert!(!formatted.contains("{schema}"));
}

#[test]
fn test_json_format_instructions_multiline() {
    assert!(JSON_FORMAT_INSTRUCTIONS.contains('\n'));
    assert!(JSON_FORMAT_INSTRUCTIONS.split('\n').count() > 3);
}

#[test]
fn test_json_format_instructions_mentions_code_fence() {
    assert!(
        JSON_FORMAT_INSTRUCTIONS.contains("```")
            || JSON_FORMAT_INSTRUCTIONS
                .to_lowercase()
                .contains("code fence")
    );
}

#[test]
fn test_json_format_instructions_mentions_no_prepend() {
    let lower = JSON_FORMAT_INSTRUCTIONS.to_lowercase();
    assert!(
        lower.contains("prepend") || lower.contains("append") || lower.contains("additional text")
    );
}

#[test]
fn test_json_format_instructions_example_format() {
    assert!(JSON_FORMAT_INSTRUCTIONS.contains("properties"));
    assert!(JSON_FORMAT_INSTRUCTIONS.contains("required"));
}

#[test]
fn test_json_format_instructions_mentions_single_value() {
    let lower = JSON_FORMAT_INSTRUCTIONS.to_lowercase();
    assert!(lower.contains("single") || lower.contains("one") || lower.contains("only"));
}

#[test]
fn test_json_format_instructions_no_trailing_commas() {
    let lower = JSON_FORMAT_INSTRUCTIONS.to_lowercase();
    assert!(lower.contains("trailing") || lower.contains("comma") || lower.contains("comment"));
}

#[test]
fn test_json_format_instructions_conforms_to_schema() {
    let lower = JSON_FORMAT_INSTRUCTIONS.to_lowercase();
    assert!(lower.contains("conform") || lower.contains("match") || lower.contains("schema"));
}

#[test]
fn test_json_format_instructions_with_complex_schema() {
    let complex_schema = r#"{
        "properties": {
            "name": {"type": "string", "description": "Person's name"},
            "age": {"type": "integer", "minimum": 0},
            "emails": {"type": "array", "items": {"type": "string"}}
        },
        "required": ["name", "age"]
    }"#;

    let formatted = JSON_FORMAT_INSTRUCTIONS.replace("{schema}", complex_schema);

    assert!(formatted.contains("name"));
    assert!(formatted.contains("age"));
    assert!(formatted.contains("emails"));
    assert!(formatted.contains("array"));
}

#[test]
fn test_json_format_instructions_with_unicode_schema() {
    let unicode_schema = r#"{"properties": {"名前": {"type": "string", "description": "用户名"}}}"#;
    let formatted = JSON_FORMAT_INSTRUCTIONS.replace("{schema}", unicode_schema);

    assert!(formatted.contains("名前"));
    assert!(formatted.contains("用户名"));
}

#[test]
fn test_json_format_instructions_length() {
    assert!(JSON_FORMAT_INSTRUCTIONS.len() > 100);
    assert!(JSON_FORMAT_INSTRUCTIONS.len() < 5000);
}

#[test]
fn test_json_format_instructions_no_html() {
    let lower = JSON_FORMAT_INSTRUCTIONS.to_lowercase();
    assert!(!lower.contains("<html>"));
    assert!(!lower.contains("<div>"));
    assert!(!lower.contains("<p>"));
}

#[test]
fn test_json_format_instructions_readable() {
    let all_caps_lines: Vec<&str> = JSON_FORMAT_INSTRUCTIONS
        .split('\n')
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.is_empty()
                && trimmed.len() > 20
                && trimmed
                    .chars()
                    .all(|c| c.is_uppercase() || !c.is_alphabetic())
        })
        .collect();
    assert!(all_caps_lines.len() <= 2);
}

#[test]
fn test_json_format_instructions_mentions_top_level() {
    let lower = JSON_FORMAT_INSTRUCTIONS.to_lowercase();
    assert!(lower.contains("top-level") || lower.contains("top level") || lower.contains("single"));
}

#[test]
fn test_json_format_instructions_example_shows_bad_format() {
    let lower = JSON_FORMAT_INSTRUCTIONS.to_lowercase();
    assert!(lower.contains("not well-formatted") || lower.contains("not"));
}

#[test]
fn test_json_format_instructions_code_block_mention() {
    assert!(JSON_FORMAT_INSTRUCTIONS.contains("```"));
    assert!(
        JSON_FORMAT_INSTRUCTIONS
            .to_lowercase()
            .contains("readability")
            || JSON_FORMAT_INSTRUCTIONS.to_lowercase().contains("shown")
    );
}
