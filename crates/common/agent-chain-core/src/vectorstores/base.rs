use std::collections::HashMap;

use async_trait::async_trait;
use serde_json::Value;

use crate::Result;
use crate::documents::Document;
use crate::embeddings::Embeddings;

/// Search type for vector store retrieval.
#[derive(Debug, Clone, PartialEq)]
pub enum SearchType {
    Similarity,
    SimilarityScoreThreshold,
    Mmr,
}

impl SearchType {
    pub fn as_str(&self) -> &str {
        match self {
            SearchType::Similarity => "similarity",
            SearchType::SimilarityScoreThreshold => "similarity_score_threshold",
            SearchType::Mmr => "mmr",
        }
    }
}

impl std::fmt::Display for SearchType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Interface for vector store.
#[async_trait]
pub trait VectorStore: Send + Sync {
    /// Add texts to the vector store.
    ///
    /// Returns list of IDs from adding the texts.
    fn add_texts(
        &self,
        texts: Vec<String>,
        metadatas: Option<Vec<HashMap<String, Value>>>,
        ids: Option<Vec<String>>,
    ) -> Result<Vec<String>> {
        let metadatas_iter: Box<dyn Iterator<Item = HashMap<String, Value>>> =
            if let Some(ref metas) = metadatas {
                Box::new(metas.iter().cloned())
            } else {
                Box::new(std::iter::repeat_with(HashMap::new))
            };

        let ids_iter: Box<dyn Iterator<Item = Option<String>>> = if let Some(ref ids) = ids {
            Box::new(ids.iter().map(|id| Some(id.clone())))
        } else {
            Box::new(std::iter::repeat_with(|| None))
        };

        let documents: Vec<Document> = texts
            .into_iter()
            .zip(metadatas_iter)
            .zip(ids_iter)
            .map(|((text, metadata), id)| {
                let mut doc = Document::new(text);
                doc.metadata = metadata;
                doc.id = id;
                doc
            })
            .collect();

        self.add_documents(documents, None)
    }

    /// Add documents to the vector store.
    ///
    /// Returns list of IDs from adding the documents.
    fn add_documents(
        &self,
        documents: Vec<Document>,
        ids: Option<Vec<String>>,
    ) -> Result<Vec<String>>;

    /// Async add documents to the vector store.
    async fn aadd_documents(
        &self,
        documents: Vec<Document>,
        ids: Option<Vec<String>>,
    ) -> Result<Vec<String>> {
        self.add_documents(documents, ids)
    }

    /// Access the query embedding object if available.
    fn embeddings(&self) -> Option<&dyn Embeddings> {
        None
    }

    /// Delete by vector ID or other criteria.
    fn delete(&self, ids: Option<Vec<String>>) -> Result<()>;

    /// Async delete by vector ID or other criteria.
    async fn adelete(&self, ids: Option<Vec<String>>) -> Result<()> {
        self.delete(ids)
    }

    /// Get documents by their IDs.
    fn get_by_ids(&self, ids: &[String]) -> Result<Vec<Document>>;

    /// Async get documents by their IDs.
    async fn aget_by_ids(&self, ids: &[String]) -> Result<Vec<Document>> {
        self.get_by_ids(ids)
    }

    /// Return docs most similar to query.
    fn similarity_search(
        &self,
        query: &str,
        k: usize,
        filter: Option<&dyn Fn(&Document) -> bool>,
    ) -> Result<Vec<Document>>;

    /// Return docs most similar to embedding vector.
    fn similarity_search_by_vector(
        &self,
        embedding: &[f32],
        k: usize,
        filter: Option<&dyn Fn(&Document) -> bool>,
    ) -> Result<Vec<Document>>;

    /// Run similarity search with distance.
    fn similarity_search_with_score(
        &self,
        query: &str,
        k: usize,
        filter: Option<&dyn Fn(&Document) -> bool>,
    ) -> Result<Vec<(Document, f32)>>;

    /// Return docs selected using the maximal marginal relevance.
    fn max_marginal_relevance_search(
        &self,
        query: &str,
        k: usize,
        fetch_k: usize,
        lambda_mult: f32,
        filter: Option<&dyn Fn(&Document) -> bool>,
    ) -> Result<Vec<Document>>;

    /// Return docs selected using MMR by vector.
    fn max_marginal_relevance_search_by_vector(
        &self,
        embedding: &[f32],
        k: usize,
        fetch_k: usize,
        lambda_mult: f32,
        filter: Option<&dyn Fn(&Document) -> bool>,
    ) -> Result<Vec<Document>>;

    /// Return docs most similar to query using a specified search type.
    fn search(&self, query: &str, search_type: &SearchType, k: usize) -> Result<Vec<Document>> {
        match search_type {
            SearchType::Similarity => self.similarity_search(query, k, None),
            SearchType::SimilarityScoreThreshold => {
                let docs_and_scores = self.similarity_search_with_score(query, k, None)?;
                Ok(docs_and_scores.into_iter().map(|(doc, _)| doc).collect())
            }
            SearchType::Mmr => self.max_marginal_relevance_search(query, k, 20, 0.5, None),
        }
    }

    /// Euclidean relevance score on a scale [0, 1].
    fn euclidean_relevance_score(distance: f32) -> f32
    where
        Self: Sized,
    {
        1.0 - distance / std::f32::consts::SQRT_2
    }

    /// Cosine relevance score on a scale [0, 1].
    fn cosine_relevance_score(distance: f32) -> f32
    where
        Self: Sized,
    {
        1.0 - distance
    }

    /// Max inner product relevance score on a scale [0, 1].
    fn max_inner_product_relevance_score(distance: f32) -> f32
    where
        Self: Sized,
    {
        if distance > 0.0 {
            1.0 - distance
        } else {
            -1.0 * distance
        }
    }
}

/// Configuration for a VectorStoreRetriever.
pub struct VectorStoreRetrieverConfig {
    pub search_type: SearchType,
    pub k: usize,
    pub fetch_k: usize,
    pub lambda_mult: f32,
    pub score_threshold: Option<f32>,
}

impl Default for VectorStoreRetrieverConfig {
    fn default() -> Self {
        Self {
            search_type: SearchType::Similarity,
            k: 4,
            fetch_k: 20,
            lambda_mult: 0.5,
            score_threshold: None,
        }
    }
}
