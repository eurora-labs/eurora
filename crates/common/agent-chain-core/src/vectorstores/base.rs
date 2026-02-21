use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;

use crate::Result;
use crate::callbacks::{AsyncCallbackManagerForRetrieverRun, CallbackManagerForRetrieverRun};
use crate::documents::Document;
use crate::embeddings::Embeddings;
use crate::error::Error;
use crate::retrievers::{BaseRetriever, LangSmithRetrieverParams};

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

#[async_trait]
pub trait VectorStore: Send + Sync {
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

    async fn aadd_texts(
        &self,
        texts: Vec<String>,
        metadatas: Option<Vec<HashMap<String, Value>>>,
        ids: Option<Vec<String>>,
    ) -> Result<Vec<String>> {
        self.add_texts(texts, metadatas, ids)
    }

    fn add_documents(
        &self,
        documents: Vec<Document>,
        ids: Option<Vec<String>>,
    ) -> Result<Vec<String>>;

    async fn aadd_documents(
        &self,
        documents: Vec<Document>,
        ids: Option<Vec<String>>,
    ) -> Result<Vec<String>> {
        self.add_documents(documents, ids)
    }

    fn embeddings(&self) -> Option<&dyn Embeddings> {
        None
    }

    fn delete(&self, ids: Option<Vec<String>>) -> Result<()>;

    async fn adelete(&self, ids: Option<Vec<String>>) -> Result<()> {
        self.delete(ids)
    }

    fn get_by_ids(&self, ids: &[String]) -> Result<Vec<Document>>;

    async fn aget_by_ids(&self, ids: &[String]) -> Result<Vec<Document>> {
        self.get_by_ids(ids)
    }

    fn similarity_search(
        &self,
        query: &str,
        k: usize,
        filter: Option<&dyn Fn(&Document) -> bool>,
    ) -> Result<Vec<Document>>;

    async fn asimilarity_search(&self, query: &str, k: usize) -> Result<Vec<Document>> {
        self.similarity_search(query, k, None)
    }

    fn similarity_search_by_vector(
        &self,
        embedding: &[f32],
        k: usize,
        filter: Option<&dyn Fn(&Document) -> bool>,
    ) -> Result<Vec<Document>>;

    async fn asimilarity_search_by_vector(
        &self,
        embedding: &[f32],
        k: usize,
    ) -> Result<Vec<Document>> {
        self.similarity_search_by_vector(embedding, k, None)
    }

    fn similarity_search_with_score(
        &self,
        query: &str,
        k: usize,
        filter: Option<&dyn Fn(&Document) -> bool>,
    ) -> Result<Vec<(Document, f32)>>;

    async fn asimilarity_search_with_score(
        &self,
        query: &str,
        k: usize,
    ) -> Result<Vec<(Document, f32)>> {
        self.similarity_search_with_score(query, k, None)
    }

    fn max_marginal_relevance_search(
        &self,
        query: &str,
        k: usize,
        fetch_k: usize,
        lambda_mult: f32,
        filter: Option<&dyn Fn(&Document) -> bool>,
    ) -> Result<Vec<Document>>;

    async fn amax_marginal_relevance_search(
        &self,
        query: &str,
        k: usize,
        fetch_k: usize,
        lambda_mult: f32,
    ) -> Result<Vec<Document>> {
        self.max_marginal_relevance_search(query, k, fetch_k, lambda_mult, None)
    }

    fn max_marginal_relevance_search_by_vector(
        &self,
        embedding: &[f32],
        k: usize,
        fetch_k: usize,
        lambda_mult: f32,
        filter: Option<&dyn Fn(&Document) -> bool>,
    ) -> Result<Vec<Document>>;

    async fn amax_marginal_relevance_search_by_vector(
        &self,
        embedding: &[f32],
        k: usize,
        fetch_k: usize,
        lambda_mult: f32,
    ) -> Result<Vec<Document>> {
        self.max_marginal_relevance_search_by_vector(embedding, k, fetch_k, lambda_mult, None)
    }

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

    async fn asearch(
        &self,
        query: &str,
        search_type: &SearchType,
        k: usize,
    ) -> Result<Vec<Document>> {
        self.search(query, search_type, k)
    }

    fn select_relevance_score_fn(&self) -> Option<fn(f32) -> f32> {
        None
    }

    fn similarity_search_with_relevance_scores_internal(
        &self,
        query: &str,
        k: usize,
        filter: Option<&dyn Fn(&Document) -> bool>,
    ) -> Result<Vec<(Document, f32)>> {
        let relevance_score_fn = self.select_relevance_score_fn().ok_or_else(|| {
            Error::NotImplemented(
                "select_relevance_score_fn not implemented for this vector store. \
                 The underlying vector store must define this to use \
                 similarity_search_with_relevance_scores."
                    .into(),
            )
        })?;
        let docs_and_scores = self.similarity_search_with_score(query, k, filter)?;
        Ok(docs_and_scores
            .into_iter()
            .map(|(doc, score)| (doc, relevance_score_fn(score)))
            .collect())
    }

    fn similarity_search_with_relevance_scores(
        &self,
        query: &str,
        k: usize,
        score_threshold: Option<f32>,
        filter: Option<&dyn Fn(&Document) -> bool>,
    ) -> Result<Vec<(Document, f32)>> {
        let mut docs_and_similarities =
            self.similarity_search_with_relevance_scores_internal(query, k, filter)?;

        if docs_and_similarities
            .iter()
            .any(|(_, score)| *score < 0.0 || *score > 1.0)
        {
            tracing::warn!(
                "Relevance scores must be between 0 and 1, got {:?}",
                docs_and_similarities
                    .iter()
                    .map(|(_, s)| *s)
                    .collect::<Vec<_>>()
            );
        }

        if let Some(threshold) = score_threshold {
            docs_and_similarities.retain(|(_, score)| *score >= threshold);
            if docs_and_similarities.is_empty() {
                tracing::warn!(
                    "No relevant docs were retrieved using the relevance score threshold {}",
                    threshold
                );
            }
        }

        Ok(docs_and_similarities)
    }

    async fn asimilarity_search_with_relevance_scores(
        &self,
        query: &str,
        k: usize,
        score_threshold: Option<f32>,
    ) -> Result<Vec<(Document, f32)>> {
        self.similarity_search_with_relevance_scores(query, k, score_threshold, None)
    }

    fn euclidean_relevance_score(distance: f32) -> f32
    where
        Self: Sized,
    {
        1.0 - distance / std::f32::consts::SQRT_2
    }

    fn cosine_relevance_score(distance: f32) -> f32
    where
        Self: Sized,
    {
        1.0 - distance
    }

    fn max_inner_product_relevance_score(distance: f32) -> f32
    where
        Self: Sized,
    {
        if distance > 0.0 {
            1.0 - distance
        } else {
            -distance
        }
    }
}

pub trait VectorStoreFactory: VectorStore + Sized {
    fn from_texts(
        texts: &[&str],
        embedding: Box<dyn Embeddings>,
        metadatas: Option<Vec<HashMap<String, Value>>>,
        ids: Option<Vec<String>>,
    ) -> Result<Self>;

    fn from_documents(documents: Vec<Document>, embedding: Box<dyn Embeddings>) -> Result<Self> {
        let texts: Vec<&str> = documents.iter().map(|d| d.page_content.as_str()).collect();
        let metadatas: Vec<HashMap<String, Value>> =
            documents.iter().map(|d| d.metadata.clone()).collect();
        let ids: Vec<String> = documents.iter().filter_map(|d| d.id.clone()).collect();
        let ids = if ids.is_empty() { None } else { Some(ids) };
        Self::from_texts(&texts, embedding, Some(metadatas), ids)
    }
}

pub struct VectorStoreRetrieverConfig {
    pub search_type: SearchType,
    pub search_kwargs: HashMap<String, Value>,
    pub tags: Option<Vec<String>>,
}

impl VectorStoreRetrieverConfig {
    pub fn k(&self) -> usize {
        self.search_kwargs
            .get("k")
            .and_then(|v| v.as_u64())
            .unwrap_or(4) as usize
    }

    pub fn fetch_k(&self) -> usize {
        self.search_kwargs
            .get("fetch_k")
            .and_then(|v| v.as_u64())
            .unwrap_or(20) as usize
    }

    pub fn lambda_mult(&self) -> f32 {
        self.search_kwargs
            .get("lambda_mult")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5) as f32
    }

    pub fn score_threshold(&self) -> Option<f32> {
        self.search_kwargs
            .get("score_threshold")
            .and_then(|v| v.as_f64())
            .map(|v| v as f32)
    }
}

impl Default for VectorStoreRetrieverConfig {
    fn default() -> Self {
        let mut search_kwargs = HashMap::new();
        search_kwargs.insert("k".to_string(), Value::from(4));
        Self {
            search_type: SearchType::Similarity,
            search_kwargs,
            tags: None,
        }
    }
}

fn validate_retriever_config(config: &VectorStoreRetrieverConfig) -> Result<()> {
    let allowed = ["similarity", "similarity_score_threshold", "mmr"];
    let search_type_str = config.search_type.as_str();
    if !allowed.contains(&search_type_str) {
        return Err(Error::InvalidConfig(format!(
            "search_type of {} not allowed. Valid values are: {:?}",
            search_type_str, allowed
        )));
    }
    if config.search_type == SearchType::SimilarityScoreThreshold
        && config.score_threshold().is_none()
    {
        return Err(Error::InvalidConfig(
            "`score_threshold` is not specified with a float value (0~1) \
                 in `search_kwargs`."
                .to_string(),
        ));
    }
    Ok(())
}

pub struct VectorStoreRetriever {
    pub vectorstore: Arc<dyn VectorStore>,
    pub search_type: SearchType,
    pub search_kwargs: HashMap<String, Value>,
    pub tags: Vec<String>,
}

impl Debug for VectorStoreRetriever {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VectorStoreRetriever")
            .field("search_type", &self.search_type)
            .field("search_kwargs", &self.search_kwargs)
            .field("tags", &self.tags)
            .finish()
    }
}

impl VectorStoreRetriever {
    pub fn new(
        vectorstore: Arc<dyn VectorStore>,
        config: VectorStoreRetrieverConfig,
    ) -> Result<Self> {
        validate_retriever_config(&config)?;
        let tags = config.tags.unwrap_or_default();
        Ok(Self {
            vectorstore,
            search_type: config.search_type,
            search_kwargs: config.search_kwargs,
            tags,
        })
    }

    fn k(&self) -> usize {
        self.search_kwargs
            .get("k")
            .and_then(|v| v.as_u64())
            .unwrap_or(4) as usize
    }

    fn fetch_k(&self) -> usize {
        self.search_kwargs
            .get("fetch_k")
            .and_then(|v| v.as_u64())
            .unwrap_or(20) as usize
    }

    fn lambda_mult(&self) -> f32 {
        self.search_kwargs
            .get("lambda_mult")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5) as f32
    }

    fn score_threshold(&self) -> Option<f32> {
        self.search_kwargs
            .get("score_threshold")
            .and_then(|v| v.as_f64())
            .map(|v| v as f32)
    }

    pub fn add_documents(
        &self,
        documents: Vec<Document>,
        ids: Option<Vec<String>>,
    ) -> Result<Vec<String>> {
        self.vectorstore.add_documents(documents, ids)
    }

    pub async fn aadd_documents(
        &self,
        documents: Vec<Document>,
        ids: Option<Vec<String>>,
    ) -> Result<Vec<String>> {
        self.vectorstore.aadd_documents(documents, ids).await
    }
}

#[async_trait]
impl BaseRetriever for VectorStoreRetriever {
    fn get_name(&self) -> String {
        "VectorStoreRetriever".to_string()
    }

    fn tags(&self) -> Option<&[String]> {
        if self.tags.is_empty() {
            None
        } else {
            Some(&self.tags)
        }
    }

    fn get_ls_params(&self) -> LangSmithRetrieverParams {
        LangSmithRetrieverParams {
            ls_retriever_name: Some("vectorstore".to_string()),
            ls_vector_store_provider: None,
            ls_embedding_provider: None,
            ls_embedding_model: None,
        }
    }

    fn get_relevant_documents(
        &self,
        query: &str,
        _run_manager: Option<&CallbackManagerForRetrieverRun>,
    ) -> Result<Vec<Document>> {
        let k = self.k();
        match &self.search_type {
            SearchType::Similarity => self.vectorstore.similarity_search(query, k, None),
            SearchType::SimilarityScoreThreshold => {
                let threshold = self.score_threshold();
                let docs_and_scores = self
                    .vectorstore
                    .similarity_search_with_relevance_scores(query, k, threshold, None)?;
                Ok(docs_and_scores.into_iter().map(|(doc, _)| doc).collect())
            }
            SearchType::Mmr => {
                let fetch_k = self.fetch_k();
                let lambda_mult = self.lambda_mult();
                self.vectorstore
                    .max_marginal_relevance_search(query, k, fetch_k, lambda_mult, None)
            }
        }
    }

    async fn aget_relevant_documents(
        &self,
        query: &str,
        _run_manager: Option<&AsyncCallbackManagerForRetrieverRun>,
    ) -> Result<Vec<Document>> {
        let k = self.k();
        match &self.search_type {
            SearchType::Similarity => self.vectorstore.asimilarity_search(query, k).await,
            SearchType::SimilarityScoreThreshold => {
                let threshold = self.score_threshold();
                let docs_and_scores = self
                    .vectorstore
                    .asimilarity_search_with_relevance_scores(query, k, threshold)
                    .await?;
                Ok(docs_and_scores.into_iter().map(|(doc, _)| doc).collect())
            }
            SearchType::Mmr => {
                let fetch_k = self.fetch_k();
                let lambda_mult = self.lambda_mult();
                self.vectorstore
                    .amax_marginal_relevance_search(query, k, fetch_k, lambda_mult)
                    .await
            }
        }
    }
}

pub trait VectorStoreRetrieverExt {
    fn into_retriever(self, config: VectorStoreRetrieverConfig) -> Result<VectorStoreRetriever>;
}

impl VectorStoreRetrieverExt for Arc<dyn VectorStore> {
    fn into_retriever(self, config: VectorStoreRetrieverConfig) -> Result<VectorStoreRetriever> {
        VectorStoreRetriever::new(self, config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_type_display() {
        assert_eq!(SearchType::Similarity.as_str(), "similarity");
        assert_eq!(
            SearchType::SimilarityScoreThreshold.as_str(),
            "similarity_score_threshold"
        );
        assert_eq!(SearchType::Mmr.as_str(), "mmr");
    }

    #[test]
    fn test_config_defaults() {
        let config = VectorStoreRetrieverConfig::default();
        assert_eq!(config.search_type, SearchType::Similarity);
        assert_eq!(config.k(), 4);
        assert_eq!(config.fetch_k(), 20);
        assert!((config.lambda_mult() - 0.5).abs() < f32::EPSILON);
        assert!(config.score_threshold().is_none());
    }

    #[test]
    fn test_config_with_values() {
        let mut search_kwargs = HashMap::new();
        search_kwargs.insert("k".to_string(), Value::from(10));
        search_kwargs.insert("fetch_k".to_string(), Value::from(50));
        search_kwargs.insert("lambda_mult".to_string(), Value::from(0.25));
        search_kwargs.insert("score_threshold".to_string(), Value::from(0.8));

        let config = VectorStoreRetrieverConfig {
            search_type: SearchType::SimilarityScoreThreshold,
            search_kwargs,
            tags: Some(vec!["test".to_string()]),
        };

        assert_eq!(config.k(), 10);
        assert_eq!(config.fetch_k(), 50);
        assert!((config.lambda_mult() - 0.25).abs() < f32::EPSILON);
        assert!((config.score_threshold().unwrap() - 0.8).abs() < f32::EPSILON);
    }

    #[test]
    fn test_validate_config_valid_similarity() {
        let config = VectorStoreRetrieverConfig::default();
        assert!(validate_retriever_config(&config).is_ok());
    }

    #[test]
    fn test_validate_config_threshold_without_score() {
        let config = VectorStoreRetrieverConfig {
            search_type: SearchType::SimilarityScoreThreshold,
            search_kwargs: HashMap::new(),
            tags: None,
        };
        assert!(validate_retriever_config(&config).is_err());
    }

    #[test]
    fn test_validate_config_threshold_with_score() {
        let mut search_kwargs = HashMap::new();
        search_kwargs.insert("score_threshold".to_string(), Value::from(0.8));

        let config = VectorStoreRetrieverConfig {
            search_type: SearchType::SimilarityScoreThreshold,
            search_kwargs,
            tags: None,
        };
        assert!(validate_retriever_config(&config).is_ok());
    }

    #[test]
    fn test_relevance_score_functions() {
        let score = InMemoryTestHelper::euclidean_relevance_score(0.0);
        assert!((score - 1.0).abs() < f32::EPSILON);

        let score = InMemoryTestHelper::cosine_relevance_score(0.0);
        assert!((score - 1.0).abs() < f32::EPSILON);
        let score = InMemoryTestHelper::cosine_relevance_score(1.0);
        assert!(score.abs() < f32::EPSILON);

        let score = InMemoryTestHelper::max_inner_product_relevance_score(0.5);
        assert!((score - 0.5).abs() < f32::EPSILON);
        let score = InMemoryTestHelper::max_inner_product_relevance_score(-0.5);
        assert!((score - 0.5).abs() < f32::EPSILON);
    }

    struct InMemoryTestHelper;
    #[async_trait]
    impl VectorStore for InMemoryTestHelper {
        fn add_documents(&self, _: Vec<Document>, _: Option<Vec<String>>) -> Result<Vec<String>> {
            Ok(vec![])
        }
        fn delete(&self, _: Option<Vec<String>>) -> Result<()> {
            Ok(())
        }
        fn get_by_ids(&self, _: &[String]) -> Result<Vec<Document>> {
            Ok(vec![])
        }
        fn similarity_search(
            &self,
            _: &str,
            _: usize,
            _: Option<&dyn Fn(&Document) -> bool>,
        ) -> Result<Vec<Document>> {
            Ok(vec![])
        }
        fn similarity_search_by_vector(
            &self,
            _: &[f32],
            _: usize,
            _: Option<&dyn Fn(&Document) -> bool>,
        ) -> Result<Vec<Document>> {
            Ok(vec![])
        }
        fn similarity_search_with_score(
            &self,
            _: &str,
            _: usize,
            _: Option<&dyn Fn(&Document) -> bool>,
        ) -> Result<Vec<(Document, f32)>> {
            Ok(vec![])
        }
        fn max_marginal_relevance_search(
            &self,
            _: &str,
            _: usize,
            _: usize,
            _: f32,
            _: Option<&dyn Fn(&Document) -> bool>,
        ) -> Result<Vec<Document>> {
            Ok(vec![])
        }
        fn max_marginal_relevance_search_by_vector(
            &self,
            _: &[f32],
            _: usize,
            _: usize,
            _: f32,
            _: Option<&dyn Fn(&Document) -> bool>,
        ) -> Result<Vec<Document>> {
            Ok(vec![])
        }
    }
}
