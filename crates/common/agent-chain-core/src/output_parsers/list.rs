//! Parsers for list output.
//!
//! This module contains output parsers that parse LLM output into lists.
//! Mirrors `langchain_core.output_parsers.list`.

use std::collections::VecDeque;
use std::fmt::Debug;

use futures::stream::BoxStream;
use regex::Regex;

use crate::error::Result;
use crate::messages::BaseMessage;

use super::base::BaseOutputParser;
use super::transform::BaseTransformOutputParser;

/// A single match from `parse_iter`, carrying the captured group text
/// and the byte offset where the overall match ends in the input.
///
/// Mirrors the `re.Match` objects returned by Python's
/// `ListOutputParser.parse_iter`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseMatch {
    /// The text captured by the first group (equivalent to `m.group(1)`).
    pub group: String,
    /// The byte offset where the overall regex match ends in the input
    /// (equivalent to `m.end()`).
    pub end: usize,
}

/// Parse the output of a model to a list.
///
/// This is a base trait for list output parsers.
pub trait ListOutputParser: BaseOutputParser<Output = Vec<String>> {
    /// Parse the output iteratively, yielding match results.
    ///
    /// Returns a vector of [`ParseMatch`] values carrying the captured text
    /// and the end position of each match. Used for streaming parsing where
    /// the caller needs to know how much of the input has been consumed.
    ///
    /// The default implementation returns an empty vector.
    fn parse_iter(&self, _text: &str) -> Vec<ParseMatch> {
        Vec::new()
    }

    /// Parse the output without filtering empty strings.
    ///
    /// Used internally by the streaming transform fallback path.
    /// Python's CSV reader preserves empty fields from trailing commas
    /// (e.g., `"foo,"` → `["foo", ""]`), which the streaming logic relies on
    /// to detect complete items. The default delegates to `parse()`.
    fn parse_with_empties(&self, text: &str) -> Result<Vec<String>> {
        self.parse(text)
    }
}

/// Parse the output of a model to a comma-separated list.
///
/// # Example
///
/// ```ignore
/// use agent_chain_core::output_parsers::CommaSeparatedListOutputParser;
///
/// let parser = CommaSeparatedListOutputParser::new();
/// let result = parser.parse("apple, banana, cherry").unwrap();
/// assert_eq!(result, vec!["apple", "banana", "cherry"]);
/// ```
#[derive(Debug, Clone, Default)]
pub struct CommaSeparatedListOutputParser {
    _private: (),
}

impl CommaSeparatedListOutputParser {
    /// Create a new `CommaSeparatedListOutputParser`.
    pub fn new() -> Self {
        Self { _private: () }
    }

    /// Returns `true` as this class is serializable.
    pub fn is_lc_serializable() -> bool {
        true
    }

    /// Get the namespace of the LangChain object.
    pub fn get_lc_namespace() -> Vec<&'static str> {
        vec!["langchain", "output_parsers", "list"]
    }
}

impl BaseOutputParser for CommaSeparatedListOutputParser {
    type Output = Vec<String>;

    fn parse(&self, text: &str) -> Result<Vec<String>> {
        let mut reader = csv::ReaderBuilder::new()
            .has_headers(false)
            .flexible(true)
            .trim(csv::Trim::All)
            .from_reader(text.as_bytes());

        let mut result = Vec::new();
        for record in reader.records() {
            match record {
                Ok(rec) => {
                    for field in rec.iter() {
                        if !field.is_empty() {
                            result.push(field.to_string());
                        }
                    }
                }
                Err(_) => {
                    return Ok(text
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect());
                }
            }
        }

        if result.is_empty() {
            Ok(text
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect())
        } else {
            Ok(result)
        }
    }

    fn get_format_instructions(&self) -> Result<String> {
        Ok("Your response should be a list of comma separated values, \
             eg: `foo, bar, baz` or `foo,bar,baz`"
            .to_string())
    }

    fn parser_type(&self) -> &str {
        "comma-separated-list"
    }
}

impl BaseTransformOutputParser for CommaSeparatedListOutputParser {
    fn transform<'a>(
        &'a self,
        input: BoxStream<'a, BaseMessage>,
    ) -> BoxStream<'a, Result<Self::Output>>
    where
        Self::Output: 'a,
    {
        list_transform(self, input)
    }
}

impl ListOutputParser for CommaSeparatedListOutputParser {
    fn parse_with_empties(&self, text: &str) -> Result<Vec<String>> {
        let mut reader = csv::ReaderBuilder::new()
            .has_headers(false)
            .flexible(true)
            .trim(csv::Trim::All)
            .from_reader(text.as_bytes());

        let mut result = Vec::new();
        for record in reader.records() {
            match record {
                Ok(rec) => {
                    for field in rec.iter() {
                        result.push(field.to_string());
                    }
                }
                Err(_) => {
                    return Ok(text.split(',').map(|s| s.trim().to_string()).collect());
                }
            }
        }

        if result.is_empty() {
            Ok(text.split(',').map(|s| s.trim().to_string()).collect())
        } else {
            Ok(result)
        }
    }
}

/// Parse a numbered list.
///
/// # Example
///
/// ```ignore
/// use agent_chain_core::output_parsers::NumberedListOutputParser;
///
/// let parser = NumberedListOutputParser::new();
/// let result = parser.parse("1. apple\n2. banana\n3. cherry").unwrap();
/// assert_eq!(result, vec!["apple", "banana", "cherry"]);
/// ```
#[derive(Debug, Clone)]
pub struct NumberedListOutputParser {
    /// The regex pattern to match numbered list items.
    pub pattern: String,
}

impl NumberedListOutputParser {
    /// Create a new `NumberedListOutputParser`.
    pub fn new() -> Self {
        Self {
            pattern: r"\d+\.\s*([^\n]+)".to_string(),
        }
    }

    /// Create a parser with a custom pattern.
    pub fn with_pattern(pattern: impl Into<String>) -> Self {
        Self {
            pattern: pattern.into(),
        }
    }

    fn get_regex(&self) -> Result<Regex> {
        Regex::new(&self.pattern).map_err(|e| {
            crate::Error::InvalidConfig(format!("Invalid regex pattern '{}': {}", self.pattern, e))
        })
    }
}

impl Default for NumberedListOutputParser {
    fn default() -> Self {
        Self::new()
    }
}

impl BaseOutputParser for NumberedListOutputParser {
    type Output = Vec<String>;

    fn parse(&self, text: &str) -> Result<Vec<String>> {
        let re = self.get_regex()?;
        Ok(re
            .captures_iter(text)
            .filter_map(|cap| cap.get(1).map(|m| m.as_str().trim().to_string()))
            .collect())
    }

    fn get_format_instructions(&self) -> Result<String> {
        Ok(
            "Your response should be a numbered list with each item on a new line. \
             For example: \n\n1. foo\n\n2. bar\n\n3. baz"
                .to_string(),
        )
    }

    fn parser_type(&self) -> &str {
        "numbered-list"
    }
}

impl BaseTransformOutputParser for NumberedListOutputParser {
    fn transform<'a>(
        &'a self,
        input: BoxStream<'a, BaseMessage>,
    ) -> BoxStream<'a, Result<Self::Output>>
    where
        Self::Output: 'a,
    {
        list_transform(self, input)
    }
}

impl ListOutputParser for NumberedListOutputParser {
    fn parse_iter(&self, text: &str) -> Vec<ParseMatch> {
        let re = match self.get_regex() {
            Ok(re) => re,
            Err(_) => return Vec::new(),
        };
        re.captures_iter(text)
            .filter_map(|cap| {
                let overall = cap.get(0)?;
                let group = cap.get(1)?;
                Some(ParseMatch {
                    group: group.as_str().trim().to_string(),
                    end: overall.end(),
                })
            })
            .collect()
    }
}

/// Parse a Markdown list.
///
/// # Example
///
/// ```ignore
/// use agent_chain_core::output_parsers::MarkdownListOutputParser;
///
/// let parser = MarkdownListOutputParser::new();
/// let result = parser.parse("- apple\n- banana\n- cherry").unwrap();
/// assert_eq!(result, vec!["apple", "banana", "cherry"]);
/// ```
#[derive(Debug, Clone)]
pub struct MarkdownListOutputParser {
    /// The regex pattern to match Markdown list items.
    pub pattern: String,
}

impl MarkdownListOutputParser {
    /// Create a new `MarkdownListOutputParser`.
    pub fn new() -> Self {
        Self {
            pattern: r"^\s*[-*]\s+([^\n]+)$".to_string(),
        }
    }

    /// Create a parser with a custom pattern.
    pub fn with_pattern(pattern: impl Into<String>) -> Self {
        Self {
            pattern: pattern.into(),
        }
    }

    fn get_regex(&self) -> Result<Regex> {
        Regex::new(&self.pattern).map_err(|e| {
            crate::Error::InvalidConfig(format!("Invalid regex pattern '{}': {}", self.pattern, e))
        })
    }
}

impl Default for MarkdownListOutputParser {
    fn default() -> Self {
        Self::new()
    }
}

impl BaseOutputParser for MarkdownListOutputParser {
    type Output = Vec<String>;

    fn parse(&self, text: &str) -> Result<Vec<String>> {
        let re = self.get_regex()?;
        Ok(text
            .lines()
            .filter_map(|line| {
                re.captures(line)
                    .and_then(|cap| cap.get(1).map(|m| m.as_str().trim().to_string()))
            })
            .collect())
    }

    fn get_format_instructions(&self) -> Result<String> {
        Ok("Your response should be a markdown list, eg: `- foo\n- bar\n- baz`".to_string())
    }

    fn parser_type(&self) -> &str {
        "markdown-list"
    }
}

impl BaseTransformOutputParser for MarkdownListOutputParser {
    fn transform<'a>(
        &'a self,
        input: BoxStream<'a, BaseMessage>,
    ) -> BoxStream<'a, Result<Self::Output>>
    where
        Self::Output: 'a,
    {
        list_transform(self, input)
    }
}

impl ListOutputParser for MarkdownListOutputParser {
    fn parse_iter(&self, text: &str) -> Vec<ParseMatch> {
        let re = match self.get_regex() {
            Ok(re) => re,
            Err(_) => return Vec::new(),
        };
        let mut offset = 0;
        text.lines()
            .filter_map(|line| {
                let line_start = offset;
                offset += line.len() + 1;
                re.captures(line).and_then(|cap| {
                    let group = cap.get(1)?;
                    let overall = cap.get(0)?;
                    Some(ParseMatch {
                        group: group.as_str().trim().to_string(),
                        end: line_start + overall.end(),
                    })
                })
            })
            .collect()
    }
}

/// Streaming transform implementation for list parsers.
///
/// Mirrors the `_transform` method of Python's `ListOutputParser`. Accumulates
/// text from incoming message chunks and yields individual list items as they
/// become complete. For parsers that implement `parse_iter` (returning non-empty
/// results), it uses `drop_last_n` to avoid yielding the last (potentially
/// incomplete) item until the stream ends. For parsers without `parse_iter`
/// (like `CommaSeparatedListOutputParser`), it falls back to `parse()` and holds
/// back the last item.
fn list_transform<'a, P: ListOutputParser + 'a>(
    parser: &'a P,
    input: BoxStream<'a, BaseMessage>,
) -> BoxStream<'a, Result<Vec<String>>> {
    use futures::StreamExt;

    Box::pin(async_stream::stream! {
        let mut buffer = String::new();
        let mut stream = input;

        while let Some(message) = stream.next().await {
            let chunk_content = message.text();
            buffer.push_str(&chunk_content);

            let iter_results = parser.parse_iter(&buffer);
            if !iter_results.is_empty() {
                let mut done_idx = 0;
                for m in drop_last_n(iter_results.into_iter(), 1) {
                    done_idx = m.end;
                    yield Ok(vec![m.group]);
                }
                buffer = buffer[done_idx..].to_string();
            } else {
                match parser.parse_with_empties(&buffer) {
                    Ok(parts) => {
                        if parts.len() > 1 {
                            for part in &parts[..parts.len() - 1] {
                                if !part.is_empty() {
                                    yield Ok(vec![part.clone()]);
                                }
                            }
                            buffer = parts[parts.len() - 1].clone();
                        }
                    }
                    Err(err) => {
                        yield Err(err);
                    }
                }
            }
        }

        match parser.parse(&buffer) {
            Ok(parts) => {
                for part in parts {
                    yield Ok(vec![part]);
                }
            }
            Err(err) => {
                yield Err(err);
            }
        }
    })
}

/// Drop the last n elements of an iterator.
///
/// This is useful for streaming list parsing where we want to avoid
/// yielding incomplete items.
pub fn drop_last_n<T, I: Iterator<Item = T>>(iter: I, n: usize) -> impl Iterator<Item = T> {
    let mut buffer: VecDeque<T> = VecDeque::with_capacity(n);

    iter.filter_map(move |item| {
        buffer.push_back(item);
        if buffer.len() > n {
            buffer.pop_front()
        } else {
            None
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_comma_separated_list() {
        let parser = CommaSeparatedListOutputParser::new();
        let result = parser.parse("apple, banana, cherry").unwrap();
        assert_eq!(result, vec!["apple", "banana", "cherry"]);
    }

    #[test]
    fn test_comma_separated_list_no_spaces() {
        let parser = CommaSeparatedListOutputParser::new();
        let result = parser.parse("apple,banana,cherry").unwrap();
        assert_eq!(result, vec!["apple", "banana", "cherry"]);
    }

    #[test]
    fn test_comma_separated_list_quoted() {
        let parser = CommaSeparatedListOutputParser::new();
        let result = parser.parse(r#""hello, world", foo, bar"#).unwrap();
        assert_eq!(result, vec!["hello, world", "foo", "bar"]);
    }

    #[test]
    fn test_comma_separated_list_empty() {
        let parser = CommaSeparatedListOutputParser::new();
        let result = parser.parse("").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_numbered_list() {
        let parser = NumberedListOutputParser::new();
        let result = parser.parse("1. apple\n2. banana\n3. cherry").unwrap();
        assert_eq!(result, vec!["apple", "banana", "cherry"]);
    }

    #[test]
    fn test_numbered_list_with_spaces() {
        let parser = NumberedListOutputParser::new();
        let result = parser.parse("1.  apple\n2.  banana").unwrap();
        assert_eq!(result, vec!["apple", "banana"]);
    }

    #[test]
    fn test_numbered_list_empty() {
        let parser = NumberedListOutputParser::new();
        let result = parser.parse("").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_markdown_list_dash() {
        let parser = MarkdownListOutputParser::new();
        let result = parser.parse("- apple\n- banana\n- cherry").unwrap();
        assert_eq!(result, vec!["apple", "banana", "cherry"]);
    }

    #[test]
    fn test_markdown_list_asterisk() {
        let parser = MarkdownListOutputParser::new();
        let result = parser.parse("* apple\n* banana").unwrap();
        assert_eq!(result, vec!["apple", "banana"]);
    }

    #[test]
    fn test_markdown_list_indented() {
        let parser = MarkdownListOutputParser::new();
        let result = parser.parse("  - apple\n  - banana").unwrap();
        assert_eq!(result, vec!["apple", "banana"]);
    }

    #[test]
    fn test_markdown_list_empty() {
        let parser = MarkdownListOutputParser::new();
        let result = parser.parse("").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_format_instructions() {
        let parser = CommaSeparatedListOutputParser::new();
        let instructions = parser
            .get_format_instructions()
            .expect("should return format instructions");
        assert!(instructions.contains("comma separated"));
    }

    #[test]
    fn test_drop_last_n() {
        let items = vec![1, 2, 3, 4, 5];
        let result: Vec<_> = drop_last_n(items.into_iter(), 2).collect();
        assert_eq!(result, vec![1, 2, 3]);
    }

    #[test]
    fn test_drop_last_n_empty() {
        let items: Vec<i32> = vec![];
        let result: Vec<_> = drop_last_n(items.into_iter(), 2).collect();
        assert!(result.is_empty());
    }

    #[test]
    fn test_drop_last_n_less_than_n() {
        let items = vec![1, 2];
        let result: Vec<_> = drop_last_n(items.into_iter(), 5).collect();
        assert!(result.is_empty());
    }
}
