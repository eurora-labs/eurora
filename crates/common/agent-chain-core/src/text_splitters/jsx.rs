use std::collections::HashMap;

use async_trait::async_trait;
use regex::Regex;
use serde_json::Value;

use crate::documents::{BaseDocumentTransformer, Document};
use crate::text_splitters::character::RecursiveCharacterTextSplitter;
use crate::text_splitters::{TextSplitter, TextSplitterConfig};

pub struct JSFrameworkTextSplitter {
    config: TextSplitterConfig,
    custom_separators: Vec<String>,
}

impl JSFrameworkTextSplitter {
    pub fn new(separators: Option<Vec<String>>, config: TextSplitterConfig) -> Self {
        Self {
            config,
            custom_separators: separators.unwrap_or_default(),
        }
    }

    fn build_separators(&self, text: &str) -> Vec<String> {
        let tag_regex = Regex::new(r"<\s*([a-zA-Z0-9]+)[^>]*>")
            .unwrap_or_else(|_| Regex::new(r"<([a-zA-Z0-9]+)").expect("fallback regex is valid"));

        let mut component_tags: Vec<String> = Vec::new();
        for captures in tag_regex.captures_iter(text) {
            if let Some(tag) = captures.get(1) {
                let tag_str = tag.as_str().to_string();
                if !component_tags.contains(&tag_str) {
                    component_tags.push(tag_str);
                }
            }
        }
        let component_separators: Vec<String> = component_tags
            .iter()
            .map(|tag| format!("<{}", tag))
            .collect();

        let js_separators: Vec<String> = vec![
            "\nexport ",
            " export ",
            "\nfunction ",
            "\nasync function ",
            " async function ",
            "\nconst ",
            "\nlet ",
            "\nvar ",
            "\nclass ",
            " class ",
            "\nif ",
            " if ",
            "\nfor ",
            " for ",
            "\nwhile ",
            " while ",
            "\nswitch ",
            " switch ",
            "\ncase ",
            " case ",
            "\ndefault ",
            " default ",
        ]
        .into_iter()
        .map(|s| s.to_string())
        .collect();

        let tail_separators: Vec<String> = vec!["<>", "\n\n", "&&\n", "||\n"]
            .into_iter()
            .map(|s| s.to_string())
            .collect();

        let mut all_separators = self.custom_separators.clone();
        all_separators.extend(js_separators);
        all_separators.extend(component_separators);
        all_separators.extend(tail_separators);
        all_separators
    }
}

#[async_trait]
impl BaseDocumentTransformer for JSFrameworkTextSplitter {
    fn transform_documents(
        &self,
        documents: Vec<Document>,
        _kwargs: HashMap<String, Value>,
    ) -> Result<Vec<Document>, Box<dyn std::error::Error + Send + Sync>> {
        self.split_documents(&documents)
    }
}

#[async_trait]
impl TextSplitter for JSFrameworkTextSplitter {
    fn config(&self) -> &TextSplitterConfig {
        &self.config
    }

    fn split_text(
        &self,
        text: &str,
    ) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
        let separators = self.build_separators(text);
        let inner =
            RecursiveCharacterTextSplitter::new(Some(separators), None, self.config.clone());
        inner.split_text(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jsx_framework_text_splitter() {
        let config = TextSplitterConfig::new(100, 0, None, None, None, None).unwrap();
        let splitter = JSFrameworkTextSplitter::new(None, config);
        let text = r#"
function App() {
    return (
        <div>
            <Header title="Hello" />
            <Content>
                <p>Some text here</p>
            </Content>
        </div>
    );
}

const Footer = () => {
    return <footer>Footer content</footer>;
};
"#;
        let result = splitter.split_text(text).unwrap();
        assert!(!result.is_empty());
    }

    #[test]
    fn test_jsx_framework_text_splitter_extracts_tags() {
        let config = TextSplitterConfig::new(50, 0, None, None, None, None).unwrap();
        let splitter = JSFrameworkTextSplitter::new(None, config);
        let separators = splitter.build_separators("<div><span>text</span></div>");
        assert!(separators.contains(&"<div".to_string()));
        assert!(separators.contains(&"<span".to_string()));
    }
}
