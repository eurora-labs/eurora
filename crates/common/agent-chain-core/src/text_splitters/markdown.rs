use std::collections::HashMap;

use async_trait::async_trait;
use regex::Regex;
use serde_json::Value;

use crate::documents::{BaseDocumentTransformer, Document};
use crate::text_splitters::character::RecursiveCharacterTextSplitter;
use crate::text_splitters::{Language, TextSplitter, TextSplitterConfig};

pub struct MarkdownTextSplitter {
    inner: RecursiveCharacterTextSplitter,
}

impl MarkdownTextSplitter {
    pub fn new(config: TextSplitterConfig) -> Self {
        let separators =
            RecursiveCharacterTextSplitter::get_separators_for_language(Language::Markdown);
        Self {
            inner: RecursiveCharacterTextSplitter::new(Some(separators), Some(true), config),
        }
    }
}

#[async_trait]
impl BaseDocumentTransformer for MarkdownTextSplitter {
    fn transform_documents(
        &self,
        documents: Vec<Document>,
        kwargs: HashMap<String, Value>,
    ) -> Result<Vec<Document>, Box<dyn std::error::Error + Send + Sync>> {
        self.inner.transform_documents(documents, kwargs)
    }
}

#[async_trait]
impl TextSplitter for MarkdownTextSplitter {
    fn config(&self) -> &TextSplitterConfig {
        self.inner.config()
    }

    fn split_text(
        &self,
        text: &str,
    ) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
        self.inner.split_text(text)
    }
}

pub struct LineType {
    pub metadata: HashMap<String, String>,
    pub content: String,
}

pub struct HeaderType {
    pub level: usize,
    pub name: String,
    pub data: String,
}

pub struct MarkdownHeaderTextSplitter {
    headers_to_split_on: Vec<(String, String)>,
    return_each_line: bool,
    strip_headers: bool,
    custom_header_patterns: HashMap<String, usize>,
}

impl MarkdownHeaderTextSplitter {
    pub fn new(
        headers_to_split_on: Vec<(String, String)>,
        return_each_line: bool,
        strip_headers: bool,
        custom_header_patterns: Option<HashMap<String, usize>>,
    ) -> Self {
        let mut headers = headers_to_split_on;
        headers.sort_by(|a, b| b.0.len().cmp(&a.0.len()));
        Self {
            headers_to_split_on: headers,
            return_each_line,
            strip_headers,
            custom_header_patterns: custom_header_patterns.unwrap_or_default(),
        }
    }

    fn is_custom_header(&self, line: &str, sep: &str) -> bool {
        let level = match self.custom_header_patterns.get(sep) {
            Some(level) => *level,
            Option::None => return false,
        };
        let _ = level;

        let escaped_sep = regex::escape(sep);
        let pattern =
            format!("^{escaped_sep}(?!{escaped_sep})(.+?)(?<!{escaped_sep}){escaped_sep}$");

        if let Ok(re) = Regex::new(&pattern) {
            if let Some(captures) = re.captures(line) {
                if let Some(content_match) = captures.get(1) {
                    let content = content_match.as_str().trim();
                    if !content.is_empty() {
                        let only_sep_chars =
                            content.replace(' ', "").chars().all(|c| sep.contains(c));
                        return !only_sep_chars;
                    }
                }
            }
        }
        false
    }

    pub fn aggregate_lines_to_chunks(&self, lines: Vec<LineType>) -> Vec<Document> {
        let mut aggregated: Vec<LineType> = Vec::new();

        for line in lines {
            let should_append_to_last = if let Some(last) = aggregated.last() {
                if last.metadata == line.metadata {
                    true
                } else if last.metadata.len() < line.metadata.len() && !self.strip_headers {
                    if let Some(last_line) = last.content.split('\n').last() {
                        last_line.starts_with('#')
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            };

            if should_append_to_last {
                if let Some(last) = aggregated.last_mut() {
                    last.content.push_str("  \n");
                    last.content.push_str(&line.content);
                    last.metadata = line.metadata;
                }
            } else {
                aggregated.push(line);
            }
        }

        aggregated
            .into_iter()
            .map(|chunk| {
                let mut metadata = HashMap::new();
                for (key, value) in chunk.metadata {
                    metadata.insert(key, Value::String(value));
                }
                Document::builder()
                    .page_content(chunk.content)
                    .metadata(metadata)
                    .build()
            })
            .collect()
    }

    pub fn split_text(
        &self,
        text: &str,
    ) -> Result<Vec<Document>, Box<dyn std::error::Error + Send + Sync>> {
        let lines: Vec<&str> = text.split('\n').collect();
        let mut lines_with_metadata: Vec<LineType> = Vec::new();
        let mut current_content: Vec<String> = Vec::new();
        let mut current_metadata: HashMap<String, String> = HashMap::new();
        let mut header_stack: Vec<HeaderType> = Vec::new();
        let mut initial_metadata: HashMap<String, String> = HashMap::new();

        let mut in_code_block = false;
        let mut opening_fence = String::new();

        for line in lines {
            let stripped_line = line.trim();
            let stripped_line: String = stripped_line
                .chars()
                .filter(|c| !c.is_control() || *c == '\t' || *c == '\n')
                .collect();

            if !in_code_block {
                if stripped_line.starts_with("```") && stripped_line.matches("```").count() == 1 {
                    in_code_block = true;
                    opening_fence = "```".to_string();
                } else if stripped_line.starts_with("~~~") {
                    in_code_block = true;
                    opening_fence = "~~~".to_string();
                }
            } else if stripped_line.starts_with(&opening_fence) {
                in_code_block = false;
                opening_fence.clear();
            }

            if in_code_block {
                current_content.push(stripped_line);
                continue;
            }

            let mut matched = false;
            for (sep, name) in &self.headers_to_split_on {
                let is_standard_header = stripped_line.starts_with(sep)
                    && (stripped_line.len() == sep.len()
                        || stripped_line.as_bytes().get(sep.len()) == Some(&b' '));

                let is_custom_header = self.is_custom_header(&stripped_line, sep);

                if is_standard_header || is_custom_header {
                    let current_header_level =
                        if self.custom_header_patterns.contains_key(sep.as_str()) {
                            self.custom_header_patterns[sep.as_str()]
                        } else {
                            sep.chars().filter(|c| *c == '#').count()
                        };

                    while let Some(last) = header_stack.last() {
                        if last.level >= current_header_level {
                            let popped = header_stack.pop().expect("checked non-empty");
                            initial_metadata.remove(&popped.name);
                        } else {
                            break;
                        }
                    }

                    let header_text = if is_custom_header {
                        stripped_line[sep.len()..stripped_line.len() - sep.len()]
                            .trim()
                            .to_string()
                    } else {
                        stripped_line[sep.len()..].trim().to_string()
                    };

                    let header = HeaderType {
                        level: current_header_level,
                        name: name.clone(),
                        data: header_text,
                    };
                    initial_metadata.insert(name.clone(), header.data.clone());
                    header_stack.push(header);

                    if !current_content.is_empty() {
                        lines_with_metadata.push(LineType {
                            content: current_content.join("\n"),
                            metadata: current_metadata.clone(),
                        });
                        current_content.clear();
                    }

                    if !self.strip_headers {
                        current_content.push(stripped_line.clone());
                    }

                    matched = true;
                    break;
                }
            }

            if !matched {
                if !stripped_line.is_empty() {
                    current_content.push(stripped_line);
                } else if !current_content.is_empty() {
                    lines_with_metadata.push(LineType {
                        content: current_content.join("\n"),
                        metadata: current_metadata.clone(),
                    });
                    current_content.clear();
                }
            }

            current_metadata = initial_metadata.clone();
        }

        if !current_content.is_empty() {
            lines_with_metadata.push(LineType {
                content: current_content.join("\n"),
                metadata: current_metadata,
            });
        }

        if !self.return_each_line {
            Ok(self.aggregate_lines_to_chunks(lines_with_metadata))
        } else {
            Ok(lines_with_metadata
                .into_iter()
                .map(|chunk| {
                    let mut metadata = HashMap::new();
                    for (key, value) in chunk.metadata {
                        metadata.insert(key, Value::String(value));
                    }
                    Document::builder()
                        .page_content(chunk.content)
                        .metadata(metadata)
                        .build()
                })
                .collect())
        }
    }
}

pub struct ExperimentalMarkdownSyntaxTextSplitter {
    splittable_headers: HashMap<String, String>,
    strip_headers: bool,
    return_each_line: bool,
}

impl ExperimentalMarkdownSyntaxTextSplitter {
    const DEFAULT_HEADER_KEYS: &[(&str, &str)] = &[
        ("#", "Header 1"),
        ("##", "Header 2"),
        ("###", "Header 3"),
        ("####", "Header 4"),
        ("#####", "Header 5"),
        ("######", "Header 6"),
    ];

    pub fn new(
        headers_to_split_on: Option<Vec<(String, String)>>,
        return_each_line: bool,
        strip_headers: bool,
    ) -> Self {
        let splittable_headers = if let Some(headers) = headers_to_split_on {
            headers.into_iter().collect()
        } else {
            Self::DEFAULT_HEADER_KEYS
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect()
        };
        Self {
            splittable_headers,
            strip_headers,
            return_each_line,
        }
    }

    fn match_header<'a>(&self, line: &'a str) -> Option<(usize, &'a str)> {
        let re = Regex::new(r"^(#{1,6}) (.*)").ok()?;
        let captures = re.captures(line)?;
        let hashes = captures.get(1)?.as_str();
        if self.splittable_headers.contains_key(hashes) {
            Some((hashes.len(), captures.get(2)?.as_str()))
        } else {
            Option::None
        }
    }

    fn match_code(line: &str) -> Option<String> {
        if let Some(rest) = line.strip_prefix("```") {
            return Some(rest.to_string());
        }
        if let Some(rest) = line.strip_prefix("~~~") {
            return Some(rest.to_string());
        }
        Option::None
    }

    fn match_horz(line: &str) -> bool {
        let patterns = [r"^\*\*\*+\n?$", r"^---+\n?$", r"^___+\n?$"];
        patterns
            .iter()
            .any(|p| Regex::new(p).map(|re| re.is_match(line)).unwrap_or(false))
    }

    fn resolve_header_stack(
        stack: &mut Vec<(usize, String)>,
        header_depth: usize,
        header_text: &str,
    ) {
        if let Some(pos) = stack.iter().position(|(depth, _)| *depth >= header_depth) {
            stack.truncate(pos);
        }
        stack.push((header_depth, header_text.to_string()));
    }

    fn resolve_code_chunk(current_line: &str, raw_lines: &mut Vec<String>) -> String {
        let mut chunk = current_line.to_string();
        while let Some(raw_line) = raw_lines.first().cloned() {
            raw_lines.remove(0);
            chunk.push_str(&raw_line);
            if Self::match_code(&raw_line).is_some() {
                return chunk;
            }
        }
        String::new()
    }

    pub fn split_text(
        &self,
        text: &str,
    ) -> Result<Vec<Document>, Box<dyn std::error::Error + Send + Sync>> {
        let mut chunks: Vec<Document> = Vec::new();
        let mut current_chunk_content = String::new();
        let mut current_chunk_metadata: HashMap<String, Value> = HashMap::new();
        let mut current_header_stack: Vec<(usize, String)> = Vec::new();

        let mut raw_lines: Vec<String> =
            text.split_inclusive('\n').map(|s| s.to_string()).collect();
        if !text.ends_with('\n') && !text.is_empty() {
            // split_inclusive won't add a trailing element for text not ending in \n
            // but the last element already contains everything - nothing to do
        }

        let complete_chunk =
            |chunks: &mut Vec<Document>,
             content: &mut String,
             metadata: &mut HashMap<String, Value>,
             header_stack: &[(usize, String)],
             splittable_headers: &HashMap<String, String>| {
                let chunk_content = content.clone();
                if !chunk_content.is_empty() && !chunk_content.chars().all(|c| c.is_whitespace()) {
                    for (depth, value) in header_stack {
                        let key = "#".repeat(*depth);
                        if let Some(header_key) = splittable_headers.get(&key) {
                            metadata.insert(header_key.clone(), Value::String(value.clone()));
                        }
                    }
                    chunks.push(
                        Document::builder()
                            .page_content(chunk_content)
                            .metadata(metadata.clone())
                            .build(),
                    );
                }
                content.clear();
                *metadata = HashMap::new();
            };

        while !raw_lines.is_empty() {
            let raw_line = raw_lines.remove(0);

            if let Some((header_depth, header_text)) = self.match_header(&raw_line) {
                complete_chunk(
                    &mut chunks,
                    &mut current_chunk_content,
                    &mut current_chunk_metadata,
                    &current_header_stack,
                    &self.splittable_headers,
                );

                if !self.strip_headers {
                    current_chunk_content.push_str(&raw_line);
                }

                Self::resolve_header_stack(&mut current_header_stack, header_depth, header_text);
            } else if let Some(code_lang) = Self::match_code(&raw_line) {
                complete_chunk(
                    &mut chunks,
                    &mut current_chunk_content,
                    &mut current_chunk_metadata,
                    &current_header_stack,
                    &self.splittable_headers,
                );

                current_chunk_content = Self::resolve_code_chunk(&raw_line, &mut raw_lines);
                current_chunk_metadata.insert("Code".to_string(), Value::String(code_lang));

                complete_chunk(
                    &mut chunks,
                    &mut current_chunk_content,
                    &mut current_chunk_metadata,
                    &current_header_stack,
                    &self.splittable_headers,
                );
            } else if Self::match_horz(&raw_line) {
                complete_chunk(
                    &mut chunks,
                    &mut current_chunk_content,
                    &mut current_chunk_metadata,
                    &current_header_stack,
                    &self.splittable_headers,
                );
            } else {
                current_chunk_content.push_str(&raw_line);
            }
        }

        complete_chunk(
            &mut chunks,
            &mut current_chunk_content,
            &mut current_chunk_metadata,
            &current_header_stack,
            &self.splittable_headers,
        );

        if self.return_each_line {
            Ok(chunks
                .into_iter()
                .flat_map(|chunk| {
                    chunk
                        .page_content
                        .lines()
                        .filter(|line| !line.is_empty() && !line.chars().all(|c| c.is_whitespace()))
                        .map(|line| {
                            Document::builder()
                                .page_content(line)
                                .metadata(chunk.metadata.clone())
                                .build()
                        })
                        .collect::<Vec<_>>()
                })
                .collect())
        } else {
            Ok(chunks)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_markdown_header_text_splitter_basic() {
        let splitter = MarkdownHeaderTextSplitter::new(
            vec![
                ("#".to_string(), "Header 1".to_string()),
                ("##".to_string(), "Header 2".to_string()),
                ("###".to_string(), "Header 3".to_string()),
            ],
            false,
            true,
            None,
        );

        let text = "# Foo\n\nBar\n## Baz\n\nQux\n### Quux\n\nCorge";
        let docs = splitter.split_text(text).unwrap();
        assert!(!docs.is_empty());
        assert_eq!(docs[0].page_content, "Bar");
        assert_eq!(docs[0].metadata["Header 1"], "Foo");
    }

    #[test]
    fn test_markdown_header_text_splitter_with_code_blocks() {
        let splitter = MarkdownHeaderTextSplitter::new(
            vec![
                ("#".to_string(), "Header 1".to_string()),
                ("##".to_string(), "Header 2".to_string()),
            ],
            false,
            true,
            None,
        );

        let text = "# Title\n\n```\n# Not a header\n```\n\nSome text";
        let docs = splitter.split_text(text).unwrap();
        assert!(!docs.is_empty());
    }

    #[test]
    fn test_markdown_header_text_splitter_return_each_line() {
        let splitter = MarkdownHeaderTextSplitter::new(
            vec![("#".to_string(), "Header 1".to_string())],
            true,
            true,
            None,
        );

        let text = "# Foo\n\nBar\nBaz\n\nQux";
        let docs = splitter.split_text(text).unwrap();
        // return_each_line returns each LineType separately (blank lines cause splits)
        assert!(
            docs.len() >= 2,
            "Expected at least 2 docs, got {}: {:?}",
            docs.len(),
            docs.iter().map(|d| &d.page_content).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_markdown_text_splitter() {
        let config = TextSplitterConfig::new(50, 0, None, None, None, None).unwrap();
        let splitter = MarkdownTextSplitter::new(config);
        let text = "# Header\n\nSome text\n\n## Subheader\n\nMore text";
        let result = splitter.split_text(text).unwrap();
        assert!(!result.is_empty());
    }

    #[test]
    fn test_experimental_markdown_splitter_basic() {
        let splitter = ExperimentalMarkdownSyntaxTextSplitter::new(None, false, true);
        let text = "# Title\n\nSome content\n\n## Subtitle\n\nMore content\n";
        let docs = splitter.split_text(text).unwrap();
        assert!(!docs.is_empty());
        assert_eq!(docs[0].page_content.trim(), "Some content");
    }

    #[test]
    fn test_experimental_markdown_splitter_code_blocks() {
        let splitter = ExperimentalMarkdownSyntaxTextSplitter::new(None, false, true);
        let text = "# Title\n\n```python\nprint('hello')\n```\n\nSome text\n";
        let docs = splitter.split_text(text).unwrap();
        assert!(docs.len() >= 2);
        let code_doc = docs.iter().find(|d| d.metadata.contains_key("Code"));
        assert!(code_doc.is_some());
    }

    #[test]
    fn test_experimental_markdown_splitter_horizontal_rule() {
        let splitter = ExperimentalMarkdownSyntaxTextSplitter::new(None, false, true);
        let text = "Part 1\n---\nPart 2\n";
        let docs = splitter.split_text(text).unwrap();
        assert_eq!(docs.len(), 2);
    }

    #[test]
    fn test_experimental_markdown_splitter_return_each_line() {
        let splitter = ExperimentalMarkdownSyntaxTextSplitter::new(None, true, true);
        let text = "# Title\n\nLine 1\nLine 2\n";
        let docs = splitter.split_text(text).unwrap();
        assert!(docs.len() >= 2);
    }
}
