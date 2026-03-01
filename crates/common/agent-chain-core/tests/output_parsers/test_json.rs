use agent_chain_core::messages::BaseMessage;
use agent_chain_core::output_parsers::{
    BaseOutputParser, BaseTransformOutputParser, SimpleJsonOutputParser,
};
use agent_chain_core::utils::json::{
    parse_and_check_json_markdown, parse_json_markdown, parse_partial_json,
};
use futures::StreamExt;
use serde_json::{Value, json};

const GOOD_JSON: &str = r#"```json
{
    "foo": "bar"
}
```"#;

const JSON_WITH_NEW_LINES: &str = "\n```json\n{\n    \"foo\": \"bar\"\n}\n```\n\n";

const JSON_WITH_NEW_LINES_INSIDE: &str = "```json\n{\n\n    \"foo\": \"bar\"\n\n}\n```";

const JSON_WITH_NEW_LINES_EVERYWHERE: &str =
    "\n```json\n\n{\n\n    \"foo\": \"bar\"\n\n}\n\n```\n\n";

const TICKS_WITH_NEW_LINES_EVERYWHERE: &str = "\n```\n\n{\n\n    \"foo\": \"bar\"\n\n}\n\n```\n\n";

const JSON_WITH_MARKDOWN_CODE_BLOCK: &str = "```json\n{\n    \"foo\": \"```bar```\"\n}\n```";

const JSON_WITH_PART_MARKDOWN_CODE_BLOCK: &str =
    "\n{\"valid_json\": \"hey ```print(hello world!)``` hey\"}\n";

const JSON_WITH_MARKDOWN_CODE_BLOCK_AND_NEWLINES: &str = "```json\n{\n    \"action\": \"Final Answer\",\n    \"action_input\": \"```bar\n<div id=\\\"1\\\" class=\\\"value\\\">\n\ttext\n</div>```\"\n}\n```";

const JSON_WITH_PYTHON_DICT: &str = "```json\n{\n    \"action\": \"Final Answer\",\n    \"action_input\": {\"foo\": \"bar\", \"bar\": \"foo\"}\n}\n```";

const JSON_WITH_ESCAPED_DOUBLE_QUOTES_IN_NESTED_JSON: &str = "```json\n{\n    \"action\": \"Final Answer\",\n    \"action_input\": \"{\\\"foo\\\": \\\"bar\\\", \\\"bar\\\": \\\"foo\\\"}\"\n}\n```";

const NO_TICKS: &str = "{\n    \"foo\": \"bar\"\n}";

const NO_TICKS_WHITE_SPACE: &str = "\n{\n    \"foo\": \"bar\"\n}\n";

const TEXT_BEFORE: &str =
    "Thought: I need to use the search tool\n\nAction:\n```\n{\n  \"foo\": \"bar\"\n}\n```";

const TEXT_AFTER: &str = "```\n{\n  \"foo\": \"bar\"\n}\n```\nThis should do the trick";

const TEXT_BEFORE_AND_AFTER: &str =
    "Action: Testing\n\n```\n{\n  \"foo\": \"bar\"\n}\n```\nThis should do the trick";

const WITHOUT_END_BRACKET: &str =
    "Here is a response formatted as schema:\n\n```json\n{\n  \"foo\": \"bar\"\n\n\n";

const WITH_END_BRACKET: &str =
    "Here is a response formatted as schema:\n\n```json\n{\n  \"foo\": \"bar\"\n}\n\n";

const WITH_END_TICK: &str =
    "Here is a response formatted as schema:\n\n```json\n{\n  \"foo\": \"bar\"\n}\n```\n";

const WITH_END_TEXT: &str = "Here is a response formatted as schema:\n\n```\n{\n  \"foo\": \"bar\"\n\n```\nThis should do the trick\n";

fn assert_parse_json_foo_bar(json_string: &str) {
    let parsed = parse_json_markdown(json_string).expect("should parse JSON");
    assert_eq!(
        parsed,
        json!({"foo": "bar"}),
        "Failed for input:\n{json_string}"
    );
}

#[test]
fn test_parse_json_good_json() {
    assert_parse_json_foo_bar(GOOD_JSON);
}

#[test]
fn test_parse_json_with_new_lines() {
    assert_parse_json_foo_bar(JSON_WITH_NEW_LINES);
}

#[test]
fn test_parse_json_with_new_lines_inside() {
    assert_parse_json_foo_bar(JSON_WITH_NEW_LINES_INSIDE);
}

#[test]
fn test_parse_json_with_new_lines_everywhere() {
    assert_parse_json_foo_bar(JSON_WITH_NEW_LINES_EVERYWHERE);
}

#[test]
fn test_parse_json_ticks_with_new_lines_everywhere() {
    assert_parse_json_foo_bar(TICKS_WITH_NEW_LINES_EVERYWHERE);
}

#[test]
fn test_parse_json_no_ticks() {
    assert_parse_json_foo_bar(NO_TICKS);
}

#[test]
fn test_parse_json_no_ticks_white_space() {
    assert_parse_json_foo_bar(NO_TICKS_WHITE_SPACE);
}

#[test]
fn test_parse_json_text_before() {
    assert_parse_json_foo_bar(TEXT_BEFORE);
}

#[test]
fn test_parse_json_text_after() {
    assert_parse_json_foo_bar(TEXT_AFTER);
}

#[test]
fn test_parse_json_text_before_and_after() {
    assert_parse_json_foo_bar(TEXT_BEFORE_AND_AFTER);
}

#[test]
fn test_parse_json_without_end_bracket() {
    assert_parse_json_foo_bar(WITHOUT_END_BRACKET);
}

#[test]
fn test_parse_json_with_end_bracket() {
    assert_parse_json_foo_bar(WITH_END_BRACKET);
}

#[test]
fn test_parse_json_with_end_tick() {
    assert_parse_json_foo_bar(WITH_END_TICK);
}

#[test]
fn test_parse_json_with_end_text() {
    assert_parse_json_foo_bar(WITH_END_TEXT);
}

#[test]
fn test_parse_json_with_code_blocks() {
    let parsed = parse_json_markdown(JSON_WITH_MARKDOWN_CODE_BLOCK).expect("should parse JSON");
    assert_eq!(parsed, json!({"foo": "```bar```"}));
}

#[test]
fn test_parse_json_with_part_code_blocks() {
    let parsed =
        parse_json_markdown(JSON_WITH_PART_MARKDOWN_CODE_BLOCK).expect("should parse JSON");
    assert_eq!(
        parsed,
        json!({"valid_json": "hey ```print(hello world!)``` hey"})
    );
}

#[test]
fn test_parse_json_with_code_blocks_and_newlines() {
    let parsed =
        parse_json_markdown(JSON_WITH_MARKDOWN_CODE_BLOCK_AND_NEWLINES).expect("should parse JSON");
    assert_eq!(
        parsed,
        json!({
            "action": "Final Answer",
            "action_input": "```bar\n<div id=\"1\" class=\"value\">\n\ttext\n</div>```"
        })
    );
}

#[test]
fn test_parse_non_dict_json_output() {
    let text = "```json\n1\n```";
    let result = parse_and_check_json_markdown(text, &["foo"]);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("Expected JSON object (dict)"),
        "Error should mention expected dict, got: {err_msg}"
    );
}

#[test]
fn test_parse_nested_json_with_escaped_quotes() {
    let parsed = parse_json_markdown(JSON_WITH_ESCAPED_DOUBLE_QUOTES_IN_NESTED_JSON)
        .expect("should parse JSON");
    assert_eq!(
        parsed,
        json!({
            "action": "Final Answer",
            "action_input": "{\"foo\": \"bar\", \"bar\": \"foo\"}"
        })
    );
}

#[test]
fn test_parse_json_with_python_dict() {
    let parsed = parse_json_markdown(JSON_WITH_PYTHON_DICT).expect("should parse JSON");
    assert_eq!(
        parsed,
        json!({
            "action": "Final Answer",
            "action_input": {"foo": "bar", "bar": "foo"}
        })
    );
}

#[test]
fn test_parse_partial_json_complete_object() {
    let parsed = parse_partial_json(r#"{"foo": "bar", "bar": "foo"}"#, false).unwrap();
    assert_eq!(parsed, json!({"foo": "bar", "bar": "foo"}));
}

#[test]
fn test_parse_partial_json_missing_closing_quote() {
    let parsed = parse_partial_json(r#"{"foo": "bar", "bar": "foo"#, false).unwrap();
    assert_eq!(parsed, json!({"foo": "bar", "bar": "foo"}));
}

#[test]
fn test_parse_partial_json_unclosed_brace_in_value() {
    let parsed = parse_partial_json(r#"{"foo": "bar", "bar": "foo}"#, false).unwrap();
    assert_eq!(parsed, json!({"foo": "bar", "bar": "foo}"}));
}

#[test]
fn test_parse_partial_json_unclosed_bracket_in_value() {
    let parsed = parse_partial_json(r#"{"foo": "bar", "bar": "foo["#, false).unwrap();
    assert_eq!(parsed, json!({"foo": "bar", "bar": "foo["}));
}

#[test]
fn test_parse_partial_json_escaped_quote_in_value() {
    let parsed = parse_partial_json(r#"{"foo": "bar", "bar": "foo\""#, false).unwrap();
    assert_eq!(parsed, json!({"foo": "bar", "bar": "foo\""}));
}

#[test]
fn test_parse_partial_json_trailing_colon() {
    let parsed = parse_partial_json(r#"{"foo": "bar", "bar":"#, false).unwrap();
    assert_eq!(parsed, json!({"foo": "bar"}));
}

#[test]
fn test_parse_partial_json_trailing_key() {
    let parsed = parse_partial_json(r#"{"foo": "bar", "bar""#, false).unwrap();
    assert_eq!(parsed, json!({"foo": "bar"}));
}

#[test]
fn test_parse_partial_json_trailing_comma() {
    let parsed = parse_partial_json(r#"{"foo": "bar", "#, false).unwrap();
    assert_eq!(parsed, json!({"foo": "bar"}));
}

#[test]
fn test_parse_partial_json_trailing_backslash() {
    let parsed = parse_partial_json(r#"{"foo":"bar\"#, false).unwrap();
    assert_eq!(parsed, json!({"foo": "bar"}));
}

fn streamed_tokens() -> Vec<&'static str> {
    vec![
        "",
        "{",
        "",
        " \"",
        "setup",
        "\":",
        " \"",
        "Why",
        " did",
        " the",
        " bears",
        " start",
        " a",
        " band",
        " called",
        " Bears",
        " Bears",
        " Bears",
        " ?",
        "\"",
        ",",
        " \"",
        "punchline",
        "\":",
        " \"",
        "Because",
        " they",
        " wanted",
        " to",
        " play",
        " bear",
        " -y",
        " good",
        " music",
        " !",
        "\"",
        ",",
        " \"",
        "audience",
        "\":",
        " [",
        "\"",
        "Haha",
        "\"",
        ",",
        " \"",
        "So",
        " funny",
        "\"]",
        "",
        "}",
    ]
}

fn expected_streamed_json() -> Vec<Value> {
    vec![
        json!({}),
        json!({"setup": ""}),
        json!({"setup": "Why"}),
        json!({"setup": "Why did"}),
        json!({"setup": "Why did the"}),
        json!({"setup": "Why did the bears"}),
        json!({"setup": "Why did the bears start"}),
        json!({"setup": "Why did the bears start a"}),
        json!({"setup": "Why did the bears start a band"}),
        json!({"setup": "Why did the bears start a band called"}),
        json!({"setup": "Why did the bears start a band called Bears"}),
        json!({"setup": "Why did the bears start a band called Bears Bears"}),
        json!({"setup": "Why did the bears start a band called Bears Bears Bears"}),
        json!({"setup": "Why did the bears start a band called Bears Bears Bears ?"}),
        json!({"setup": "Why did the bears start a band called Bears Bears Bears ?", "punchline": ""}),
        json!({"setup": "Why did the bears start a band called Bears Bears Bears ?", "punchline": "Because"}),
        json!({"setup": "Why did the bears start a band called Bears Bears Bears ?", "punchline": "Because they"}),
        json!({"setup": "Why did the bears start a band called Bears Bears Bears ?", "punchline": "Because they wanted"}),
        json!({"setup": "Why did the bears start a band called Bears Bears Bears ?", "punchline": "Because they wanted to"}),
        json!({"setup": "Why did the bears start a band called Bears Bears Bears ?", "punchline": "Because they wanted to play"}),
        json!({"setup": "Why did the bears start a band called Bears Bears Bears ?", "punchline": "Because they wanted to play bear"}),
        json!({"setup": "Why did the bears start a band called Bears Bears Bears ?", "punchline": "Because they wanted to play bear -y"}),
        json!({"setup": "Why did the bears start a band called Bears Bears Bears ?", "punchline": "Because they wanted to play bear -y good"}),
        json!({"setup": "Why did the bears start a band called Bears Bears Bears ?", "punchline": "Because they wanted to play bear -y good music"}),
        json!({"setup": "Why did the bears start a band called Bears Bears Bears ?", "punchline": "Because they wanted to play bear -y good music !"}),
        json!({"setup": "Why did the bears start a band called Bears Bears Bears ?", "punchline": "Because they wanted to play bear -y good music !", "audience": []}),
        json!({"setup": "Why did the bears start a band called Bears Bears Bears ?", "punchline": "Because they wanted to play bear -y good music !", "audience": [""]}),
        json!({"setup": "Why did the bears start a band called Bears Bears Bears ?", "punchline": "Because they wanted to play bear -y good music !", "audience": ["Haha"]}),
        json!({"setup": "Why did the bears start a band called Bears Bears Bears ?", "punchline": "Because they wanted to play bear -y good music !", "audience": ["Haha", ""]}),
        json!({"setup": "Why did the bears start a band called Bears Bears Bears ?", "punchline": "Because they wanted to play bear -y good music !", "audience": ["Haha", "So"]}),
        json!({"setup": "Why did the bears start a band called Bears Bears Bears ?", "punchline": "Because they wanted to play bear -y good music !", "audience": ["Haha", "So funny"]}),
    ]
}

fn expected_streamed_json_diff() -> Vec<Value> {
    vec![
        json!([{"op": "replace", "path": "", "value": {}}]),
        json!([{"op": "add", "path": "/setup", "value": ""}]),
        json!([{"op": "replace", "path": "/setup", "value": "Why"}]),
        json!([{"op": "replace", "path": "/setup", "value": "Why did"}]),
        json!([{"op": "replace", "path": "/setup", "value": "Why did the"}]),
        json!([{"op": "replace", "path": "/setup", "value": "Why did the bears"}]),
        json!([{"op": "replace", "path": "/setup", "value": "Why did the bears start"}]),
        json!([{"op": "replace", "path": "/setup", "value": "Why did the bears start a"}]),
        json!([{"op": "replace", "path": "/setup", "value": "Why did the bears start a band"}]),
        json!([{"op": "replace", "path": "/setup", "value": "Why did the bears start a band called"}]),
        json!([{"op": "replace", "path": "/setup", "value": "Why did the bears start a band called Bears"}]),
        json!([{"op": "replace", "path": "/setup", "value": "Why did the bears start a band called Bears Bears"}]),
        json!([{"op": "replace", "path": "/setup", "value": "Why did the bears start a band called Bears Bears Bears"}]),
        json!([{"op": "replace", "path": "/setup", "value": "Why did the bears start a band called Bears Bears Bears ?"}]),
        json!([{"op": "add", "path": "/punchline", "value": ""}]),
        json!([{"op": "replace", "path": "/punchline", "value": "Because"}]),
        json!([{"op": "replace", "path": "/punchline", "value": "Because they"}]),
        json!([{"op": "replace", "path": "/punchline", "value": "Because they wanted"}]),
        json!([{"op": "replace", "path": "/punchline", "value": "Because they wanted to"}]),
        json!([{"op": "replace", "path": "/punchline", "value": "Because they wanted to play"}]),
        json!([{"op": "replace", "path": "/punchline", "value": "Because they wanted to play bear"}]),
        json!([{"op": "replace", "path": "/punchline", "value": "Because they wanted to play bear -y"}]),
        json!([{"op": "replace", "path": "/punchline", "value": "Because they wanted to play bear -y good"}]),
        json!([{"op": "replace", "path": "/punchline", "value": "Because they wanted to play bear -y good music"}]),
        json!([{"op": "replace", "path": "/punchline", "value": "Because they wanted to play bear -y good music !"}]),
        json!([{"op": "add", "path": "/audience", "value": []}]),
        json!([{"op": "add", "path": "/audience/0", "value": ""}]),
        json!([{"op": "replace", "path": "/audience/0", "value": "Haha"}]),
        json!([{"op": "add", "path": "/audience/1", "value": ""}]),
        json!([{"op": "replace", "path": "/audience/1", "value": "So"}]),
        json!([{"op": "replace", "path": "/audience/1", "value": "So funny"}]),
    ]
}

#[tokio::test]
async fn test_partial_text_json_output_parser() {
    let parser = SimpleJsonOutputParser::builder().build();
    let tokens = streamed_tokens();

    let input_stream = futures::stream::iter(tokens.into_iter().map(BaseMessage::from));

    let results: Vec<Value> = parser
        .transform(Box::pin(input_stream))
        .filter_map(|r| async { r.ok() })
        .collect()
        .await;

    let expected = expected_streamed_json();
    assert_eq!(
        results.len(),
        expected.len(),
        "Different number of results: got {}, expected {}",
        results.len(),
        expected.len()
    );
    for (i, (got, want)) in results.iter().zip(expected.iter()).enumerate() {
        assert_eq!(got, want, "Mismatch at index {i}");
    }
}

#[tokio::test]
async fn test_partial_text_json_output_parser_diff() {
    let parser = SimpleJsonOutputParser::builder().diff(true).build();
    let tokens = streamed_tokens();

    let input_stream = futures::stream::iter(tokens.into_iter().map(BaseMessage::from));

    let results: Vec<Value> = parser
        .transform(Box::pin(input_stream))
        .filter_map(|r| async { r.ok() })
        .collect()
        .await;

    let expected = expected_streamed_json_diff();
    assert_eq!(
        results.len(),
        expected.len(),
        "Different number of diff results: got {}, expected {}",
        results.len(),
        expected.len()
    );
    for (i, (got, want)) in results.iter().zip(expected.iter()).enumerate() {
        assert_eq!(got, want, "Diff mismatch at index {i}");
    }
}

#[tokio::test]
async fn test_partial_text_json_output_parser_async() {
    let parser = SimpleJsonOutputParser::builder().build();
    let tokens = streamed_tokens();

    let input_stream = futures::stream::iter(tokens.into_iter().map(BaseMessage::from));

    let results: Vec<Value> = parser
        .atransform(Box::pin(input_stream))
        .filter_map(|r| async { r.ok() })
        .collect()
        .await;

    let expected = expected_streamed_json();
    assert_eq!(results.len(), expected.len());
    for (i, (got, want)) in results.iter().zip(expected.iter()).enumerate() {
        assert_eq!(got, want, "Async mismatch at index {i}");
    }
}

#[tokio::test]
async fn test_partial_text_json_output_parser_diff_async() {
    let parser = SimpleJsonOutputParser::builder().diff(true).build();
    let tokens = streamed_tokens();

    let input_stream = futures::stream::iter(tokens.into_iter().map(BaseMessage::from));

    let results: Vec<Value> = parser
        .atransform(Box::pin(input_stream))
        .filter_map(|r| async { r.ok() })
        .collect()
        .await;

    let expected = expected_streamed_json_diff();
    assert_eq!(results.len(), expected.len());
    for (i, (got, want)) in results.iter().zip(expected.iter()).enumerate() {
        assert_eq!(got, want, "Async diff mismatch at index {i}");
    }
}

#[test]
fn test_raises_error() {
    let parser = SimpleJsonOutputParser::builder().build();
    let result = parser.parse("hi");
    assert!(result.is_err(), "Parsing 'hi' should produce an error");
}

#[tokio::test]
async fn test_partial_text_json_output_parser_with_json_code_block() {
    let tokens: Vec<&str> = vec![
        " France",
        ":",
        "\n\n```",
        "json",
        "\n{",
        "\n ",
        " \"",
        "country",
        "_",
        "name",
        "\":",
        " \"",
        "France",
        "\",",
        " \n ",
        " \"",
        "population",
        "_",
        "size",
        "\":",
        " 67",
        "39",
        "15",
        "82",
        "\n}",
        "\n```",
        "\n\nI",
        " looked",
        " up",
    ];

    let parser = SimpleJsonOutputParser::builder().build();

    let input_stream = futures::stream::iter(tokens.into_iter().map(BaseMessage::from));

    let results: Vec<Value> = parser
        .transform(Box::pin(input_stream))
        .filter_map(|r| async { r.ok() })
        .collect()
        .await;

    let expected = [
        json!({}),
        json!({"country_name": ""}),
        json!({"country_name": "France"}),
        json!({"country_name": "France", "population_size": 67}),
        json!({"country_name": "France", "population_size": 6739}),
        json!({"country_name": "France", "population_size": 673915}),
        json!({"country_name": "France", "population_size": 67391582}),
    ];

    assert_eq!(
        results.len(),
        expected.len(),
        "Code block streaming: got {} results, expected {}",
        results.len(),
        expected.len()
    );
    for (i, (got, want)) in results.iter().zip(expected.iter()).enumerate() {
        assert_eq!(got, want, "Code block mismatch at index {i}");
    }
}

#[test]
fn test_unicode_handling() {
    let schema = json!({
        "title": "Sample",
        "type": "object",
        "properties": {
            "title": {
                "type": "string",
                "description": "科学文章的标题"
            }
        }
    });
    let parser = SimpleJsonOutputParser::with_schema(schema);
    let format_instructions = parser.get_format_instructions().unwrap();
    assert!(
        format_instructions.contains("科学文章的标题"),
        "Unicode characters should not be escaped in format instructions"
    );
}
