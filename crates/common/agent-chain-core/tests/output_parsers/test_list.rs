//! Tests for list output parsers.
//!
//! Ported from langchain/libs/core/tests/unit_tests/output_parsers/test_list.py

use agent_chain_core::drop_last_n;
use agent_chain_core::messages::BaseMessage;
use agent_chain_core::output_parsers::{
    BaseOutputParser, BaseTransformOutputParser, CommaSeparatedListOutputParser, ListOutputParser,
    MarkdownListOutputParser, NumberedListOutputParser,
};
use futures::StreamExt;

// --- droplastn() utility function tests ---

#[test]
fn test_drop_last_one() {
    let result: Vec<_> = drop_last_n(vec![1, 2, 3, 4, 5].into_iter(), 1).collect();
    assert_eq!(result, vec![1, 2, 3, 4]);
}

#[test]
fn test_drop_last_two() {
    let result: Vec<_> = drop_last_n(vec![1, 2, 3, 4, 5].into_iter(), 2).collect();
    assert_eq!(result, vec![1, 2, 3]);
}

#[test]
fn test_drop_last_zero() {
    let result: Vec<_> = drop_last_n(vec![1, 2, 3].into_iter(), 0).collect();
    assert_eq!(result, vec![1, 2, 3]);
}

#[test]
fn test_drop_all() {
    let result: Vec<_> = drop_last_n(vec![1, 2, 3].into_iter(), 3).collect();
    assert_eq!(result, Vec::<i32>::new());
}

#[test]
fn test_drop_more_than_length() {
    let result: Vec<_> = drop_last_n(vec![1, 2].into_iter(), 5).collect();
    assert_eq!(result, Vec::<i32>::new());
}

#[test]
fn test_empty_iterator() {
    let result: Vec<_> = drop_last_n(Vec::<i32>::new().into_iter(), 1).collect();
    assert_eq!(result, Vec::<i32>::new());
}

#[test]
fn test_single_element_drop_one() {
    let result: Vec<_> = drop_last_n(vec![42].into_iter(), 1).collect();
    assert_eq!(result, Vec::<i32>::new());
}

#[test]
fn test_string_elements() {
    let result: Vec<_> = drop_last_n(vec!["a", "b", "c"].into_iter(), 1).collect();
    assert_eq!(result, vec!["a", "b"]);
}

// --- CommaSeparatedListOutputParser tests ---

#[test]
fn test_comma_single_item() {
    let parser = CommaSeparatedListOutputParser::new();
    assert_eq!(parser.parse("foo").unwrap(), vec!["foo"]);
}

#[test]
fn test_comma_multiple_items_with_spaces() {
    let parser = CommaSeparatedListOutputParser::new();
    assert_eq!(
        parser.parse("foo, bar, baz").unwrap(),
        vec!["foo", "bar", "baz"]
    );
}

#[test]
fn test_comma_multiple_items_no_spaces() {
    let parser = CommaSeparatedListOutputParser::new();
    assert_eq!(
        parser.parse("foo,bar,baz").unwrap(),
        vec!["foo", "bar", "baz"]
    );
}

#[test]
fn test_comma_quoted_items_with_commas() {
    let parser = CommaSeparatedListOutputParser::new();
    assert_eq!(
        parser.parse("\"foo, foo2\",bar,baz").unwrap(),
        vec!["foo, foo2", "bar", "baz"]
    );
}

#[test]
fn test_comma_empty_string() {
    let parser = CommaSeparatedListOutputParser::new();
    let result = parser.parse("").unwrap();
    assert!(result.is_empty());
}

#[test]
fn test_comma_many_items() {
    let parser = CommaSeparatedListOutputParser::new();
    let items: Vec<String> = (0..20).map(|i| format!("item{i}")).collect();
    let text = items.join(", ");
    assert_eq!(parser.parse(&text).unwrap(), items);
}

#[test]
fn test_comma_type_property() {
    let parser = CommaSeparatedListOutputParser::new();
    assert_eq!(parser.parser_type(), "comma-separated-list");
}

#[test]
fn test_comma_is_lc_serializable() {
    assert!(CommaSeparatedListOutputParser::is_lc_serializable());
}

#[test]
fn test_comma_get_lc_namespace() {
    assert_eq!(
        CommaSeparatedListOutputParser::get_lc_namespace(),
        vec!["langchain", "output_parsers", "list"]
    );
}

#[test]
fn test_comma_get_format_instructions() {
    let parser = CommaSeparatedListOutputParser::new();
    let instructions = parser.get_format_instructions().unwrap();
    assert!(instructions.to_lowercase().contains("comma"));
    assert!(instructions.contains("foo"));
}

#[tokio::test]
async fn test_comma_transform_single_chunk() {
    let parser = CommaSeparatedListOutputParser::new();
    let text = "foo, bar, baz";

    let input_stream = futures::stream::iter(vec![BaseMessage::from(text)]);
    let results: Vec<Vec<String>> = parser
        .transform(Box::pin(input_stream))
        .filter_map(|r| async { r.ok() })
        .collect()
        .await;

    assert_eq!(results, vec![vec!["foo", "bar", "baz"]]);
}

// --- CommaSeparatedListOutputParser async tests ---

#[tokio::test]
async fn test_comma_aparse() {
    let parser = CommaSeparatedListOutputParser::new();
    assert_eq!(parser.aparse("foo, bar").await.unwrap(), vec!["foo", "bar"]);
}

// --- NumberedListOutputParser tests ---

#[test]
fn test_numbered_basic_list() {
    let parser = NumberedListOutputParser::new();
    let text = "1. foo\n2. bar\n3. baz";
    assert_eq!(parser.parse(text).unwrap(), vec!["foo", "bar", "baz"]);
}

#[test]
fn test_numbered_extra_spacing() {
    let parser = NumberedListOutputParser::new();
    let text = "1. foo\n\n2. bar\n\n3. baz";
    assert_eq!(parser.parse(text).unwrap(), vec!["foo", "bar", "baz"]);
}

#[test]
fn test_numbered_prefix_text() {
    let parser = NumberedListOutputParser::new();
    let text = "Here are the items:\n\n1. apple\n2. banana\n3. cherry";
    assert_eq!(
        parser.parse(text).unwrap(),
        vec!["apple", "banana", "cherry"]
    );
}

#[test]
fn test_numbered_empty_text_returns_empty_list() {
    let parser = NumberedListOutputParser::new();
    assert!(parser.parse("No items here.").unwrap().is_empty());
}

#[test]
fn test_numbered_indented_numbers() {
    let parser = NumberedListOutputParser::new();
    let text = "Items:\n\n1. apple\n\n    2. banana\n\n3. cherry";
    assert_eq!(
        parser.parse(text).unwrap(),
        vec!["apple", "banana", "cherry"]
    );
}

#[test]
fn test_numbered_large_numbers() {
    let parser = NumberedListOutputParser::new();
    let text = "100. first\n200. second\n300. third";
    assert_eq!(
        parser.parse(text).unwrap(),
        vec!["first", "second", "third"]
    );
}

#[test]
fn test_numbered_type_property() {
    let parser = NumberedListOutputParser::new();
    assert_eq!(parser.parser_type(), "numbered-list");
}

#[test]
fn test_numbered_get_format_instructions() {
    let parser = NumberedListOutputParser::new();
    let instructions = parser.get_format_instructions().unwrap();
    assert!(instructions.to_lowercase().contains("numbered"));
    assert!(instructions.contains("1."));
}

#[test]
fn test_numbered_parse_iter() {
    let parser = NumberedListOutputParser::new();
    let text = "1. foo\n2. bar";
    let matches = parser.parse_iter(text);
    assert_eq!(matches.len(), 2);
    assert_eq!(matches[0].group, "foo");
    assert_eq!(matches[1].group, "bar");
}

#[test]
fn test_numbered_custom_pattern() {
    let parser = NumberedListOutputParser::with_pattern(r"\d+\)\s([^\n]+)");
    let text = "1) foo\n2) bar";
    assert_eq!(parser.parse(text).unwrap(), vec!["foo", "bar"]);
}

#[tokio::test]
async fn test_numbered_transform_single_chunk() {
    let parser = NumberedListOutputParser::new();
    let text = "1. foo\n2. bar\n3. baz";

    let input_stream = futures::stream::iter(vec![BaseMessage::from(text)]);
    let results: Vec<Vec<String>> = parser
        .transform(Box::pin(input_stream))
        .filter_map(|r| async { r.ok() })
        .collect()
        .await;

    assert_eq!(results, vec![vec!["foo", "bar", "baz"]]);
}

// --- NumberedListOutputParser async tests ---

#[tokio::test]
async fn test_numbered_aparse() {
    let parser = NumberedListOutputParser::new();
    let text = "1. foo\n2. bar\n3. baz";
    assert_eq!(
        parser.aparse(text).await.unwrap(),
        vec!["foo", "bar", "baz"]
    );
}

// --- MarkdownListOutputParser tests ---

#[test]
fn test_markdown_basic_dash_list() {
    let parser = MarkdownListOutputParser::new();
    let text = "- foo\n- bar\n- baz";
    assert_eq!(parser.parse(text).unwrap(), vec!["foo", "bar", "baz"]);
}

#[test]
fn test_markdown_asterisk_list() {
    let parser = MarkdownListOutputParser::new();
    let text = "* foo\n* bar\n* baz";
    assert_eq!(parser.parse(text).unwrap(), vec!["foo", "bar", "baz"]);
}

#[test]
fn test_markdown_list_with_prefix_text() {
    let parser = MarkdownListOutputParser::new();
    let text = "Items:\n- apple\n- banana\n- cherry";
    assert_eq!(
        parser.parse(text).unwrap(),
        vec!["apple", "banana", "cherry"]
    );
}

#[test]
fn test_markdown_empty_text_returns_empty_list() {
    let parser = MarkdownListOutputParser::new();
    assert!(parser.parse("No items here.").unwrap().is_empty());
}

#[test]
fn test_markdown_indented_items() {
    let parser = MarkdownListOutputParser::new();
    let text = "Items:\n- apple\n     - banana\n- cherry";
    assert_eq!(
        parser.parse(text).unwrap(),
        vec!["apple", "banana", "cherry"]
    );
}

#[test]
fn test_markdown_text_with_dashes_in_prose() {
    let parser = MarkdownListOutputParser::new();
    let text = "This is a sentence with - not a list item.\n- actual item";
    let result = parser.parse(text).unwrap();
    assert!(result.contains(&"actual item".to_string()));
}

#[test]
fn test_markdown_type_property() {
    let parser = MarkdownListOutputParser::new();
    assert_eq!(parser.parser_type(), "markdown-list");
}

#[test]
fn test_markdown_get_format_instructions() {
    let parser = MarkdownListOutputParser::new();
    let instructions = parser.get_format_instructions().unwrap();
    assert!(instructions.to_lowercase().contains("markdown"));
    assert!(instructions.contains("- foo"));
}

#[test]
fn test_markdown_parse_iter() {
    let parser = MarkdownListOutputParser::new();
    let text = "- foo\n- bar";
    let matches = parser.parse_iter(text);
    assert_eq!(matches.len(), 2);
    assert_eq!(matches[0].group, "foo");
    assert_eq!(matches[1].group, "bar");
}

#[tokio::test]
async fn test_markdown_transform_single_chunk() {
    let parser = MarkdownListOutputParser::new();
    let text = "- foo\n- bar\n- baz";

    let input_stream = futures::stream::iter(vec![BaseMessage::from(text)]);
    let results: Vec<Vec<String>> = parser
        .transform(Box::pin(input_stream))
        .filter_map(|r| async { r.ok() })
        .collect()
        .await;

    assert_eq!(results, vec![vec!["foo", "bar", "baz"]]);
}

// --- MarkdownListOutputParser async tests ---

#[tokio::test]
async fn test_markdown_aparse() {
    let parser = MarkdownListOutputParser::new();
    let text = "- foo\n- bar\n- baz";
    assert_eq!(
        parser.aparse(text).await.unwrap(),
        vec!["foo", "bar", "baz"]
    );
}

// --- ListOutputParser base trait tests ---

#[test]
fn test_list_type_property_via_numbered() {
    let parser = NumberedListOutputParser::new();
    assert_eq!(parser.parser_type(), "numbered-list");
}

#[test]
fn test_default_parse_iter_returns_empty() {
    let parser = CommaSeparatedListOutputParser::new();
    let matches = parser.parse_iter("foo, bar");
    assert!(matches.is_empty());
}
