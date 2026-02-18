use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use rand::SeedableRng;
use rand::rngs::StdRng;
use rand_distr::{Distribution, StandardNormal};

use crate::Result;
use crate::embeddings::Embeddings;

/// Fake embedding model for unit testing purposes.
///
/// This embedding model creates embeddings by sampling from a normal distribution.
///
/// Do not use this outside of testing, as it is not a real embedding model.
pub struct FakeEmbeddings {
    /// The size of the embedding vector.
    pub size: usize,
}

impl FakeEmbeddings {
    pub fn new(size: usize) -> Self {
        Self { size }
    }

    fn get_embedding(&self) -> Vec<f32> {
        let mut rng = rand::rng();
        (0..self.size)
            .map(|_| StandardNormal.sample(&mut rng))
            .collect()
    }
}

#[async_trait::async_trait]
impl Embeddings for FakeEmbeddings {
    fn embed_documents(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        Ok(texts.iter().map(|_| self.get_embedding()).collect())
    }

    fn embed_query(&self, _text: &str) -> Result<Vec<f32>> {
        Ok(self.get_embedding())
    }
}

/// Deterministic fake embedding model for unit testing purposes.
///
/// This embedding model creates embeddings by sampling from a normal distribution
/// with a seed based on the hash of the text.
///
/// Do not use this outside of testing, as it is not a real embedding model.
pub struct DeterministicFakeEmbedding {
    /// The size of the embedding vector.
    pub size: usize,
}

impl DeterministicFakeEmbedding {
    pub fn new(size: usize) -> Self {
        Self { size }
    }

    fn get_seed(text: &str) -> u64 {
        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        hasher.finish() % 100_000_000
    }

    fn get_embedding(&self, seed: u64) -> Vec<f32> {
        let mut rng = StdRng::seed_from_u64(seed);
        (0..self.size)
            .map(|_| StandardNormal.sample(&mut rng))
            .collect()
    }
}

#[async_trait::async_trait]
impl Embeddings for DeterministicFakeEmbedding {
    fn embed_documents(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        Ok(texts
            .iter()
            .map(|text| {
                let seed = Self::get_seed(text);
                self.get_embedding(seed)
            })
            .collect())
    }

    fn embed_query(&self, text: &str) -> Result<Vec<f32>> {
        let seed = Self::get_seed(text);
        Ok(self.get_embedding(seed))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fake_embeddings() {
        let embeddings = FakeEmbeddings::new(10);
        let result = embeddings
            .embed_documents(vec!["hello".into(), "world".into()])
            .unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].len(), 10);
        assert_eq!(result[1].len(), 10);
    }

    #[test]
    fn test_fake_embeddings_query() {
        let embeddings = FakeEmbeddings::new(5);
        let result = embeddings.embed_query("hello").unwrap();
        assert_eq!(result.len(), 5);
    }

    #[test]
    fn test_deterministic_fake_embeddings() {
        let embeddings = DeterministicFakeEmbedding::new(10);
        let result1 = embeddings.embed_query("hello").unwrap();
        let result2 = embeddings.embed_query("hello").unwrap();
        assert_eq!(result1, result2);
    }

    #[test]
    fn test_deterministic_fake_embeddings_different_texts() {
        let embeddings = DeterministicFakeEmbedding::new(10);
        let result1 = embeddings.embed_query("hello").unwrap();
        let result2 = embeddings.embed_query("world").unwrap();
        assert_ne!(result1, result2);
    }

    #[test]
    fn test_deterministic_fake_embed_documents() {
        let embeddings = DeterministicFakeEmbedding::new(8);
        let texts = vec!["foo".into(), "bar".into(), "foo".into()];
        let result = embeddings.embed_documents(texts).unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].len(), 8);
        assert_eq!(result[0], result[2]);
        assert_ne!(result[0], result[1]);
    }
}
