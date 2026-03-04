use async_trait::async_trait;

use crate::documents::{BaseDocumentTransformer, Document};
use crate::text_splitters::character::RecursiveCharacterTextSplitter;
use crate::text_splitters::{Language, TextSplitter, TextSplitterConfig};

pub struct PythonCodeTextSplitter {
    inner: RecursiveCharacterTextSplitter,
}

impl PythonCodeTextSplitter {
    pub fn new(config: TextSplitterConfig) -> Self {
        let separators =
            RecursiveCharacterTextSplitter::get_separators_for_language(Language::Python);
        Self {
            inner: RecursiveCharacterTextSplitter::builder()
                .separators(separators)
                .is_separator_regex(true)
                .config(config)
                .build(),
        }
    }
}

#[async_trait]
impl BaseDocumentTransformer for PythonCodeTextSplitter {
    fn transform_documents(
        &self,
        documents: &[Document],
    ) -> Result<Vec<Document>, Box<dyn std::error::Error + Send + Sync>> {
        self.inner.transform_documents(documents)
    }
}

#[async_trait]
impl TextSplitter for PythonCodeTextSplitter {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_python_code_text_splitter() {
        let config = TextSplitterConfig::new(50, 0, None, None, None, None).unwrap();
        let splitter = PythonCodeTextSplitter::new(config);
        let text =
            "\nclass Foo:\n\n    def bar():\n\n\ndef foo():\n\ndef testing_func():\n\ndef bar():\n";
        let result = splitter.split_text(text).unwrap();
        assert!(!result.is_empty());
    }
}
