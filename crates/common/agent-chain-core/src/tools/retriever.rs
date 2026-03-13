use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::documents::Document;
use crate::error::Result;
use crate::retrievers::BaseRetriever;

use super::base::{ArgsSchema, ResponseFormat};
use super::structured::StructuredTool;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrieverInput {
    pub query: String,
}

impl RetrieverInput {
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
        }
    }
}

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

pub struct RetrieverTool;

#[bon::bon]
impl RetrieverTool {
    #[builder]
    pub fn create<R>(
        retriever: Arc<R>,
        name: impl Into<String>,
        description: impl Into<String>,
        document_prompt: Option<String>,
        #[builder(default = "\n\n".to_string())] document_separator: String,
        #[builder(default)] response_format: ResponseFormat,
    ) -> StructuredTool
    where
        R: BaseRetriever + Send + Sync + 'static,
    {
        let coroutine = move |args: HashMap<String, Value>| -> Pin<
            Box<dyn Future<Output = Result<Value>> + Send>,
        > {
            let query = args
                .get("query")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let retriever = retriever.clone();
            let document_separator = document_separator.clone();
            let document_prompt = document_prompt.clone();
            Box::pin(async move {
                let docs = retriever.get_relevant_documents(&query, None).await?;
                let content =
                    format_documents(&docs, &document_separator, document_prompt.as_deref());

                match response_format {
                    ResponseFormat::Content => Ok(Value::String(content)),
                    ResponseFormat::ContentAndArtifact => {
                        let docs_json: Vec<Value> = docs
                            .iter()
                            .map(|d| {
                                serde_json::json!({
                                    "page_content": d.page_content(),
                                    "metadata": d.metadata()
                                })
                            })
                            .collect();
                        Ok(serde_json::json!([content, docs_json]))
                    }
                }
            })
        };

        StructuredTool::builder()
            .name(name)
            .description(description)
            .args_schema(retriever_args_schema())
            .coroutine(Arc::new(coroutine))
            .response_format(response_format)
            .build()
    }

    #[builder]
    pub fn create_async<R, F, Fut>(
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
        let coroutine = move |args: HashMap<String, Value>| -> Pin<
            Box<dyn Future<Output = Result<Value>> + Send>,
        > {
            let query = args
                .get("query")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let retriever = retriever.clone();
            let fut = retrieve_fn(retriever, query);
            Box::pin(async move {
                let docs = fut.await?;
                let content = format_documents(&docs, "\n\n", None);
                Ok(Value::String(content))
            })
        };

        StructuredTool::builder()
            .name(name)
            .description(description)
            .args_schema(retriever_args_schema())
            .coroutine(Arc::new(coroutine))
            .build()
    }
}

fn format_documents(docs: &[Document], separator: &str, prompt: Option<&str>) -> String {
    docs.iter()
        .map(|doc| match prompt {
            Some(template) => template.replace("{page_content}", doc.page_content()),
            None => doc.page_content().to_string(),
        })
        .collect::<Vec<_>>()
        .join(separator)
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
            Document::builder().page_content("First document").build(),
            Document::builder().page_content("Second document").build(),
        ];

        let formatted = format_documents(&docs, "\n\n", None);
        assert_eq!(formatted, "First document\n\nSecond document");
    }

    #[test]
    fn test_format_documents_custom_separator() {
        let docs = vec![
            Document::builder().page_content("Doc 1").build(),
            Document::builder().page_content("Doc 2").build(),
            Document::builder().page_content("Doc 3").build(),
        ];

        let formatted = format_documents(&docs, " | ", None);
        assert_eq!(formatted, "Doc 1 | Doc 2 | Doc 3");
    }

    #[test]
    fn test_format_documents_with_prompt() {
        let docs = vec![
            Document::builder().page_content("Hello").build(),
            Document::builder().page_content("World").build(),
        ];

        let formatted = format_documents(&docs, "\n", Some("Content: {page_content}"));
        assert_eq!(formatted, "Content: Hello\nContent: World");
    }
}
