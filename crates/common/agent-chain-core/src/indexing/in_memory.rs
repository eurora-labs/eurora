use std::collections::HashMap;
use std::fmt;
use std::sync::RwLock;

use async_trait::async_trait;
use serde_json::Value;
use uuid::Uuid;

use crate::callbacks::manager::{
    AsyncCallbackManagerForRetrieverRun, CallbackManagerForRetrieverRun,
};
use crate::documents::Document;
use crate::error::{Error, Result};
use crate::indexing::base::{DeleteResponse, DocumentIndex, UpsertResponse};
use crate::retrievers::BaseRetriever;
use crate::retrievers::LangSmithRetrieverParams;
use crate::runnables::config::RunnableConfig;

pub struct InMemoryDocumentIndex {
    store: RwLock<HashMap<String, Document>>,
    top_k: usize,
}

impl fmt::Debug for InMemoryDocumentIndex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let count = self.store.read().map(|s| s.len()).unwrap_or(0);
        f.debug_struct("InMemoryDocumentIndex")
            .field("top_k", &self.top_k)
            .field("num_documents", &count)
            .finish()
    }
}

impl InMemoryDocumentIndex {
    pub fn new(top_k: usize) -> Self {
        Self {
            store: RwLock::new(HashMap::new()),
            top_k,
        }
    }

    pub fn len(&self) -> Result<usize> {
        let store = self
            .store
            .read()
            .map_err(|e| Error::Other(format!("Failed to acquire read lock: {e}")))?;
        Ok(store.len())
    }

    pub fn is_empty(&self) -> Result<bool> {
        Ok(self.len()? == 0)
    }
}

impl Default for InMemoryDocumentIndex {
    fn default() -> Self {
        Self::new(4)
    }
}

#[async_trait]
impl DocumentIndex for InMemoryDocumentIndex {
    fn upsert(&self, items: &[Document]) -> Result<UpsertResponse> {
        let mut store = self
            .store
            .write()
            .map_err(|e| Error::Other(format!("Failed to acquire write lock: {e}")))?;

        let mut ok_ids = Vec::with_capacity(items.len());

        for item in items {
            let (id, doc) = if item.id.is_none() {
                let id = Uuid::new_v4().to_string();
                let mut doc = item.clone();
                doc.id = Some(id.clone());
                (id, doc)
            } else {
                let id = match item.id.clone() {
                    Some(id) => id,
                    None => continue, // unreachable due to outer if
                };
                (id, item.clone())
            };

            store.insert(id.clone(), doc);
            ok_ids.push(id);
        }

        Ok(UpsertResponse {
            succeeded: ok_ids,
            failed: vec![],
        })
    }

    fn delete(&self, ids: Option<&[String]>) -> Result<DeleteResponse> {
        let ids = ids
            .ok_or_else(|| Error::InvalidConfig("IDs must be provided for deletion".to_string()))?;

        let mut store = self
            .store
            .write()
            .map_err(|e| Error::Other(format!("Failed to acquire write lock: {e}")))?;

        let mut ok_ids = Vec::new();
        for id in ids {
            if store.remove(id).is_some() {
                ok_ids.push(id.clone());
            }
        }

        let num_deleted = ok_ids.len();
        Ok(DeleteResponse {
            succeeded: Some(ok_ids),
            num_deleted: Some(num_deleted),
            num_failed: Some(0),
            failed: Some(vec![]),
        })
    }

    fn get(&self, ids: &[String]) -> Result<Vec<Document>> {
        let store = self
            .store
            .read()
            .map_err(|e| Error::Other(format!("Failed to acquire read lock: {e}")))?;

        Ok(ids.iter().filter_map(|id| store.get(id).cloned()).collect())
    }
}

#[async_trait]
impl BaseRetriever for InMemoryDocumentIndex {
    fn get_name(&self) -> String {
        "InMemoryDocumentIndex".to_string()
    }

    fn tags(&self) -> Option<&[String]> {
        None
    }

    fn metadata(&self) -> Option<&HashMap<String, Value>> {
        None
    }

    fn get_ls_params(&self) -> LangSmithRetrieverParams {
        LangSmithRetrieverParams::default()
    }

    fn get_relevant_documents(
        &self,
        query: &str,
        _run_manager: Option<&CallbackManagerForRetrieverRun>,
    ) -> Result<Vec<Document>> {
        let store = self
            .store
            .read()
            .map_err(|e| Error::Other(format!("Failed to acquire read lock: {e}")))?;

        let mut counts_by_doc: Vec<(Document, usize)> = store
            .values()
            .map(|doc| {
                let count = doc.page_content.matches(query).count();
                (doc.clone(), count)
            })
            .collect();

        counts_by_doc.sort_by(|a, b| b.1.cmp(&a.1));

        Ok(counts_by_doc
            .into_iter()
            .take(self.top_k)
            .map(|(doc, _)| doc)
            .collect())
    }

    async fn aget_relevant_documents(
        &self,
        query: &str,
        _run_manager: Option<&AsyncCallbackManagerForRetrieverRun>,
    ) -> Result<Vec<Document>> {
        self.get_relevant_documents(query, None)
    }

    fn invoke(&self, input: &str, _config: Option<RunnableConfig>) -> Result<Vec<Document>> {
        self.get_relevant_documents(input, None)
    }

    async fn ainvoke(&self, input: &str, config: Option<RunnableConfig>) -> Result<Vec<Document>> {
        self.invoke(input, config)
    }
}
