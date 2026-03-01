use std::collections::HashMap;
use std::path::Path;
use std::sync::RwLock;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::Result;
use crate::documents::Document;
use crate::embeddings::Embeddings;
use crate::vectorstores::base::{VectorStore, VectorStoreFactory};
use crate::vectorstores::utils::{cosine_similarity, maximal_marginal_relevance};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoreEntry {
    id: String,
    vector: Vec<f32>,
    text: String,
    metadata: HashMap<String, Value>,
}

pub struct InMemoryVectorStore {
    store: RwLock<HashMap<String, StoreEntry>>,
    embedding: Box<dyn Embeddings>,
}

impl InMemoryVectorStore {
    pub fn new(embedding: Box<dyn Embeddings>) -> Self {
        Self {
            store: RwLock::new(HashMap::new()),
            embedding,
        }
    }

    pub fn store_keys(&self) -> Result<Vec<String>> {
        let store = self.lock_read()?;
        Ok(store.keys().cloned().collect())
    }

    pub fn len(&self) -> Result<usize> {
        let store = self.lock_read()?;
        Ok(store.len())
    }

    pub fn is_empty(&self) -> Result<bool> {
        let store = self.lock_read()?;
        Ok(store.is_empty())
    }

    pub fn dump(&self, path: &Path) -> Result<()> {
        let store = self.lock_read()?;

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let file = std::fs::File::create(path)?;
        let writer = std::io::BufWriter::new(file);
        serde_json::to_writer_pretty(writer, &*store)?;
        Ok(())
    }

    pub fn load(path: &Path, embedding: Box<dyn Embeddings>) -> Result<Self> {
        let file = std::fs::File::open(path)?;
        let reader = std::io::BufReader::new(file);
        let store: HashMap<String, StoreEntry> = serde_json::from_reader(reader)?;

        Ok(Self {
            store: RwLock::new(store),
            embedding,
        })
    }
    fn lock_read(&self) -> Result<std::sync::RwLockReadGuard<'_, HashMap<String, StoreEntry>>> {
        self.store
            .read()
            .map_err(|e| crate::Error::Other(format!("Failed to acquire read lock: {}", e)))
    }

    fn lock_write(&self) -> Result<std::sync::RwLockWriteGuard<'_, HashMap<String, StoreEntry>>> {
        self.store
            .write()
            .map_err(|e| crate::Error::Other(format!("Failed to acquire write lock: {}", e)))
    }

    fn similarity_search_with_score_by_vector_internal(
        &self,
        embedding: &[f32],
        k: usize,
        filter: Option<&dyn Fn(&Document) -> bool>,
    ) -> Result<Vec<(Document, f32, Vec<f32>)>> {
        let store = self.lock_read()?;

        let mut docs: Vec<&StoreEntry> = store.values().collect();

        if let Some(filter_fn) = filter {
            docs.retain(|entry| {
                let doc = Document {
                    id: Some(entry.id.clone()),
                    page_content: entry.text.clone(),
                    metadata: entry.metadata.clone(),
                    type_: "Document".to_string(),
                };
                filter_fn(&doc)
            });
        }

        if docs.is_empty() {
            return Ok(vec![]);
        }

        let doc_vectors: Vec<Vec<f32>> = docs.iter().map(|d| d.vector.clone()).collect();
        let similarity = cosine_similarity(&[embedding.to_vec()], &doc_vectors)?;
        let scores = &similarity[0];

        let mut indexed_scores: Vec<(usize, f32)> = scores.iter().copied().enumerate().collect();
        indexed_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        indexed_scores.truncate(k);

        let results = indexed_scores
            .into_iter()
            .map(|(idx, score)| {
                let entry = docs[idx];
                let doc = Document {
                    id: Some(entry.id.clone()),
                    page_content: entry.text.clone(),
                    metadata: entry.metadata.clone(),
                    type_: "Document".to_string(),
                };
                (doc, score, entry.vector.clone())
            })
            .collect();

        Ok(results)
    }

    fn add_documents_with_vectors(
        &self,
        documents: &[Document],
        vectors: Vec<Vec<f32>>,
        ids: Option<Vec<String>>,
    ) -> Result<Vec<String>> {
        if let Some(ref ids) = ids
            && ids.len() != documents.len()
        {
            return Err(crate::Error::Other(format!(
                "ids must be the same length as documents. Got {} ids and {} documents.",
                ids.len(),
                documents.len()
            )));
        }

        let mut id_iter: Box<dyn Iterator<Item = Option<String>>> = if let Some(ids) = ids {
            Box::new(ids.into_iter().map(Some))
        } else {
            Box::new(documents.iter().map(|d| d.id.clone()))
        };

        let mut store = self.lock_write()?;
        let mut result_ids = Vec::with_capacity(documents.len());

        for (doc, vector) in documents.iter().zip(vectors.into_iter()) {
            let doc_id = id_iter
                .next()
                .flatten()
                .unwrap_or_else(|| Uuid::new_v4().to_string());
            result_ids.push(doc_id.clone());
            store.insert(
                doc_id.clone(),
                StoreEntry {
                    id: doc_id,
                    vector,
                    text: doc.page_content.clone(),
                    metadata: doc.metadata.clone(),
                },
            );
        }

        Ok(result_ids)
    }
}

#[async_trait::async_trait]
impl VectorStore for InMemoryVectorStore {
    fn add_documents(
        &self,
        documents: Vec<Document>,
        ids: Option<Vec<String>>,
    ) -> Result<Vec<String>> {
        let texts: Vec<String> = documents.iter().map(|d| d.page_content.clone()).collect();
        let vectors = self.embedding.embed_documents(texts)?;
        self.add_documents_with_vectors(&documents, vectors, ids)
    }

    async fn aadd_documents(
        &self,
        documents: Vec<Document>,
        ids: Option<Vec<String>>,
    ) -> Result<Vec<String>> {
        let texts: Vec<String> = documents.iter().map(|d| d.page_content.clone()).collect();
        let vectors = self.embedding.aembed_documents(texts).await?;
        self.add_documents_with_vectors(&documents, vectors, ids)
    }

    fn embeddings(&self) -> Option<&dyn Embeddings> {
        Some(self.embedding.as_ref())
    }

    fn delete(&self, ids: Option<Vec<String>>) -> Result<()> {
        if let Some(ids) = ids {
            let mut store = self.lock_write()?;
            for id in ids {
                store.remove(&id);
            }
        }
        Ok(())
    }

    fn get_by_ids(&self, ids: &[String]) -> Result<Vec<Document>> {
        let store = self.lock_read()?;
        let mut documents = Vec::new();
        for id in ids {
            if let Some(entry) = store.get(id) {
                documents.push(Document {
                    id: Some(entry.id.clone()),
                    page_content: entry.text.clone(),
                    metadata: entry.metadata.clone(),
                    type_: "Document".to_string(),
                });
            }
        }
        Ok(documents)
    }

    fn similarity_search(
        &self,
        query: &str,
        k: usize,
        filter: Option<&dyn Fn(&Document) -> bool>,
    ) -> Result<Vec<Document>> {
        let docs_and_scores = self.similarity_search_with_score(query, k, filter)?;
        Ok(docs_and_scores.into_iter().map(|(doc, _)| doc).collect())
    }

    fn similarity_search_by_vector(
        &self,
        embedding: &[f32],
        k: usize,
        filter: Option<&dyn Fn(&Document) -> bool>,
    ) -> Result<Vec<Document>> {
        let results = self.similarity_search_with_score_by_vector_internal(embedding, k, filter)?;
        Ok(results.into_iter().map(|(doc, _, _)| doc).collect())
    }

    fn similarity_search_with_score(
        &self,
        query: &str,
        k: usize,
        filter: Option<&dyn Fn(&Document) -> bool>,
    ) -> Result<Vec<(Document, f32)>> {
        let embedding = self.embedding.embed_query(query)?;
        let results =
            self.similarity_search_with_score_by_vector_internal(&embedding, k, filter)?;
        Ok(results
            .into_iter()
            .map(|(doc, score, _)| (doc, score))
            .collect())
    }

    fn max_marginal_relevance_search(
        &self,
        query: &str,
        k: usize,
        fetch_k: usize,
        lambda_mult: f32,
        filter: Option<&dyn Fn(&Document) -> bool>,
    ) -> Result<Vec<Document>> {
        let embedding_vector = self.embedding.embed_query(query)?;
        self.max_marginal_relevance_search_by_vector(
            &embedding_vector,
            k,
            fetch_k,
            lambda_mult,
            filter,
        )
    }

    fn max_marginal_relevance_search_by_vector(
        &self,
        embedding: &[f32],
        k: usize,
        fetch_k: usize,
        lambda_mult: f32,
        filter: Option<&dyn Fn(&Document) -> bool>,
    ) -> Result<Vec<Document>> {
        let prefetch_hits =
            self.similarity_search_with_score_by_vector_internal(embedding, fetch_k, filter)?;

        if prefetch_hits.is_empty() {
            return Ok(vec![]);
        }

        let vectors: Vec<Vec<f32>> = prefetch_hits.iter().map(|(_, _, v)| v.clone()).collect();
        let mmr_indices = maximal_marginal_relevance(embedding, &vectors, lambda_mult, k)?;

        Ok(mmr_indices
            .into_iter()
            .map(|idx| prefetch_hits[idx].0.clone())
            .collect())
    }
}

impl VectorStoreFactory for InMemoryVectorStore {
    fn from_texts(
        texts: &[&str],
        embedding: Box<dyn Embeddings>,
        metadatas: Option<Vec<HashMap<String, Value>>>,
        ids: Option<Vec<String>>,
    ) -> Result<Self> {
        let store = Self::new(embedding);
        let text_strings: Vec<String> = texts.iter().map(|t| t.to_string()).collect();
        store.add_texts(text_strings, metadatas, ids)?;
        Ok(store)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embeddings::DeterministicFakeEmbedding;

    fn make_store() -> InMemoryVectorStore {
        InMemoryVectorStore::new(Box::new(DeterministicFakeEmbedding::new(10)))
    }

    #[test]
    fn test_add_and_search() {
        let store = make_store();
        let docs = vec![
            Document::builder().page_content("foo").id("1").build(),
            Document::builder().page_content("bar").id("2").build(),
            Document::builder().page_content("baz").id("3").build(),
        ];
        let ids = store.add_documents(docs, None).unwrap();
        assert_eq!(ids.len(), 3);

        let results = store.similarity_search("foo", 1, None).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].page_content, "foo");
    }

    #[test]
    fn test_delete() {
        let store = make_store();
        let docs = vec![
            Document::builder().page_content("foo").id("1").build(),
            Document::builder().page_content("bar").id("2").build(),
        ];
        store.add_documents(docs, None).unwrap();
        store.delete(Some(vec!["1".into()])).unwrap();

        let results = store.get_by_ids(&["1".into()]).unwrap();
        assert!(results.is_empty());

        let results = store.get_by_ids(&["2".into()]).unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_get_by_ids() {
        let store = make_store();
        let docs = vec![
            Document::builder().page_content("foo").id("1").build(),
            Document::builder().page_content("bar").id("2").build(),
        ];
        store.add_documents(docs, None).unwrap();

        let results = store
            .get_by_ids(&["1".into(), "2".into(), "nonexistent".into()])
            .unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_similarity_search_with_score() {
        let store = make_store();
        let docs = vec![
            Document::builder()
                .page_content("hello world")
                .id("1")
                .build(),
            Document::builder()
                .page_content("goodbye world")
                .id("2")
                .build(),
        ];
        store.add_documents(docs, None).unwrap();

        let results = store
            .similarity_search_with_score("hello world", 2, None)
            .unwrap();
        assert_eq!(results.len(), 2);
        assert!(results[0].1 >= results[1].1);
    }

    #[test]
    fn test_similarity_search_with_filter() {
        let store = make_store();
        let mut doc1 = Document::builder().page_content("foo").id("1").build();
        doc1.metadata
            .insert("category".into(), Value::String("a".into()));
        let mut doc2 = Document::builder().page_content("bar").id("2").build();
        doc2.metadata
            .insert("category".into(), Value::String("b".into()));
        store.add_documents(vec![doc1, doc2], None).unwrap();

        let filter = |doc: &Document| -> bool {
            doc.metadata.get("category").and_then(|v| v.as_str()) == Some("b")
        };
        let results = store.similarity_search("bar", 2, Some(&filter)).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].page_content, "bar");
    }

    #[test]
    fn test_mmr_search() {
        let store = make_store();
        let docs = vec![
            Document::builder().page_content("apple").id("1").build(),
            Document::builder().page_content("banana").id("2").build(),
            Document::builder().page_content("cherry").id("3").build(),
        ];
        store.add_documents(docs, None).unwrap();

        let results = store
            .max_marginal_relevance_search("apple", 2, 3, 0.5, None)
            .unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_dump_and_load() {
        let store = make_store();
        let docs = vec![
            Document::builder()
                .page_content("hello world")
                .id("1")
                .build(),
            Document::builder()
                .page_content("goodbye world")
                .id("2")
                .build(),
        ];
        store.add_documents(docs, None).unwrap();

        let temp_dir = std::env::temp_dir().join("agent_chain_test_vectorstore");
        let path = temp_dir.join("test_store.json");

        store.dump(&path).unwrap();
        assert!(path.exists());

        let loaded_store =
            InMemoryVectorStore::load(&path, Box::new(DeterministicFakeEmbedding::new(10)))
                .unwrap();
        let loaded_docs = loaded_store
            .get_by_ids(&["1".to_string(), "2".to_string()])
            .unwrap();
        assert_eq!(loaded_docs.len(), 2);

        let doc1 = loaded_docs
            .iter()
            .find(|d| d.id.as_deref() == Some("1"))
            .unwrap();
        assert_eq!(doc1.page_content, "hello world");

        let doc2 = loaded_docs
            .iter()
            .find(|d| d.id.as_deref() == Some("2"))
            .unwrap();
        assert_eq!(doc2.page_content, "goodbye world");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_add_with_explicit_ids() {
        let store = make_store();
        let docs = vec![
            Document::builder().page_content("foo").build(),
            Document::builder().page_content("bar").build(),
        ];
        let ids = store
            .add_documents(docs, Some(vec!["custom1".into(), "custom2".into()]))
            .unwrap();
        assert_eq!(ids, vec!["custom1", "custom2"]);

        let results = store.get_by_ids(&["custom1".into()]).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].page_content, "foo");
    }
}
