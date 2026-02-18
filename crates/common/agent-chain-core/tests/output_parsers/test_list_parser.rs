//! Tests for list output parsers — streaming transform behavior.
//!
//! Ported from langchain/libs/core/tests/unit_tests/output_parsers/test_list_parser.py
//!
//! These tests exercise the `_transform` streaming logic of the list parsers,
//! verifying that character-by-character, line-by-line, word-by-word, and
//! single-chunk streaming all produce the correct results.

use agent_chain_core::messages::BaseMessage;
use agent_chain_core::output_parsers::{
    BaseOutputParser, BaseTransformOutputParser, CommaSeparatedListOutputParser,
    MarkdownListOutputParser, NumberedListOutputParser,
};
use futures::StreamExt;

/// Collect all yielded chunks from a transform stream into a flat list.
/// Equivalent to Python's `add(parser.transform(...))`.
async fn transform_add<P: BaseTransformOutputParser<Output = Vec<String>>>(
    parser: &P,
    chunks: Vec<BaseMessage>,
) -> Vec<String> {
    let input_stream = futures::stream::iter(chunks);
    parser
        .transform(Box::pin(input_stream))
        .filter_map(|r| async { r.ok() })
        .collect::<Vec<Vec<String>>>()
        .await
        .into_iter()
        .flatten()
        .collect()
}

/// Collect all yielded chunks from a transform stream, keeping each yield
/// as a separate `Vec<String>`.
/// Equivalent to Python's `list(parser.transform(...))`.
async fn transform_list<P: BaseTransformOutputParser<Output = Vec<String>>>(
    parser: &P,
    chunks: Vec<BaseMessage>,
) -> Vec<Vec<String>> {
    let input_stream = futures::stream::iter(chunks);
    parser
        .transform(Box::pin(input_stream))
        .filter_map(|r| async { r.ok() })
        .collect()
        .await
}

/// Convert a string into character-by-character BaseMessage chunks.
fn char_chunks(text: &str) -> Vec<BaseMessage> {
    text.chars()
        .map(|c| BaseMessage::from(c.to_string()))
        .collect()
}

/// Convert a string into line-by-line BaseMessage chunks (keeping line endings).
fn line_chunks(text: &str) -> Vec<BaseMessage> {
    let mut chunks = Vec::new();
    let mut start = 0;
    for (idx, ch) in text.char_indices() {
        if ch == '\n' {
            chunks.push(BaseMessage::from(&text[start..=idx]));
            start = idx + 1;
        }
    }
    if start < text.len() {
        chunks.push(BaseMessage::from(&text[start..]));
    }
    chunks
}

/// Convert a string into word-by-word BaseMessage chunks.
/// Words after the first are prefixed with a space (matching the Python
/// `" " + t if i > 0 else t for i, t in enumerate(text.split(" "))` pattern).
fn word_chunks(text: &str) -> Vec<BaseMessage> {
    text.split(' ')
        .enumerate()
        .map(|(i, word)| {
            if i > 0 {
                BaseMessage::from(format!(" {word}"))
            } else {
                BaseMessage::from(word)
            }
        })
        .collect()
}

/// Wrap a full string as a single BaseMessage chunk.
fn single_chunk(text: &str) -> Vec<BaseMessage> {
    vec![BaseMessage::from(text)]
}


#[tokio::test]
async fn test_single_item_parse() {
    let parser = CommaSeparatedListOutputParser::new();
    assert_eq!(parser.parse("foo").unwrap(), vec!["foo"]);
}

#[tokio::test]
async fn test_single_item_transform_add_chars() {
    let parser = CommaSeparatedListOutputParser::new();
    assert_eq!(
        transform_add(&parser, char_chunks("foo")).await,
        vec!["foo"]
    );
}

#[tokio::test]
async fn test_single_item_transform_list_chars() {
    let parser = CommaSeparatedListOutputParser::new();
    let result = transform_list(&parser, char_chunks("foo")).await;
    assert_eq!(result, vec![vec!["foo"]]);
}

#[tokio::test]
async fn test_single_item_transform_list_lines() {
    let parser = CommaSeparatedListOutputParser::new();
    let result = transform_list(&parser, line_chunks("foo")).await;
    assert_eq!(result, vec![vec!["foo"]]);
}

#[tokio::test]
async fn test_single_item_transform_list_words() {
    let parser = CommaSeparatedListOutputParser::new();
    let result = transform_list(&parser, word_chunks("foo")).await;
    assert_eq!(result, vec![vec!["foo"]]);
}

#[tokio::test]
async fn test_single_item_transform_list_single_chunk() {
    let parser = CommaSeparatedListOutputParser::new();
    let result = transform_list(&parser, single_chunk("foo")).await;
    assert_eq!(result, vec![vec!["foo"]]);
}

#[tokio::test]
async fn test_multiple_items_with_spaces_parse() {
    let parser = CommaSeparatedListOutputParser::new();
    assert_eq!(
        parser.parse("foo, bar, baz").unwrap(),
        vec!["foo", "bar", "baz"]
    );
}

#[tokio::test]
async fn test_multiple_items_with_spaces_transform_add_chars() {
    let parser = CommaSeparatedListOutputParser::new();
    assert_eq!(
        transform_add(&parser, char_chunks("foo, bar, baz")).await,
        vec!["foo", "bar", "baz"]
    );
}

#[tokio::test]
async fn test_multiple_items_with_spaces_transform_list_chars() {
    let parser = CommaSeparatedListOutputParser::new();
    let result = transform_list(&parser, char_chunks("foo, bar, baz")).await;
    assert_eq!(result, vec![vec!["foo"], vec!["bar"], vec!["baz"]]);
}

#[tokio::test]
async fn test_multiple_items_with_spaces_transform_list_lines() {
    let parser = CommaSeparatedListOutputParser::new();
    let result = transform_list(&parser, line_chunks("foo, bar, baz")).await;
    assert_eq!(result, vec![vec!["foo"], vec!["bar"], vec!["baz"]]);
}

#[tokio::test]
async fn test_multiple_items_with_spaces_transform_list_words() {
    let parser = CommaSeparatedListOutputParser::new();
    let result = transform_list(&parser, word_chunks("foo, bar, baz")).await;
    assert_eq!(result, vec![vec!["foo"], vec!["bar"], vec!["baz"]]);
}

#[tokio::test]
async fn test_multiple_items_with_spaces_transform_list_single_chunk() {
    let parser = CommaSeparatedListOutputParser::new();
    let result = transform_list(&parser, single_chunk("foo, bar, baz")).await;
    assert_eq!(result, vec![vec!["foo"], vec!["bar"], vec!["baz"]]);
}

#[tokio::test]
async fn test_multiple_items_no_spaces_parse() {
    let parser = CommaSeparatedListOutputParser::new();
    assert_eq!(
        parser.parse("foo,bar,baz").unwrap(),
        vec!["foo", "bar", "baz"]
    );
}

#[tokio::test]
async fn test_multiple_items_no_spaces_transform_add_chars() {
    let parser = CommaSeparatedListOutputParser::new();
    assert_eq!(
        transform_add(&parser, char_chunks("foo,bar,baz")).await,
        vec!["foo", "bar", "baz"]
    );
}

#[tokio::test]
async fn test_multiple_items_no_spaces_transform_list_chars() {
    let parser = CommaSeparatedListOutputParser::new();
    let result = transform_list(&parser, char_chunks("foo,bar,baz")).await;
    assert_eq!(result, vec![vec!["foo"], vec!["bar"], vec!["baz"]]);
}

#[tokio::test]
async fn test_multiple_items_no_spaces_transform_list_lines() {
    let parser = CommaSeparatedListOutputParser::new();
    let result = transform_list(&parser, line_chunks("foo,bar,baz")).await;
    assert_eq!(result, vec![vec!["foo"], vec!["bar"], vec!["baz"]]);
}

#[tokio::test]
async fn test_multiple_items_no_spaces_transform_list_words() {
    let parser = CommaSeparatedListOutputParser::new();
    let result = transform_list(&parser, word_chunks("foo,bar,baz")).await;
    assert_eq!(result, vec![vec!["foo"], vec!["bar"], vec!["baz"]]);
}

#[tokio::test]
async fn test_multiple_items_no_spaces_transform_list_single_chunk() {
    let parser = CommaSeparatedListOutputParser::new();
    let result = transform_list(&parser, single_chunk("foo,bar,baz")).await;
    assert_eq!(result, vec![vec!["foo"], vec!["bar"], vec!["baz"]]);
}

#[tokio::test]
async fn test_multiple_items_with_comma_parse() {
    let parser = CommaSeparatedListOutputParser::new();
    assert_eq!(
        parser.parse(r#""foo, foo2",bar,baz"#).unwrap(),
        vec!["foo, foo2", "bar", "baz"]
    );
}

#[tokio::test]
async fn test_multiple_items_with_comma_transform_add_chars() {
    let parser = CommaSeparatedListOutputParser::new();
    assert_eq!(
        transform_add(&parser, char_chunks(r#""foo, foo2",bar,baz"#)).await,
        vec!["foo, foo2", "bar", "baz"]
    );
}

#[tokio::test]
async fn test_multiple_items_with_comma_transform_list_chars() {
    let parser = CommaSeparatedListOutputParser::new();
    let result = transform_list(&parser, char_chunks(r#""foo, foo2",bar,baz"#)).await;
    assert_eq!(result, vec![vec!["foo, foo2"], vec!["bar"], vec!["baz"]]);
}

#[tokio::test]
async fn test_multiple_items_with_comma_transform_list_lines() {
    let parser = CommaSeparatedListOutputParser::new();
    let result = transform_list(&parser, line_chunks(r#""foo, foo2",bar,baz"#)).await;
    assert_eq!(result, vec![vec!["foo, foo2"], vec!["bar"], vec!["baz"]]);
}

#[tokio::test]
async fn test_multiple_items_with_comma_transform_list_words() {
    let parser = CommaSeparatedListOutputParser::new();
    let result = transform_list(&parser, word_chunks(r#""foo, foo2",bar,baz"#)).await;
    assert_eq!(result, vec![vec!["foo, foo2"], vec!["bar"], vec!["baz"]]);
}

#[tokio::test]
async fn test_multiple_items_with_comma_transform_list_single_chunk() {
    let parser = CommaSeparatedListOutputParser::new();
    let result = transform_list(&parser, single_chunk(r#""foo, foo2",bar,baz"#)).await;
    assert_eq!(result, vec![vec!["foo, foo2"], vec!["bar"], vec!["baz"]]);
}


#[tokio::test]
async fn test_numbered_list_text1_parse() {
    let parser = NumberedListOutputParser::new();
    let text = "Your response should be a numbered list with each item on a new line. \
                For example: \n\n1. foo\n\n2. bar\n\n3. baz";
    assert_eq!(parser.parse(text).unwrap(), vec!["foo", "bar", "baz"]);
}

#[tokio::test]
async fn test_numbered_list_text1_transform_add_chars() {
    let parser = NumberedListOutputParser::new();
    let text = "Your response should be a numbered list with each item on a new line. \
                For example: \n\n1. foo\n\n2. bar\n\n3. baz";
    assert_eq!(
        transform_add(&parser, char_chunks(text)).await,
        vec!["foo", "bar", "baz"]
    );
}

#[tokio::test]
async fn test_numbered_list_text1_transform_list_chars() {
    let parser = NumberedListOutputParser::new();
    let text = "Your response should be a numbered list with each item on a new line. \
                For example: \n\n1. foo\n\n2. bar\n\n3. baz";
    let result = transform_list(&parser, char_chunks(text)).await;
    assert_eq!(result, vec![vec!["foo"], vec!["bar"], vec!["baz"]]);
}

#[tokio::test]
async fn test_numbered_list_text1_transform_list_lines() {
    let parser = NumberedListOutputParser::new();
    let text = "Your response should be a numbered list with each item on a new line. \
                For example: \n\n1. foo\n\n2. bar\n\n3. baz";
    let result = transform_list(&parser, line_chunks(text)).await;
    assert_eq!(result, vec![vec!["foo"], vec!["bar"], vec!["baz"]]);
}

#[tokio::test]
async fn test_numbered_list_text1_transform_list_words() {
    let parser = NumberedListOutputParser::new();
    let text = "Your response should be a numbered list with each item on a new line. \
                For example: \n\n1. foo\n\n2. bar\n\n3. baz";
    let result = transform_list(&parser, word_chunks(text)).await;
    assert_eq!(result, vec![vec!["foo"], vec!["bar"], vec!["baz"]]);
}

#[tokio::test]
async fn test_numbered_list_text1_transform_list_single_chunk() {
    let parser = NumberedListOutputParser::new();
    let text = "Your response should be a numbered list with each item on a new line. \
                For example: \n\n1. foo\n\n2. bar\n\n3. baz";
    let result = transform_list(&parser, single_chunk(text)).await;
    assert_eq!(result, vec![vec!["foo"], vec!["bar"], vec!["baz"]]);
}

#[tokio::test]
async fn test_numbered_list_text2_parse() {
    let parser = NumberedListOutputParser::new();
    let text = "Items:\n\n1. apple\n\n    2. banana\n\n3. cherry";
    assert_eq!(
        parser.parse(text).unwrap(),
        vec!["apple", "banana", "cherry"]
    );
}

#[tokio::test]
async fn test_numbered_list_text2_transform_add_chars() {
    let parser = NumberedListOutputParser::new();
    let text = "Items:\n\n1. apple\n\n    2. banana\n\n3. cherry";
    assert_eq!(
        transform_add(&parser, char_chunks(text)).await,
        vec!["apple", "banana", "cherry"]
    );
}

#[tokio::test]
async fn test_numbered_list_text2_transform_list_chars() {
    let parser = NumberedListOutputParser::new();
    let text = "Items:\n\n1. apple\n\n    2. banana\n\n3. cherry";
    let result = transform_list(&parser, char_chunks(text)).await;
    assert_eq!(result, vec![vec!["apple"], vec!["banana"], vec!["cherry"]]);
}

#[tokio::test]
async fn test_numbered_list_text2_transform_list_lines() {
    let parser = NumberedListOutputParser::new();
    let text = "Items:\n\n1. apple\n\n    2. banana\n\n3. cherry";
    let result = transform_list(&parser, line_chunks(text)).await;
    assert_eq!(result, vec![vec!["apple"], vec!["banana"], vec!["cherry"]]);
}

#[tokio::test]
async fn test_numbered_list_text2_transform_list_words() {
    let parser = NumberedListOutputParser::new();
    let text = "Items:\n\n1. apple\n\n    2. banana\n\n3. cherry";
    let result = transform_list(&parser, word_chunks(text)).await;
    assert_eq!(result, vec![vec!["apple"], vec!["banana"], vec!["cherry"]]);
}

#[tokio::test]
async fn test_numbered_list_text2_transform_list_single_chunk() {
    let parser = NumberedListOutputParser::new();
    let text = "Items:\n\n1. apple\n\n    2. banana\n\n3. cherry";
    let result = transform_list(&parser, single_chunk(text)).await;
    assert_eq!(result, vec![vec!["apple"], vec!["banana"], vec!["cherry"]]);
}

#[tokio::test]
async fn test_numbered_list_text3_no_items_parse() {
    let parser = NumberedListOutputParser::new();
    assert!(parser.parse("No items in the list.").unwrap().is_empty());
}

#[tokio::test]
async fn test_numbered_list_text3_no_items_transform_add_chars() {
    let parser = NumberedListOutputParser::new();
    let result = transform_add(&parser, char_chunks("No items in the list.")).await;
    assert!(result.is_empty());
}

#[tokio::test]
async fn test_numbered_list_text3_no_items_transform_list_chars() {
    let parser = NumberedListOutputParser::new();
    let result = transform_list(&parser, char_chunks("No items in the list.")).await;
    let empty: Vec<Vec<String>> = vec![];
    assert_eq!(result, empty);
}

#[tokio::test]
async fn test_numbered_list_text3_no_items_transform_list_lines() {
    let parser = NumberedListOutputParser::new();
    let result = transform_list(&parser, line_chunks("No items in the list.")).await;
    let empty: Vec<Vec<String>> = vec![];
    assert_eq!(result, empty);
}

#[tokio::test]
async fn test_numbered_list_text3_no_items_transform_list_words() {
    let parser = NumberedListOutputParser::new();
    let result = transform_list(&parser, word_chunks("No items in the list.")).await;
    let empty: Vec<Vec<String>> = vec![];
    assert_eq!(result, empty);
}

#[tokio::test]
async fn test_numbered_list_text3_no_items_transform_list_single_chunk() {
    let parser = NumberedListOutputParser::new();
    let result = transform_list(&parser, single_chunk("No items in the list.")).await;
    let empty: Vec<Vec<String>> = vec![];
    assert_eq!(result, empty);
}


#[tokio::test]
async fn test_markdown_list_text1_parse() {
    let parser = MarkdownListOutputParser::new();
    let text = "Your response should be a numbered - not a list item - \
                list with each item on a new line.\
                For example: \n- foo\n- bar\n- baz";
    assert_eq!(parser.parse(text).unwrap(), vec!["foo", "bar", "baz"]);
}

#[tokio::test]
async fn test_markdown_list_text1_transform_add_chars() {
    let parser = MarkdownListOutputParser::new();
    let text = "Your response should be a numbered - not a list item - \
                list with each item on a new line.\
                For example: \n- foo\n- bar\n- baz";
    assert_eq!(
        transform_add(&parser, char_chunks(text)).await,
        vec!["foo", "bar", "baz"]
    );
}

#[tokio::test]
async fn test_markdown_list_text1_transform_list_chars() {
    let parser = MarkdownListOutputParser::new();
    let text = "Your response should be a numbered - not a list item - \
                list with each item on a new line.\
                For example: \n- foo\n- bar\n- baz";
    let result = transform_list(&parser, char_chunks(text)).await;
    assert_eq!(result, vec![vec!["foo"], vec!["bar"], vec!["baz"]]);
}

#[tokio::test]
async fn test_markdown_list_text1_transform_list_lines() {
    let parser = MarkdownListOutputParser::new();
    let text = "Your response should be a numbered - not a list item - \
                list with each item on a new line.\
                For example: \n- foo\n- bar\n- baz";
    let result = transform_list(&parser, line_chunks(text)).await;
    assert_eq!(result, vec![vec!["foo"], vec!["bar"], vec!["baz"]]);
}

#[tokio::test]
async fn test_markdown_list_text1_transform_list_words() {
    let parser = MarkdownListOutputParser::new();
    let text = "Your response should be a numbered - not a list item - \
                list with each item on a new line.\
                For example: \n- foo\n- bar\n- baz";
    let result = transform_list(&parser, word_chunks(text)).await;
    assert_eq!(result, vec![vec!["foo"], vec!["bar"], vec!["baz"]]);
}

#[tokio::test]
async fn test_markdown_list_text1_transform_list_single_chunk() {
    let parser = MarkdownListOutputParser::new();
    let text = "Your response should be a numbered - not a list item - \
                list with each item on a new line.\
                For example: \n- foo\n- bar\n- baz";
    let result = transform_list(&parser, single_chunk(text)).await;
    assert_eq!(result, vec![vec!["foo"], vec!["bar"], vec!["baz"]]);
}

#[tokio::test]
async fn test_markdown_list_text2_parse() {
    let parser = MarkdownListOutputParser::new();
    let text = "Items:\n- apple\n     - banana\n- cherry";
    assert_eq!(
        parser.parse(text).unwrap(),
        vec!["apple", "banana", "cherry"]
    );
}

#[tokio::test]
async fn test_markdown_list_text2_transform_add_chars() {
    let parser = MarkdownListOutputParser::new();
    let text = "Items:\n- apple\n     - banana\n- cherry";
    assert_eq!(
        transform_add(&parser, char_chunks(text)).await,
        vec!["apple", "banana", "cherry"]
    );
}

#[tokio::test]
async fn test_markdown_list_text2_transform_list_chars() {
    let parser = MarkdownListOutputParser::new();
    let text = "Items:\n- apple\n     - banana\n- cherry";
    let result = transform_list(&parser, char_chunks(text)).await;
    assert_eq!(result, vec![vec!["apple"], vec!["banana"], vec!["cherry"]]);
}

#[tokio::test]
async fn test_markdown_list_text2_transform_list_lines() {
    let parser = MarkdownListOutputParser::new();
    let text = "Items:\n- apple\n     - banana\n- cherry";
    let result = transform_list(&parser, line_chunks(text)).await;
    assert_eq!(result, vec![vec!["apple"], vec!["banana"], vec!["cherry"]]);
}

#[tokio::test]
async fn test_markdown_list_text2_transform_list_words() {
    let parser = MarkdownListOutputParser::new();
    let text = "Items:\n- apple\n     - banana\n- cherry";
    let result = transform_list(&parser, word_chunks(text)).await;
    assert_eq!(result, vec![vec!["apple"], vec!["banana"], vec!["cherry"]]);
}

#[tokio::test]
async fn test_markdown_list_text2_transform_list_single_chunk() {
    let parser = MarkdownListOutputParser::new();
    let text = "Items:\n- apple\n     - banana\n- cherry";
    let result = transform_list(&parser, single_chunk(text)).await;
    assert_eq!(result, vec![vec!["apple"], vec!["banana"], vec!["cherry"]]);
}

#[tokio::test]
async fn test_markdown_list_text3_no_items_parse() {
    let parser = MarkdownListOutputParser::new();
    assert!(parser.parse("No items in the list.").unwrap().is_empty());
}

#[tokio::test]
async fn test_markdown_list_text3_no_items_transform_add_chars() {
    let parser = MarkdownListOutputParser::new();
    let result = transform_add(&parser, char_chunks("No items in the list.")).await;
    assert!(result.is_empty());
}

#[tokio::test]
async fn test_markdown_list_text3_no_items_transform_list_chars() {
    let parser = MarkdownListOutputParser::new();
    let result = transform_list(&parser, char_chunks("No items in the list.")).await;
    let empty: Vec<Vec<String>> = vec![];
    assert_eq!(result, empty);
}

#[tokio::test]
async fn test_markdown_list_text3_no_items_transform_list_lines() {
    let parser = MarkdownListOutputParser::new();
    let result = transform_list(&parser, line_chunks("No items in the list.")).await;
    let empty: Vec<Vec<String>> = vec![];
    assert_eq!(result, empty);
}

#[tokio::test]
async fn test_markdown_list_text3_no_items_transform_list_words() {
    let parser = MarkdownListOutputParser::new();
    let result = transform_list(&parser, word_chunks("No items in the list.")).await;
    let empty: Vec<Vec<String>> = vec![];
    assert_eq!(result, empty);
}

#[tokio::test]
async fn test_markdown_list_text3_no_items_transform_list_single_chunk() {
    let parser = MarkdownListOutputParser::new();
    let result = transform_list(&parser, single_chunk("No items in the list.")).await;
    let empty: Vec<Vec<String>> = vec![];
    assert_eq!(result, empty);
}


#[tokio::test]
async fn test_comma_aparse_single_item() {
    let parser = CommaSeparatedListOutputParser::new();
    assert_eq!(parser.aparse("foo").await.unwrap(), vec!["foo"]);
}

#[tokio::test]
async fn test_comma_aparse_multiple_items() {
    let parser = CommaSeparatedListOutputParser::new();
    assert_eq!(
        parser.aparse("foo, bar, baz").await.unwrap(),
        vec!["foo", "bar", "baz"]
    );
}

#[tokio::test]
async fn test_numbered_aparse() {
    let parser = NumberedListOutputParser::new();
    let text = "1. foo\n2. bar\n3. baz";
    assert_eq!(
        parser.aparse(text).await.unwrap(),
        vec!["foo", "bar", "baz"]
    );
}

#[tokio::test]
async fn test_numbered_aparse_with_prefix() {
    let parser = NumberedListOutputParser::new();
    let text = "Items:\n\n1. apple\n\n2. banana\n\n3. cherry";
    assert_eq!(
        parser.aparse(text).await.unwrap(),
        vec!["apple", "banana", "cherry"]
    );
}

#[tokio::test]
async fn test_numbered_aparse_no_items() {
    let parser = NumberedListOutputParser::new();
    assert!(
        parser
            .aparse("No items in the list.")
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn test_markdown_aparse() {
    let parser = MarkdownListOutputParser::new();
    let text = "- foo\n- bar\n- baz";
    assert_eq!(
        parser.aparse(text).await.unwrap(),
        vec!["foo", "bar", "baz"]
    );
}

#[tokio::test]
async fn test_markdown_aparse_with_prefix() {
    let parser = MarkdownListOutputParser::new();
    let text = "Items:\n- apple\n- banana\n- cherry";
    assert_eq!(
        parser.aparse(text).await.unwrap(),
        vec!["apple", "banana", "cherry"]
    );
}

#[tokio::test]
async fn test_markdown_aparse_no_items() {
    let parser = MarkdownListOutputParser::new();
    assert!(
        parser
            .aparse("No items in the list.")
            .await
            .unwrap()
            .is_empty()
    );
}
