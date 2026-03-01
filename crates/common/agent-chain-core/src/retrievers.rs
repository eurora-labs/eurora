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

pub type RetrieverInput = String;

pub type RetrieverOutput = Vec<Document>;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct LangSmithRetrieverParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ls_retriever_name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub ls_vector_store_provider: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub ls_embedding_provider: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub ls_embedding_model: Option<String>,
}

impl LangSmithRetrieverParams {
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

#[async_trait]
pub trait BaseRetriever: Send + Sync + Debug {
    fn get_name(&self) -> String {
        let type_name = std::any::type_name::<Self>();
        type_name
            .rsplit("::")
            .next()
            .unwrap_or(type_name)
            .to_string()
    }

    fn tags(&self) -> Option<&[String]> {
        None
    }

    fn metadata(&self) -> Option<&HashMap<String, Value>> {
        None
    }

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

    fn get_relevant_documents(
        &self,
        query: &str,
        run_manager: Option<&CallbackManagerForRetrieverRun>,
    ) -> Result<Vec<Document>>;

    async fn aget_relevant_documents(
        &self,
        query: &str,
        run_manager: Option<&AsyncCallbackManagerForRetrieverRun>,
    ) -> Result<Vec<Document>> {
        let sync_run_manager = run_manager.map(|rm| rm.get_sync());
        self.get_relevant_documents(query, sync_run_manager.as_ref())
    }

    fn invoke(&self, input: &str, config: Option<RunnableConfig>) -> Result<Vec<Document>> {
        let config = ensure_config(config);

        let mut inheritable_metadata = config.metadata.clone();
        inheritable_metadata.extend(self.get_ls_params().to_metadata());

        let callback_manager = CallbackManager::configure(
            config.callbacks.clone(),
            None,
            false,
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

    async fn ainvoke(&self, input: &str, config: Option<RunnableConfig>) -> Result<Vec<Document>> {
        let config = ensure_config(config);

        let mut inheritable_metadata = config.metadata.clone();
        inheritable_metadata.extend(self.get_ls_params().to_metadata());

        let callback_manager = AsyncCallbackManager::configure(
            config.callbacks.clone(),
            None,
            false,
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
            Document::builder().page_content("Hello world").build(),
            Document::builder().page_content("Goodbye world").build(),
            Document::builder().page_content("Hello again").build(),
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
        let docs = vec![
            Document::builder().page_content("Hello world").build(),
            Document::builder().page_content("Goodbye world").build(),
        ];

        let retriever = TestRetriever { docs, k: 5 };

        let result = retriever.invoke("test query", None).unwrap();

        assert_eq!(result.len(), 2);
    }

    #[tokio::test]
    async fn test_retriever_ainvoke() {
        let docs = vec![
            Document::builder().page_content("Hello world").build(),
            Document::builder().page_content("Goodbye world").build(),
        ];

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
