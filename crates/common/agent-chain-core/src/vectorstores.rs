pub mod base;
pub mod in_memory;
pub mod utils;

pub use base::{
    SearchType, VectorStore, VectorStoreFactory, VectorStoreRetriever, VectorStoreRetrieverConfig,
    VectorStoreRetrieverExt,
};
pub use in_memory::InMemoryVectorStore;
pub use utils::{cosine_similarity, maximal_marginal_relevance};
