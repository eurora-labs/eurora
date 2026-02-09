//! **Retriever** trait returns Documents given a text **query**.
//!
//! It is more general than a vector store. A retriever does not need to be able to
//! store documents, only to return (or retrieve) them. Vector stores can be used as
//! the backbone of a retriever, but there are other types of retrievers as well.
//!
//! # Example
//!
//! ```ignore
//! use agent_chain_core::retrievers::BaseRetriever;
//! use agent_chain_core::documents::Document;
//! use agent_chain_core::callbacks::CallbackManagerForRetrieverRun;
//! use agent_chain_core::error::Result;
//! use async_trait::async_trait;
//!
//! struct SimpleRetriever {
//!     docs: Vec<Document>,
//!     k: usize,
//! }
//!
//! #[async_trait]
//! impl BaseRetriever for SimpleRetriever {
//!     fn get_relevant_documents(
//!         &self,
//!         query: &str,
//!         _run_manager: Option<&CallbackManagerForRetrieverRun>,
//!     ) -> Result<Vec<Document>> {
//!         Ok(self.docs.iter().take(self.k).cloned().collect())
//!     }
//! }
//! ```

use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;

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
    /// Create new LangSmithRetrieverParams with the given retriever name.
    pub fn new(retriever_name: impl Into<String>) -> Self {
        Self {
            ls_retriever_name: Some(retriever_name.into()),
            ..Default::default()
        }
    }

    /// Set the vector store provider.
    pub fn with_vector_store_provider(mut self, provider: impl Into<String>) -> Self {
        self.ls_vector_store_provider = Some(provider.into());
        self
    }

    /// Set the embedding provider.
    pub fn with_embedding_provider(mut self, provider: impl Into<String>) -> Self {
        self.ls_embedding_provider = Some(provider.into());
        self
    }

    /// Set the embedding model.
    pub fn with_embedding_model(mut self, model: impl Into<String>) -> Self {
        self.ls_embedding_model = Some(model.into());
        self
    }

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
/// A retriever follows the standard `Runnable` interface, and should be used via the
/// standard `Runnable` methods of `invoke`, `ainvoke`, `batch`, `abatch`.
///
/// # Implementation
///
/// When implementing a custom retriever, the struct should implement the
/// [`get_relevant_documents`][Self::get_relevant_documents] method to define the logic
/// for retrieving documents.
///
/// Optionally, an async native implementation can be provided by overriding the
/// [`aget_relevant_documents`][Self::aget_relevant_documents] method.
///
/// # Example
///
/// ```ignore
/// use agent_chain_core::retrievers::BaseRetriever;
/// use agent_chain_core::documents::Document;
/// use agent_chain_core::callbacks::CallbackManagerForRetrieverRun;
/// use agent_chain_core::error::Result;
/// use async_trait::async_trait;
///
/// struct SimpleRetriever {
///     docs: Vec<Document>,
///     k: usize,
/// }
///
/// #[async_trait]
/// impl BaseRetriever for SimpleRetriever {
///     fn get_relevant_documents(
///         &self,
///         query: &str,
///         _run_manager: Option<&CallbackManagerForRetrieverRun>,
///     ) -> Result<Vec<Document>> {
///         // Return the first k documents from the list of documents
///         Ok(self.docs.iter().take(self.k).cloned().collect())
///     }
///
///     // Optionally provide async native implementation
///     async fn aget_relevant_documents(
///         &self,
///         query: &str,
///         _run_manager: Option<&AsyncCallbackManagerForRetrieverRun>,
///     ) -> Result<Vec<Document>> {
///         Ok(self.docs.iter().take(self.k).cloned().collect())
///     }
/// }
/// ```
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
    ///
    /// These tags will be associated with each call to this retriever,
    /// and passed as arguments to the handlers defined in `callbacks`.
    fn tags(&self) -> Option<&[String]> {
        None
    }

    /// Optional metadata associated with the retriever.
    ///
    /// This metadata will be associated with each call to this retriever,
    /// and passed as arguments to the handlers defined in `callbacks`.
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

        LangSmithRetrieverParams::new(default_name)
    }

    /// Get documents relevant to a query.
    ///
    /// This is the main method that retriever implementations should override.
    ///
    /// # Arguments
    ///
    /// * `query` - String to find relevant documents for.
    /// * `run_manager` - Optional callback handler to use.
    ///
    /// # Returns
    ///
    /// List of relevant documents.
    fn get_relevant_documents(
        &self,
        query: &str,
        run_manager: Option<&CallbackManagerForRetrieverRun>,
    ) -> Result<Vec<Document>>;

    /// Asynchronously get documents relevant to a query.
    ///
    /// The default implementation runs the sync version.
    ///
    /// # Arguments
    ///
    /// * `query` - String to find relevant documents for.
    /// * `run_manager` - Optional async callback handler to use.
    ///
    /// # Returns
    ///
    /// List of relevant documents.
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
    ///
    /// # Arguments
    ///
    /// * `input` - The query string.
    /// * `config` - Optional configuration for the retriever.
    ///
    /// # Returns
    ///
    /// List of relevant documents.
    fn invoke(&self, input: &str, config: Option<RunnableConfig>) -> Result<Vec<Document>> {
        let config = ensure_config(config);

        // Build inheritable metadata
        let mut inheritable_metadata = config.metadata.clone();
        inheritable_metadata.extend(self.get_ls_params().to_metadata());

        // Configure callback manager
        let callback_manager = CallbackManager::configure(
            config.callbacks.clone(),
            None,
            Some(config.tags.clone()),
            self.tags().map(|t| t.to_vec()),
            Some(inheritable_metadata),
            self.metadata().cloned(),
        );

        // Start retriever run
        let run_manager =
            callback_manager.on_retriever_start(&HashMap::new(), input, config.run_id);

        // Get the run name
        let _run_name = config.run_name.clone().unwrap_or_else(|| self.get_name());

        // Execute retrieval
        match self.get_relevant_documents(input, Some(&run_manager)) {
            Ok(result) => {
                // Convert documents to JSON values for callback
                let docs_json: Vec<Value> = result
                    .iter()
                    .map(|doc| serde_json::to_value(doc).unwrap_or(Value::Null))
                    .collect();
                run_manager.on_retriever_end(&docs_json);
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
    ///
    /// # Arguments
    ///
    /// * `input` - The query string.
    /// * `config` - Optional configuration for the retriever.
    ///
    /// # Returns
    ///
    /// List of relevant documents.
    async fn ainvoke(&self, input: &str, config: Option<RunnableConfig>) -> Result<Vec<Document>> {
        let config = ensure_config(config);

        // Build inheritable metadata
        let mut inheritable_metadata = config.metadata.clone();
        inheritable_metadata.extend(self.get_ls_params().to_metadata());

        // Configure callback manager
        let callback_manager = AsyncCallbackManager::configure(
            config.callbacks.clone(),
            None,
            Some(config.tags.clone()),
            self.tags().map(|t| t.to_vec()),
            Some(inheritable_metadata),
            self.metadata().cloned(),
        );

        // Start retriever run
        let run_manager = callback_manager
            .on_retriever_start(&HashMap::new(), input, config.run_id)
            .await;

        // Execute retrieval
        let result = self
            .aget_relevant_documents(input, Some(&run_manager))
            .await;

        match &result {
            Ok(docs) => {
                // Convert documents to JSON values for callback
                let docs_json: Vec<Value> = docs
                    .iter()
                    .map(|doc| serde_json::to_value(doc).unwrap_or(Value::Null))
                    .collect();
                run_manager.on_retriever_end(&docs_json).await;
            }
            Err(e) => {
                // Use sync version for error handling to avoid Send issues
                run_manager.get_sync().on_retriever_error(e);
            }
        }

        result
    }

    /// Transform multiple inputs into outputs in parallel.
    ///
    /// # Arguments
    ///
    /// * `inputs` - List of query strings.
    /// * `config` - Optional configuration for the retriever.
    ///
    /// # Returns
    ///
    /// List of results, one for each input.
    fn batch(
        &self,
        inputs: Vec<&str>,
        config: Option<RunnableConfig>,
    ) -> Vec<Result<Vec<Document>>> {
        let config = ensure_config(config);
        inputs
            .into_iter()
            .map(|input| self.invoke(input, Some(config.clone())))
            .collect()
    }

    /// Asynchronously transform multiple inputs into outputs.
    ///
    /// # Arguments
    ///
    /// * `inputs` - List of query strings.
    /// * `config` - Optional configuration for the retriever.
    ///
    /// # Returns
    ///
    /// List of results, one for each input.
    async fn abatch(
        &self,
        inputs: Vec<&str>,
        config: Option<RunnableConfig>,
    ) -> Vec<Result<Vec<Document>>> {
        let config = ensure_config(config);
        let mut results = Vec::with_capacity(inputs.len());
        for input in inputs {
            results.push(self.ainvoke(input, Some(config.clone())).await);
        }
        results
    }
}

/// A type-erased retriever that can be stored in collections.
pub type DynRetriever = Arc<dyn BaseRetriever>;

/// Convert any retriever into a DynRetriever.
pub fn to_dyn<R>(retriever: R) -> DynRetriever
where
    R: BaseRetriever + 'static,
{
    Arc::new(retriever)
}

/// A simple retriever that returns documents from a static list.
///
/// This is useful for testing or for simple use cases where documents
/// are known ahead of time.
#[derive(Debug, Clone)]
pub struct SimpleRetriever {
    /// The list of documents to return.
    pub docs: Vec<Document>,
    /// The maximum number of documents to return.
    pub k: usize,
    /// Optional tags for this retriever.
    tags: Option<Vec<String>>,
    /// Optional metadata for this retriever.
    metadata: Option<HashMap<String, Value>>,
}

impl SimpleRetriever {
    /// Create a new SimpleRetriever with the given documents.
    pub fn new(docs: Vec<Document>) -> Self {
        Self {
            docs,
            k: 5,
            tags: None,
            metadata: None,
        }
    }

    /// Set the maximum number of documents to return.
    pub fn with_k(mut self, k: usize) -> Self {
        self.k = k;
        self
    }

    /// Set the tags for this retriever.
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = Some(tags);
        self
    }

    /// Set the metadata for this retriever.
    pub fn with_metadata(mut self, metadata: HashMap<String, Value>) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

#[async_trait]
impl BaseRetriever for SimpleRetriever {
    fn tags(&self) -> Option<&[String]> {
        self.tags.as_deref()
    }

    fn metadata(&self) -> Option<&HashMap<String, Value>> {
        self.metadata.as_ref()
    }

    fn get_relevant_documents(
        &self,
        _query: &str,
        _run_manager: Option<&CallbackManagerForRetrieverRun>,
    ) -> Result<Vec<Document>> {
        Ok(self.docs.iter().take(self.k).cloned().collect())
    }
}

/// A retriever that filters documents based on a predicate function.
#[derive(Clone)]
pub struct FilterRetriever<R, F>
where
    R: BaseRetriever,
    F: Fn(&Document) -> bool + Send + Sync,
{
    /// The underlying retriever.
    pub retriever: R,
    /// The filter predicate.
    pub filter: F,
}

impl<R, F> Debug for FilterRetriever<R, F>
where
    R: BaseRetriever,
    F: Fn(&Document) -> bool + Send + Sync,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FilterRetriever")
            .field("retriever", &self.retriever)
            .finish()
    }
}

impl<R, F> FilterRetriever<R, F>
where
    R: BaseRetriever,
    F: Fn(&Document) -> bool + Send + Sync,
{
    /// Create a new FilterRetriever.
    pub fn new(retriever: R, filter: F) -> Self {
        Self { retriever, filter }
    }
}

#[async_trait]
impl<R, F> BaseRetriever for FilterRetriever<R, F>
where
    R: BaseRetriever,
    F: Fn(&Document) -> bool + Send + Sync,
{
    fn get_name(&self) -> String {
        format!("FilterRetriever<{}>", self.retriever.get_name())
    }

    fn tags(&self) -> Option<&[String]> {
        self.retriever.tags()
    }

    fn metadata(&self) -> Option<&HashMap<String, Value>> {
        self.retriever.metadata()
    }

    fn get_relevant_documents(
        &self,
        query: &str,
        run_manager: Option<&CallbackManagerForRetrieverRun>,
    ) -> Result<Vec<Document>> {
        let docs = self.retriever.get_relevant_documents(query, run_manager)?;
        Ok(docs.into_iter().filter(&self.filter).collect())
    }

    async fn aget_relevant_documents(
        &self,
        query: &str,
        run_manager: Option<&AsyncCallbackManagerForRetrieverRun>,
    ) -> Result<Vec<Document>> {
        let docs = self
            .retriever
            .aget_relevant_documents(query, run_manager)
            .await?;
        Ok(docs.into_iter().filter(&self.filter).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_retriever() {
        let docs = vec![
            Document::new("Hello world"),
            Document::new("Goodbye world"),
            Document::new("Hello again"),
        ];

        let retriever = SimpleRetriever::new(docs.clone()).with_k(2);

        let result = retriever.get_relevant_documents("test", None).unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].page_content, "Hello world");
        assert_eq!(result[1].page_content, "Goodbye world");
    }

    #[test]
    fn test_simple_retriever_invoke() {
        let docs = vec![Document::new("Hello world"), Document::new("Goodbye world")];

        let retriever = SimpleRetriever::new(docs).with_k(5);

        let result = retriever.invoke("test query", None).unwrap();

        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_filter_retriever() {
        let docs = vec![
            Document::new("Hello world"),
            Document::new("Goodbye world"),
            Document::new("Hello again"),
        ];

        let base_retriever = SimpleRetriever::new(docs);
        let filter_retriever =
            FilterRetriever::new(base_retriever, |doc| doc.page_content.contains("Hello"));

        let result = filter_retriever
            .get_relevant_documents("test", None)
            .unwrap();

        assert_eq!(result.len(), 2);
        assert!(result.iter().all(|doc| doc.page_content.contains("Hello")));
    }

    #[test]
    fn test_langsmith_params() {
        let params = LangSmithRetrieverParams::new("my_retriever")
            .with_vector_store_provider("pinecone")
            .with_embedding_provider("openai")
            .with_embedding_model("text-embedding-3-small");

        assert_eq!(params.ls_retriever_name, Some("my_retriever".to_string()));
        assert_eq!(
            params.ls_vector_store_provider,
            Some("pinecone".to_string())
        );
        assert_eq!(params.ls_embedding_provider, Some("openai".to_string()));
        assert_eq!(
            params.ls_embedding_model,
            Some("text-embedding-3-small".to_string())
        );

        let metadata = params.to_metadata();
        assert_eq!(metadata.len(), 4);
    }

    #[tokio::test]
    async fn test_simple_retriever_ainvoke() {
        let docs = vec![Document::new("Hello world"), Document::new("Goodbye world")];

        let retriever = SimpleRetriever::new(docs).with_k(5);

        let result = retriever.ainvoke("test query", None).await.unwrap();

        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_batch() {
        let docs = vec![Document::new("Hello world"), Document::new("Goodbye world")];

        let retriever = SimpleRetriever::new(docs);

        let results = retriever.batch(vec!["query1", "query2"], None);

        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| r.is_ok()));
    }

    #[test]
    fn test_get_ls_params() {
        struct TestRetriever;

        impl Debug for TestRetriever {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.debug_struct("TestRetriever").finish()
            }
        }

        #[async_trait]
        impl BaseRetriever for TestRetriever {
            fn get_relevant_documents(
                &self,
                _query: &str,
                _run_manager: Option<&CallbackManagerForRetrieverRun>,
            ) -> Result<Vec<Document>> {
                Ok(vec![])
            }
        }

        let retriever = TestRetriever;
        let params = retriever.get_ls_params();

        assert_eq!(params.ls_retriever_name, Some("test".to_string()));
    }
}
