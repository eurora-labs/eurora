pub mod fake;
mod inner;

pub use crate::runnables::run_in_executor;
pub use fake::{DeterministicFakeEmbedding, FakeEmbeddings};
pub use inner::Embeddings;
