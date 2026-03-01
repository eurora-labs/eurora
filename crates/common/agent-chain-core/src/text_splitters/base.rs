use std::collections::{HashMap, VecDeque};

use async_trait::async_trait;
use regex::Regex;
use serde_json::Value;

use crate::documents::{BaseDocumentTransformer, Document};
use crate::text_splitters::{KeepSeparator, TextSplitter, TextSplitterConfig};

pub fn split_text_with_regex(
    text: &str,
    separator: &str,
    keep_separator: KeepSeparator,
) -> Vec<String> {
    if separator.is_empty() {
        return text.chars().map(|c| c.to_string()).collect();
    }

    let regex = match Regex::new(separator) {
        Ok(r) => r,
        Err(_) => return vec![text.to_string()],
    };

    match keep_separator {
        KeepSeparator::None => regex
            .split(text)
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect(),

        KeepSeparator::Start | KeepSeparator::End => {
            let matches: Vec<_> = regex.find_iter(text).collect();
            if matches.is_empty() {
                return if text.is_empty() {
                    vec![]
                } else {
                    vec![text.to_string()]
                };
            }

            let mut splits = Vec::new();

            match keep_separator {
                KeepSeparator::End => {
                    let mut last_end = 0;
                    for m in &matches {
                        let piece = &text[last_end..m.end()];
                        if !piece.is_empty() {
                            splits.push(piece.to_string());
                        }
                        last_end = m.end();
                    }
                    if last_end < text.len() {
                        let remainder = &text[last_end..];
                        if !remainder.is_empty() {
                            splits.push(remainder.to_string());
                        }
                    }
                }
                KeepSeparator::Start => {
                    if matches[0].start() > 0 {
                        let prefix = &text[..matches[0].start()];
                        if !prefix.is_empty() {
                            splits.push(prefix.to_string());
                        }
                    }
                    for (i, m) in matches.iter().enumerate() {
                        let end = if i + 1 < matches.len() {
                            matches[i + 1].start()
                        } else {
                            text.len()
                        };
                        let piece = &text[m.start()..end];
                        if !piece.is_empty() {
                            splits.push(piece.to_string());
                        }
                    }
                }
                KeepSeparator::None => unreachable!(),
            }

            splits.into_iter().filter(|s| !s.is_empty()).collect()
        }
    }
}

pub fn join_docs(docs: &[String], separator: &str, strip_whitespace: bool) -> Option<String> {
    let text = docs.join(separator);
    let text = if strip_whitespace {
        text.trim().to_string()
    } else {
        text
    };
    if text.is_empty() {
        Option::None
    } else {
        Some(text)
    }
}

pub fn merge_splits(
    splits: &[String],
    separator: &str,
    config: &TextSplitterConfig,
) -> Vec<String> {
    let separator_len = (config.length_function)(separator);
    let mut docs: Vec<String> = Vec::new();
    let mut current_doc: VecDeque<String> = VecDeque::new();
    let mut total: usize = 0;

    for d in splits {
        let len = (config.length_function)(d);
        let separator_contribution = if current_doc.is_empty() {
            0
        } else {
            separator_len
        };

        if total + len + separator_contribution > config.chunk_size {
            if total > config.chunk_size {
                tracing::warn!(
                    "Created a chunk of size {}, which is longer than the specified {}",
                    total,
                    config.chunk_size,
                );
            }
            if !current_doc.is_empty() {
                let current_vec: Vec<String> = current_doc.iter().cloned().collect();
                if let Some(doc) = join_docs(&current_vec, separator, config.strip_whitespace) {
                    docs.push(doc);
                }
                while total > config.chunk_overlap
                    || (total
                        + len
                        + if current_doc.is_empty() {
                            0
                        } else {
                            separator_len
                        }
                        > config.chunk_size
                        && total > 0)
                {
                    if let Some(front) = current_doc.pop_front() {
                        let front_len = (config.length_function)(&front);
                        let sep_contribution = if current_doc.is_empty() {
                            0
                        } else {
                            separator_len
                        };
                        total = total.saturating_sub(front_len + sep_contribution);
                    } else {
                        break;
                    }
                }
            }
        }
        current_doc.push_back(d.clone());
        total += len
            + if current_doc.len() > 1 {
                separator_len
            } else {
                0
            };
    }

    let current_vec: Vec<String> = current_doc.iter().cloned().collect();
    if let Some(doc) = join_docs(&current_vec, separator, config.strip_whitespace) {
        docs.push(doc);
    }
    docs
}

pub struct CharacterTextSplitterConfig {
    pub separator: String,
    pub is_separator_regex: bool,
}

impl Default for CharacterTextSplitterConfig {
    fn default() -> Self {
        Self {
            separator: "\n\n".to_string(),
            is_separator_regex: false,
        }
    }
}

pub struct CharacterTextSplitter {
    config: TextSplitterConfig,
    separator: String,
    is_separator_regex: bool,
}

impl CharacterTextSplitter {
    pub fn new(splitter_config: CharacterTextSplitterConfig, config: TextSplitterConfig) -> Self {
        Self {
            config,
            separator: splitter_config.separator,
            is_separator_regex: splitter_config.is_separator_regex,
        }
    }
}

#[async_trait]
impl BaseDocumentTransformer for CharacterTextSplitter {
    fn transform_documents(
        &self,
        documents: Vec<Document>,
        _kwargs: HashMap<String, Value>,
    ) -> Result<Vec<Document>, Box<dyn std::error::Error + Send + Sync>> {
        self.split_documents(&documents)
    }
}

#[async_trait]
impl TextSplitter for CharacterTextSplitter {
    fn config(&self) -> &TextSplitterConfig {
        &self.config
    }

    fn split_text(
        &self,
        text: &str,
    ) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
        let sep_pattern = if self.is_separator_regex {
            self.separator.clone()
        } else {
            regex::escape(&self.separator)
        };

        let splits = split_text_with_regex(text, &sep_pattern, self.config.keep_separator);

        let lookaround_prefixes = ["(?=", "(?<!", "(?<=", "(?!"];
        let is_lookaround = self.is_separator_regex
            && lookaround_prefixes
                .iter()
                .any(|p| self.separator.starts_with(p));

        let merge_sep = if self.config.keep_separator != KeepSeparator::None || is_lookaround {
            String::new()
        } else {
            self.separator.clone()
        };

        Ok(merge_splits(&splits, &merge_sep, &self.config))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_character_text_splitter() {
        let config = TextSplitterConfig::new(7, 3, None, None, None, None).unwrap();
        let splitter = CharacterTextSplitter::new(
            CharacterTextSplitterConfig {
                separator: " ".to_string(),
                is_separator_regex: false,
            },
            config,
        );
        let output = splitter.split_text("foo bar baz 123").unwrap();
        assert_eq!(output, vec!["foo bar", "bar baz", "baz 123"]);
    }

    #[test]
    fn test_character_text_splitter_empty_doc() {
        let config = TextSplitterConfig::new(2, 0, None, None, None, None).unwrap();
        let splitter = CharacterTextSplitter::new(
            CharacterTextSplitterConfig {
                separator: " ".to_string(),
                is_separator_regex: false,
            },
            config,
        );
        let output = splitter.split_text("foo  bar").unwrap();
        assert_eq!(output, vec!["foo", "bar"]);
    }

    #[test]
    fn test_character_text_splitter_separator_empty_doc() {
        let config = TextSplitterConfig::new(2, 0, None, None, None, None).unwrap();
        let splitter = CharacterTextSplitter::new(
            CharacterTextSplitterConfig {
                separator: " ".to_string(),
                is_separator_regex: false,
            },
            config,
        );
        let output = splitter.split_text("f b").unwrap();
        assert_eq!(output, vec!["f", "b"]);
    }

    #[test]
    fn test_character_text_splitter_long() {
        let config = TextSplitterConfig::new(3, 1, None, None, None, None).unwrap();
        let splitter = CharacterTextSplitter::new(
            CharacterTextSplitterConfig {
                separator: " ".to_string(),
                is_separator_regex: false,
            },
            config,
        );
        let output = splitter.split_text("foo bar baz a a").unwrap();
        assert_eq!(output, vec!["foo", "bar", "baz", "a a"]);
    }

    #[test]
    fn test_character_text_splitter_short_words_first() {
        let config = TextSplitterConfig::new(3, 1, None, None, None, None).unwrap();
        let splitter = CharacterTextSplitter::new(
            CharacterTextSplitterConfig {
                separator: " ".to_string(),
                is_separator_regex: false,
            },
            config,
        );
        let output = splitter.split_text("a a foo bar baz").unwrap();
        assert_eq!(output, vec!["a a", "foo", "bar", "baz"]);
    }

    #[test]
    fn test_character_text_splitter_longer_words() {
        let config = TextSplitterConfig::new(1, 1, None, None, None, None).unwrap();
        let splitter = CharacterTextSplitter::new(
            CharacterTextSplitterConfig {
                separator: " ".to_string(),
                is_separator_regex: false,
            },
            config,
        );
        let output = splitter.split_text("foo bar baz 123").unwrap();
        assert_eq!(output, vec!["foo", "bar", "baz", "123"]);
    }

    #[test]
    fn test_character_text_splitter_keep_separator_start() {
        let config =
            TextSplitterConfig::new(1, 0, None, Some(KeepSeparator::Start), None, None).unwrap();
        let splitter = CharacterTextSplitter::new(
            CharacterTextSplitterConfig {
                separator: regex::escape("."),
                is_separator_regex: true,
            },
            config,
        );
        let output = splitter.split_text("foo.bar.baz.123").unwrap();
        assert_eq!(output, vec!["foo", ".bar", ".baz", ".123"]);
    }

    #[test]
    fn test_character_text_splitter_keep_separator_end() {
        let config =
            TextSplitterConfig::new(1, 0, None, Some(KeepSeparator::End), None, None).unwrap();
        let splitter = CharacterTextSplitter::new(
            CharacterTextSplitterConfig {
                separator: regex::escape("."),
                is_separator_regex: true,
            },
            config,
        );
        let output = splitter.split_text("foo.bar.baz.123").unwrap();
        assert_eq!(output, vec!["foo.", "bar.", "baz.", "123"]);
    }

    #[test]
    fn test_character_text_splitter_discard_separator() {
        let config = TextSplitterConfig::new(1, 0, None, None, None, None).unwrap();
        let splitter = CharacterTextSplitter::new(
            CharacterTextSplitterConfig {
                separator: regex::escape("."),
                is_separator_regex: true,
            },
            config,
        );
        let output = splitter.split_text("foo.bar.baz.123").unwrap();
        assert_eq!(output, vec!["foo", "bar", "baz", "123"]);
    }

    #[test]
    fn test_character_text_splitter_keep_separator_literal() {
        let config =
            TextSplitterConfig::new(1, 0, None, Some(KeepSeparator::Start), None, None).unwrap();
        let splitter = CharacterTextSplitter::new(
            CharacterTextSplitterConfig {
                separator: ".".to_string(),
                is_separator_regex: false,
            },
            config,
        );
        let output = splitter.split_text("foo.bar.baz.123").unwrap();
        assert_eq!(output, vec!["foo", ".bar", ".baz", ".123"]);
    }

    #[test]
    fn test_merge_splits() {
        let config = TextSplitterConfig::new(9, 2, None, None, None, None).unwrap();
        let splits = vec!["foo".to_string(), "bar".to_string(), "baz".to_string()];
        let output = merge_splits(&splits, " ", &config);
        assert_eq!(output, vec!["foo bar", "baz"]);
    }

    #[test]
    fn test_create_documents() {
        let config = TextSplitterConfig::new(3, 0, None, None, None, None).unwrap();
        let splitter = CharacterTextSplitter::new(
            CharacterTextSplitterConfig {
                separator: " ".to_string(),
                is_separator_regex: false,
            },
            config,
        );
        let texts = vec!["foo bar".to_string(), "baz".to_string()];
        let docs = splitter.create_documents(&texts, None).unwrap();
        assert_eq!(docs.len(), 3);
        assert_eq!(docs[0].page_content, "foo");
        assert_eq!(docs[1].page_content, "bar");
        assert_eq!(docs[2].page_content, "baz");
    }

    #[test]
    fn test_create_documents_with_metadata() {
        let config = TextSplitterConfig::new(3, 0, None, None, None, None).unwrap();
        let splitter = CharacterTextSplitter::new(
            CharacterTextSplitterConfig {
                separator: " ".to_string(),
                is_separator_regex: false,
            },
            config,
        );
        let texts = vec!["foo bar".to_string(), "baz".to_string()];
        let metadatas = vec![
            HashMap::from([("source".to_string(), serde_json::json!("1"))]),
            HashMap::from([("source".to_string(), serde_json::json!("2"))]),
        ];
        let docs = splitter.create_documents(&texts, Some(&metadatas)).unwrap();
        assert_eq!(docs.len(), 3);
        assert_eq!(docs[0].page_content, "foo");
        assert_eq!(docs[0].metadata["source"], "1");
        assert_eq!(docs[1].page_content, "bar");
        assert_eq!(docs[1].metadata["source"], "1");
        assert_eq!(docs[2].page_content, "baz");
        assert_eq!(docs[2].metadata["source"], "2");
    }

    #[test]
    fn test_create_documents_with_start_index() {
        let config = TextSplitterConfig::new(7, 3, None, None, Some(true), None).unwrap();
        let splitter = CharacterTextSplitter::new(
            CharacterTextSplitterConfig {
                separator: " ".to_string(),
                is_separator_regex: false,
            },
            config,
        );
        let texts = vec!["foo bar baz 123".to_string()];
        let docs = splitter.create_documents(&texts, None).unwrap();
        assert_eq!(docs.len(), 3);
        assert_eq!(docs[0].page_content, "foo bar");
        assert_eq!(docs[0].metadata["start_index"], 0);
        assert_eq!(docs[1].page_content, "bar baz");
        assert_eq!(docs[1].metadata["start_index"], 4);
        assert_eq!(docs[2].page_content, "baz 123");
        assert_eq!(docs[2].metadata["start_index"], 8);
    }
}
