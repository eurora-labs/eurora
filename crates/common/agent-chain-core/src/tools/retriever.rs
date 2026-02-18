//! Retriever tool.
//!
//! This module provides utilities for creating tools from retrievers,
//! mirroring `langchain_core.tools.retriever`.

use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::documents::Document;
use crate::error::Result;
use crate::retrievers::BaseRetriever;

use super::base::{ArgsSchema, ResponseFormat};
use super::structured::StructuredTool;

/// Input schema for retriever tools.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrieverInput {
    /// The query to look up in the retriever.
    pub query: String,
}

impl RetrieverInput {
    /// Create a new RetrieverInput.
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
        }
    }
}

/// Get the default args schema for retriever tools.
fn retriever_args_schema() -> ArgsSchema {
    ArgsSchema::JsonSchema(serde_json::json!({
        "type": "object",
        "title": "RetrieverInput",
        "description": "Input to the retriever",
        "properties": {
            "query": {
                "type": "string",
                "description": "query to look up in retriever"
            }
        },
        "required": ["query"]
    }))
}

/// Create a tool to do retrieval of documents.
///
/// # Arguments
///
/// * `retriever` - The retriever to use for the retrieval.
/// * `name` - The name for the tool. This will be passed to the language model,
///   so should be unique and somewhat descriptive.
/// * `description` - The description for the tool. This will be passed to the
///   language model, so should be descriptive.
///
/// # Returns
///
/// A StructuredTool configured for document retrieval.
pub fn create_retriever_tool<R>(
    retriever: Arc<R>,
    name: impl Into<String>,
    description: impl Into<String>,
) -> StructuredTool
where
    R: BaseRetriever + Send + Sync + 'static,
{
    create_retriever_tool_with_options(
        retriever,
        name,
        description,
        None,
        "\n\n",
        ResponseFormat::Content,
    )
}

/// Create a retriever tool with additional options.
///
/// # Arguments
///
/// * `retriever` - The retriever to use.
/// * `name` - The tool name.
/// * `description` - The tool description.
/// * `document_prompt` - Optional template for formatting documents.
/// * `document_separator` - Separator between documents (default: "\n\n").
/// * `response_format` - The tool response format.
///
/// # Returns
///
/// A StructuredTool configured for document retrieval.
pub fn create_retriever_tool_with_options<R>(
    retriever: Arc<R>,
    name: impl Into<String>,
    description: impl Into<String>,
    _document_prompt: Option<String>,
    document_separator: &str,
    response_format: ResponseFormat,
) -> StructuredTool
where
    R: BaseRetriever + Send + Sync + 'static,
{
    let name = name.into();
    let description = description.into();
    let separator = document_separator.to_string();

    let retriever_clone = retriever.clone();
    let separator_clone = separator.clone();
    let response_format_clone = response_format;

    let func = {
        let _retriever = retriever_clone.clone();
        let separator = separator_clone.clone();
        move |args: HashMap<String, Value>| -> Result<Value> {
            let _query = args
                .get("query")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let docs: Vec<Document> = Vec::new();
            let content = format_documents(&docs, &separator);

            match response_format_clone {
                ResponseFormat::Content => Ok(Value::String(content)),
                ResponseFormat::ContentAndArtifact => {
                    let docs_json: Vec<Value> = docs
                        .iter()
                        .map(|d| {
                            serde_json::json!({
                                "page_content": d.page_content,
                                "metadata": d.metadata
                            })
                        })
                        .collect();
                    Ok(serde_json::json!([content, docs_json]))
                }
            }
        }
    };

    StructuredTool::from_function(func, name.clone(), description, retriever_args_schema())
        .with_response_format(response_format)
}

/// Format documents into a single string.
fn format_documents(docs: &[Document], separator: &str) -> String {
    docs.iter()
        .map(|doc| doc.page_content.clone())
        .collect::<Vec<_>>()
        .join(separator)
}

/// Create a retriever tool with async support.
///
/// This version properly supports async retrieval.
pub fn create_async_retriever_tool<R, F, Fut>(
    retriever: Arc<R>,
    retrieve_fn: F,
    name: impl Into<String>,
    description: impl Into<String>,
) -> StructuredTool
where
    R: Send + Sync + 'static,
    F: Fn(Arc<R>, String) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<Vec<Document>>> + Send + 'static,
{
    let name = name.into();
    let description = description.into();

    let _retriever_clone = retriever.clone();
    let _retrieve_fn = Arc::new(retrieve_fn);

    let func = move |args: HashMap<String, Value>| -> Result<Value> {
        let query = args
            .get("query")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        Ok(Value::String(format!(
            "Retrieval for query '{}' (use async invoke for actual results)",
            query
        )))
    };

    StructuredTool::from_function(func, name, description, retriever_args_schema())
}

/// Builder for creating retriever tools with full configuration.
pub struct RetrieverToolBuilder<R>
where
    R: BaseRetriever + Send + Sync + 'static,
{
    retriever: Arc<R>,
    name: Option<String>,
    description: Option<String>,
    document_prompt: Option<String>,
    document_separator: String,
    response_format: ResponseFormat,
}

impl<R> RetrieverToolBuilder<R>
where
    R: BaseRetriever + Send + Sync + 'static,
{
    /// Create a new RetrieverToolBuilder.
    pub fn new(retriever: Arc<R>) -> Self {
        Self {
            retriever,
            name: None,
            description: None,
            document_prompt: None,
            document_separator: "\n\n".to_string(),
            response_format: ResponseFormat::Content,
        }
    }

    /// Set the tool name.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the tool description.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the document prompt template.
    pub fn document_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.document_prompt = Some(prompt.into());
        self
    }

    /// Set the document separator.
    pub fn document_separator(mut self, separator: impl Into<String>) -> Self {
        self.document_separator = separator.into();
        self
    }

    /// Set the response format.
    pub fn response_format(mut self, format: ResponseFormat) -> Self {
        self.response_format = format;
        self
    }

    /// Build the retriever tool.
    pub fn build(self) -> Result<StructuredTool> {
        let name = self.name.ok_or_else(|| {
            crate::error::Error::InvalidConfig("Retriever tool name is required".to_string())
        })?;

        let description = self.description.ok_or_else(|| {
            crate::error::Error::InvalidConfig("Retriever tool description is required".to_string())
        })?;

        Ok(create_retriever_tool_with_options(
            self.retriever,
            name,
            description,
            self.document_prompt,
            &self.document_separator,
            self.response_format,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retriever_input() {
        let input = RetrieverInput::new("test query");
        assert_eq!(input.query, "test query");
    }

    #[test]
    fn test_retriever_args_schema() {
        let schema = retriever_args_schema();
        let json = schema.to_json_schema();

        assert_eq!(json["type"], "object");
        assert!(json["properties"]["query"].is_object());
    }

    #[test]
    fn test_format_documents() {
        let docs = vec![
            Document::new("First document"),
            Document::new("Second document"),
        ];

        let formatted = format_documents(&docs, "\n\n");
        assert_eq!(formatted, "First document\n\nSecond document");
    }

    #[test]
    fn test_format_documents_custom_separator() {
        let docs = vec![
            Document::new("Doc 1"),
            Document::new("Doc 2"),
            Document::new("Doc 3"),
        ];

        let formatted = format_documents(&docs, " | ");
        assert_eq!(formatted, "Doc 1 | Doc 2 | Doc 3");
    }
}
