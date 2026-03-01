pub mod base;
pub mod character;
pub mod json;
pub mod jsx;
pub mod latex;
pub mod markdown;
pub mod python;

pub use base::{
    CharacterTextSplitter, CharacterTextSplitterConfig, join_docs, merge_splits,
    split_text_with_regex,
};
pub use character::RecursiveCharacterTextSplitter;
pub use json::RecursiveJsonSplitter;
pub use jsx::JSFrameworkTextSplitter;
pub use latex::LatexTextSplitter;
pub use markdown::{
    ExperimentalMarkdownSyntaxTextSplitter, MarkdownHeaderTextSplitter, MarkdownTextSplitter,
};
pub use python::PythonCodeTextSplitter;

use crate::documents::{BaseDocumentTransformer, Document};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeepSeparator {
    None,
    Start,
    End,
}

impl Default for KeepSeparator {
    fn default() -> Self {
        KeepSeparator::None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    Cpp,
    Go,
    Java,
    Kotlin,
    Js,
    Ts,
    Php,
    Proto,
    Python,
    R,
    Rst,
    Ruby,
    Rust,
    Scala,
    Swift,
    Markdown,
    Latex,
    Html,
    Sol,
    CSharp,
    Cobol,
    C,
    Lua,
    Perl,
    Haskell,
    Elixir,
    PowerShell,
    VisualBasic6,
}

pub type LengthFunction = Arc<dyn Fn(&str) -> usize + Send + Sync>;

#[derive(Clone)]
pub struct TextSplitterConfig {
    pub chunk_size: usize,
    pub chunk_overlap: usize,
    pub length_function: LengthFunction,
    pub keep_separator: KeepSeparator,
    pub add_start_index: bool,
    pub strip_whitespace: bool,
}

impl std::fmt::Debug for TextSplitterConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TextSplitterConfig")
            .field("chunk_size", &self.chunk_size)
            .field("chunk_overlap", &self.chunk_overlap)
            .field("keep_separator", &self.keep_separator)
            .field("add_start_index", &self.add_start_index)
            .field("strip_whitespace", &self.strip_whitespace)
            .finish()
    }
}

impl Default for TextSplitterConfig {
    fn default() -> Self {
        Self {
            chunk_size: 4000,
            chunk_overlap: 200,
            length_function: Arc::new(|s: &str| s.len()),
            keep_separator: KeepSeparator::None,
            add_start_index: false,
            strip_whitespace: true,
        }
    }
}

impl TextSplitterConfig {
    pub fn new(
        chunk_size: usize,
        chunk_overlap: usize,
        length_function: Option<LengthFunction>,
        keep_separator: Option<KeepSeparator>,
        add_start_index: Option<bool>,
        strip_whitespace: Option<bool>,
    ) -> Result<Self, crate::Error> {
        if chunk_size == 0 {
            return Err(crate::Error::ValidationError(format!(
                "chunk_size must be > 0, got {}",
                chunk_size
            )));
        }
        if chunk_overlap > chunk_size {
            return Err(crate::Error::ValidationError(format!(
                "Got a larger chunk overlap ({}) than chunk size ({}), should be smaller.",
                chunk_overlap, chunk_size
            )));
        }
        Ok(Self {
            chunk_size,
            chunk_overlap,
            length_function: length_function.unwrap_or_else(|| Arc::new(|s: &str| s.len())),
            keep_separator: keep_separator.unwrap_or_default(),
            add_start_index: add_start_index.unwrap_or(false),
            strip_whitespace: strip_whitespace.unwrap_or(true),
        })
    }
}

pub struct Tokenizer {
    pub chunk_overlap: usize,
    pub tokens_per_chunk: usize,
    pub decode: Box<dyn Fn(&[i64]) -> String + Send + Sync>,
    pub encode: Box<dyn Fn(&str) -> Vec<i64> + Send + Sync>,
}

pub fn split_text_on_tokens(
    text: &str,
    tokenizer: &Tokenizer,
) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
    if tokenizer.tokens_per_chunk <= tokenizer.chunk_overlap {
        return Err(Box::new(crate::Error::ValidationError(
            "tokens_per_chunk must be greater than chunk_overlap".to_string(),
        )));
    }
    let input_ids = (tokenizer.encode)(text);
    let mut splits = Vec::new();
    let mut start_idx = 0;
    let step = tokenizer.tokens_per_chunk - tokenizer.chunk_overlap;

    while start_idx < input_ids.len() {
        let cur_idx = (start_idx + tokenizer.tokens_per_chunk).min(input_ids.len());
        let chunk_ids = &input_ids[start_idx..cur_idx];
        if chunk_ids.is_empty() {
            break;
        }
        let decoded = (tokenizer.decode)(chunk_ids);
        if !decoded.is_empty() {
            splits.push(decoded);
        }
        if cur_idx == input_ids.len() {
            break;
        }
        start_idx += step;
    }
    Ok(splits)
}

#[async_trait]
pub trait TextSplitter: BaseDocumentTransformer {
    fn config(&self) -> &TextSplitterConfig;

    fn split_text(
        &self,
        text: &str,
    ) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>>;

    fn create_documents(
        &self,
        texts: &[String],
        metadatas: Option<&[HashMap<String, serde_json::Value>]>,
    ) -> Result<Vec<Document>, Box<dyn std::error::Error + Send + Sync>> {
        let config = self.config();
        let empty: Vec<HashMap<String, serde_json::Value>> = vec![HashMap::new(); texts.len()];
        let metadatas = metadatas.unwrap_or(&empty);

        let mut documents = Vec::new();
        for (i, text) in texts.iter().enumerate() {
            let metadata = metadatas.get(i).cloned().unwrap_or_default();
            let mut index: usize = 0;
            let mut previous_chunk_len: usize = 0;
            for chunk in self.split_text(text)? {
                let mut doc_metadata = metadata.clone();
                if config.add_start_index {
                    let offset = (index + previous_chunk_len).saturating_sub(config.chunk_overlap);
                    index = text[offset..]
                        .find(&chunk)
                        .map(|pos| pos + offset)
                        .unwrap_or(index);
                    doc_metadata.insert(
                        "start_index".to_string(),
                        serde_json::Value::Number(serde_json::Number::from(index)),
                    );
                    previous_chunk_len = chunk.len();
                }
                let doc = Document::builder()
                    .page_content(chunk)
                    .metadata(doc_metadata)
                    .build();
                documents.push(doc);
            }
        }
        Ok(documents)
    }

    fn split_documents(
        &self,
        documents: &[Document],
    ) -> Result<Vec<Document>, Box<dyn std::error::Error + Send + Sync>> {
        let texts: Vec<String> = documents.iter().map(|d| d.page_content.clone()).collect();
        let metadatas: Vec<HashMap<String, serde_json::Value>> =
            documents.iter().map(|d| d.metadata.clone()).collect();
        self.create_documents(&texts, Some(&metadatas))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_splitter_config_defaults() {
        let config = TextSplitterConfig::default();
        assert_eq!(config.chunk_size, 4000);
        assert_eq!(config.chunk_overlap, 200);
        assert_eq!(config.keep_separator, KeepSeparator::None);
        assert!(!config.add_start_index);
        assert!(config.strip_whitespace);
        assert_eq!((config.length_function)("hello"), 5);
    }

    #[test]
    fn test_text_splitter_config_validation() {
        assert!(TextSplitterConfig::new(0, 0, None, None, None, None).is_err());
        assert!(TextSplitterConfig::new(2, 4, None, None, None, None).is_err());
        assert!(TextSplitterConfig::new(4, 2, None, None, None, None).is_ok());
    }

    #[test]
    fn test_split_text_on_tokens() {
        let tokenizer = Tokenizer {
            chunk_overlap: 1,
            tokens_per_chunk: 3,
            encode: Box::new(|text: &str| text.chars().map(|c| c as i64).collect()),
            decode: Box::new(|ids: &[i64]| {
                ids.iter()
                    .map(|&id| char::from_u32(id as u32).unwrap_or('?'))
                    .collect()
            }),
        };
        let result = split_text_on_tokens("abcdef", &tokenizer).unwrap();
        assert_eq!(result, vec!["abc", "cde", "ef"]);
    }

    #[test]
    fn test_split_text_on_tokens_validation() {
        let tokenizer = Tokenizer {
            chunk_overlap: 3,
            tokens_per_chunk: 3,
            encode: Box::new(|_: &str| vec![]),
            decode: Box::new(|_: &[i64]| String::new()),
        };
        assert!(split_text_on_tokens("test", &tokenizer).is_err());
    }

    struct NewlineSplitter {
        config: TextSplitterConfig,
    }

    impl NewlineSplitter {
        fn new() -> Self {
            Self {
                config: TextSplitterConfig::default(),
            }
        }
    }

    #[async_trait]
    impl BaseDocumentTransformer for NewlineSplitter {
        fn transform_documents(
            &self,
            documents: Vec<Document>,
            _kwargs: HashMap<String, serde_json::Value>,
        ) -> Result<Vec<Document>, Box<dyn std::error::Error + Send + Sync>> {
            self.split_documents(&documents)
        }
    }

    #[async_trait]
    impl TextSplitter for NewlineSplitter {
        fn config(&self) -> &TextSplitterConfig {
            &self.config
        }

        fn split_text(
            &self,
            text: &str,
        ) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
            Ok(text
                .split('\n')
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .collect())
        }
    }

    #[test]
    fn test_split_text() {
        let splitter = NewlineSplitter::new();
        let chunks = splitter.split_text("hello\nworld\n").unwrap();
        assert_eq!(chunks, vec!["hello", "world"]);
    }

    #[test]
    fn test_create_documents() {
        let splitter = NewlineSplitter::new();
        let texts = vec!["hello\nworld".to_string()];
        let docs = splitter.create_documents(&texts, None).unwrap();
        assert_eq!(docs.len(), 2);
        assert_eq!(docs[0].page_content, "hello");
        assert_eq!(docs[1].page_content, "world");
    }

    #[test]
    fn test_create_documents_with_metadata() {
        let splitter = NewlineSplitter::new();
        let texts = vec!["a\nb".to_string()];
        let metadata = vec![HashMap::from([(
            "source".to_string(),
            serde_json::json!("test.txt"),
        )])];
        let docs = splitter.create_documents(&texts, Some(&metadata)).unwrap();
        assert_eq!(docs.len(), 2);
        assert_eq!(docs[0].metadata["source"], "test.txt");
        assert_eq!(docs[1].metadata["source"], "test.txt");
    }

    #[test]
    fn test_split_documents() {
        let splitter = NewlineSplitter::new();
        let input_docs = vec![
            Document::builder().page_content("hello\nworld").build(),
            Document::builder().page_content("foo\nbar\nbaz").build(),
        ];
        let result = splitter.split_documents(&input_docs).unwrap();
        assert_eq!(result.len(), 5);
        assert_eq!(result[0].page_content, "hello");
        assert_eq!(result[1].page_content, "world");
        assert_eq!(result[2].page_content, "foo");
        assert_eq!(result[3].page_content, "bar");
        assert_eq!(result[4].page_content, "baz");
    }

    #[test]
    fn test_split_documents_preserves_metadata() {
        let splitter = NewlineSplitter::new();
        let mut metadata = HashMap::new();
        metadata.insert("key".to_string(), serde_json::json!("value"));
        let input_docs = vec![
            Document::builder()
                .page_content("a\nb")
                .metadata(metadata)
                .build(),
        ];
        let result = splitter.split_documents(&input_docs).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].metadata["key"], "value");
        assert_eq!(result[1].metadata["key"], "value");
    }

    #[test]
    fn test_transform_documents_delegates_to_split() {
        let splitter = NewlineSplitter::new();
        let docs = vec![Document::builder().page_content("x\ny").build()];
        let result = splitter.transform_documents(docs, HashMap::new()).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].page_content, "x");
        assert_eq!(result[1].page_content, "y");
    }
}
