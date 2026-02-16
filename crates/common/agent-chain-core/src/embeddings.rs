pub mod fake;
mod inner;

pub use fake::{DeterministicFakeEmbedding, FakeEmbeddings};
pub use inner::Embeddings;
