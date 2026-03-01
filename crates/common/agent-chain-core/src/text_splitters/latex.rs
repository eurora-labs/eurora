use std::collections::HashMap;

use async_trait::async_trait;
use serde_json::Value;

use crate::documents::{BaseDocumentTransformer, Document};
use crate::text_splitters::character::RecursiveCharacterTextSplitter;
use crate::text_splitters::{Language, TextSplitter, TextSplitterConfig};

pub struct LatexTextSplitter {
    inner: RecursiveCharacterTextSplitter,
}

impl LatexTextSplitter {
    pub fn new(config: TextSplitterConfig) -> Self {
        let separators =
            RecursiveCharacterTextSplitter::get_separators_for_language(Language::Latex);
        Self {
            inner: RecursiveCharacterTextSplitter::new(Some(separators), Some(true), config),
        }
    }
}

#[async_trait]
impl BaseDocumentTransformer for LatexTextSplitter {
    fn transform_documents(
        &self,
        documents: Vec<Document>,
        kwargs: HashMap<String, Value>,
    ) -> Result<Vec<Document>, Box<dyn std::error::Error + Send + Sync>> {
        self.inner.transform_documents(documents, kwargs)
    }
}

#[async_trait]
impl TextSplitter for LatexTextSplitter {
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
    fn test_latex_text_splitter() {
        let config = TextSplitterConfig::new(100, 0, None, None, None, None).unwrap();
        let splitter = LatexTextSplitter::new(config);
        let text = r"\section{Introduction}
Some text about the introduction.

\subsection{Background}
Some background text.

\section{Methods}
Methodology description.
";
        let result = splitter.split_text(text).unwrap();
        assert!(!result.is_empty());
    }
}
