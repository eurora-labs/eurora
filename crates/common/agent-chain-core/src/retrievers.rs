//! **Retriever** trait returns Documents given a text **query**.
//!
//! It is more general than a vector store. A retriever does not need to be able to
//! store documents, only to return (or retrieve) them. Vector stores can be used as
//! the backbone of a retriever, but there are other types of retrievers as well.

use std::collections::HashMap;
use std::fmt::Debug;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::callbacks::{
    AsyncCallbackManager, AsyncCallbackManagerForRetrieverRun, CallbackManager,
    CallbackManagerForRetrieverRun,
};
use crate::documents::Document;
use crate::error::Result;
use crate::runnables::{RunnableConfig, ensure_config};

/// Type alias for retriever input (a query string).
pub type RetrieverInput = String;

/// Type alias for retriever output (a list of documents).
pub type RetrieverOutput = Vec<Document>;

/// LangSmith parameters for tracing.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct LangSmithRetrieverParams {
    /// Retriever name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ls_retriever_name: Option<String>,

    /// Vector store provider.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ls_vector_store_provider: Option<String>,

    /// Embedding provider.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ls_embedding_provider: Option<String>,

    /// Embedding model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ls_embedding_model: Option<String>,
}

impl LangSmithRetrieverParams {
    /// Convert to a HashMap for use in metadata.
    pub fn to_metadata(&self) -> HashMap<String, Value> {
        let mut metadata = HashMap::new();
        if let Some(ref name) = self.ls_retriever_name {
            metadata.insert("ls_retriever_name".to_string(), Value::String(name.clone()));
        }
        if let Some(ref provider) = self.ls_vector_store_provider {
            metadata.insert(
                "ls_vector_store_provider".to_string(),
                Value::String(provider.clone()),
            );
        }
        if let Some(ref provider) = self.ls_embedding_provider {
            metadata.insert(
                "ls_embedding_provider".to_string(),
                Value::String(provider.clone()),
            );
        }
        if let Some(ref model) = self.ls_embedding_model {
            metadata.insert(
                "ls_embedding_model".to_string(),
                Value::String(model.clone()),
            );
        }
        metadata
    }
}

/// Abstract base trait for a document retrieval system.
///
/// A retrieval system is defined as something that can take string queries and return
/// the most 'relevant' documents from some source.
///
/// # Usage
///
/// A retriever follows the standard Runnable interface, and should be used via the
/// standard Runnable methods of `invoke`, `ainvoke`.
///
/// # Implementation
///
/// When implementing a custom retriever, the struct should implement the
/// `get_relevant_documents` method to define the logic for retrieving documents.
///
/// Optionally, an async native implementation can be provided by overriding the
/// `aget_relevant_documents` method.
#[async_trait]
pub trait BaseRetriever: Send + Sync + Debug {
    /// Get the name of this retriever.
    fn get_name(&self) -> String {
        let type_name = std::any::type_name::<Self>();
        type_name
            .rsplit("::")
            .next()
            .unwrap_or(type_name)
            .to_string()
    }

    /// Optional list of tags associated with the retriever.
    fn tags(&self) -> Option<&[String]> {
        None
    }

    /// Optional metadata associated with the retriever.
    fn metadata(&self) -> Option<&HashMap<String, Value>> {
        None
    }

    /// Get standard params for tracing.
    fn get_ls_params(&self) -> LangSmithRetrieverParams {
        let name = self.get_name();
        let default_name = if let Some(stripped) = name.strip_prefix("Retriever") {
            stripped.to_lowercase()
        } else if let Some(stripped) = name.strip_suffix("Retriever") {
            stripped.to_lowercase()
        } else {
            name.to_lowercase()
        };

        LangSmithRetrieverParams {
            ls_retriever_name: Some(default_name),
            ..Default::default()
        }
    }

    /// Get documents relevant to a query.
    ///
    /// This is the main method that retriever implementations should override.
    fn get_relevant_documents(
        &self,
        query: &str,
        run_manager: Option<&CallbackManagerForRetrieverRun>,
    ) -> Result<Vec<Document>>;

    /// Asynchronously get documents relevant to a query.
    ///
    /// The default implementation runs the sync version.
    async fn aget_relevant_documents(
        &self,
        query: &str,
        run_manager: Option<&AsyncCallbackManagerForRetrieverRun>,
    ) -> Result<Vec<Document>> {
        let sync_run_manager = run_manager.map(|rm| rm.get_sync());
        self.get_relevant_documents(query, sync_run_manager.as_ref())
    }

    /// Invoke the retriever to get relevant documents.
    ///
    /// Main entry point for synchronous retriever invocations.
    fn invoke(&self, input: &str, config: Option<RunnableConfig>) -> Result<Vec<Document>> {
        let config = ensure_config(config);

        let mut inheritable_metadata = config.metadata.clone();
        inheritable_metadata.extend(self.get_ls_params().to_metadata());

        let callback_manager = CallbackManager::configure(
            config.callbacks.clone(),
            None,
            Some(config.tags.clone()),
            self.tags().map(|t| t.to_vec()),
            Some(inheritable_metadata),
            self.metadata().cloned(),
        );

        let run_manager = callback_manager
            .on_retriever_start()
            .serialized(&HashMap::new())
            .query(input)
            .maybe_run_id(config.run_id)
            .name(&config.run_name.clone().unwrap_or_else(|| self.get_name()))
            .call();

        match self.get_relevant_documents(input, Some(&run_manager)) {
            Ok(result) => {
                run_manager.on_retriever_end(
                    &result
                        .iter()
                        .filter_map(|doc| serde_json::to_value(doc).ok())
                        .collect::<Vec<_>>(),
                );
                Ok(result)
            }
            Err(e) => {
                run_manager.on_retriever_error(&e);
                Err(e)
            }
        }
    }

    /// Asynchronously invoke the retriever to get relevant documents.
    ///
    /// Main entry point for asynchronous retriever invocations.
    async fn ainvoke(&self, input: &str, config: Option<RunnableConfig>) -> Result<Vec<Document>> {
        let config = ensure_config(config);

        let mut inheritable_metadata = config.metadata.clone();
        inheritable_metadata.extend(self.get_ls_params().to_metadata());

        let callback_manager = AsyncCallbackManager::configure(
            config.callbacks.clone(),
            None,
            Some(config.tags.clone()),
            self.tags().map(|t| t.to_vec()),
            Some(inheritable_metadata),
            self.metadata().cloned(),
        );

        let run_manager = callback_manager
            .on_retriever_start()
            .serialized(&HashMap::new())
            .query(input)
            .maybe_run_id(config.run_id)
            .name(&config.run_name.clone().unwrap_or_else(|| self.get_name()))
            .call()
            .await;

        let result = self
            .aget_relevant_documents(input, Some(&run_manager))
            .await;

        match &result {
            Ok(docs) => {
                run_manager
                    .on_retriever_end(
                        &docs
                            .iter()
                            .filter_map(|doc| serde_json::to_value(doc).ok())
                            .collect::<Vec<_>>(),
                    )
                    .await;
            }
            Err(e) => {
                run_manager.get_sync().on_retriever_error(e);
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct TestRetriever {
        docs: Vec<Document>,
        k: usize,
    }

    #[async_trait]
    impl BaseRetriever for TestRetriever {
        fn get_relevant_documents(
            &self,
            _query: &str,
            _run_manager: Option<&CallbackManagerForRetrieverRun>,
        ) -> Result<Vec<Document>> {
            Ok(self.docs.iter().take(self.k).cloned().collect())
        }
    }

    #[test]
    fn test_retriever_get_relevant_documents() {
        let docs = vec![
            Document::new("Hello world"),
            Document::new("Goodbye world"),
            Document::new("Hello again"),
        ];

        let retriever = TestRetriever {
            docs: docs.clone(),
            k: 2,
        };

        let result = retriever.get_relevant_documents("test", None).unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].page_content, "Hello world");
        assert_eq!(result[1].page_content, "Goodbye world");
    }

    #[test]
    fn test_retriever_invoke() {
        let docs = vec![Document::new("Hello world"), Document::new("Goodbye world")];

        let retriever = TestRetriever { docs, k: 5 };

        let result = retriever.invoke("test query", None).unwrap();

        assert_eq!(result.len(), 2);
    }

    #[tokio::test]
    async fn test_retriever_ainvoke() {
        let docs = vec![Document::new("Hello world"), Document::new("Goodbye world")];

        let retriever = TestRetriever { docs, k: 5 };

        let result = retriever.ainvoke("test query", None).await.unwrap();

        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_get_ls_params() {
        #[derive(Debug)]
        struct MyTestRetriever;

        #[async_trait]
        impl BaseRetriever for MyTestRetriever {
            fn get_relevant_documents(
                &self,
                _query: &str,
                _run_manager: Option<&CallbackManagerForRetrieverRun>,
            ) -> Result<Vec<Document>> {
                Ok(vec![])
            }
        }

        let retriever = MyTestRetriever;
        let params = retriever.get_ls_params();

        assert!(params.ls_retriever_name.is_some());
    }
}
