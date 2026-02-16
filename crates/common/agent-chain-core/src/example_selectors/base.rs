use std::collections::HashMap;

use async_trait::async_trait;
use serde_json::Value;

use crate::Result;

/// Interface for selecting examples to include in prompts.
#[async_trait]
pub trait BaseExampleSelector: Send + Sync {
    /// Add new example to store.
    fn add_example(&mut self, example: HashMap<String, String>) -> Result<Option<String>>;

    /// Async add new example to store.
    async fn aadd_example(&mut self, example: HashMap<String, String>) -> Result<Option<String>> {
        self.add_example(example)
    }

    /// Select which examples to use based on the inputs.
    fn select_examples(
        &self,
        input_variables: HashMap<String, String>,
    ) -> Result<Vec<HashMap<String, Value>>>;

    /// Async select which examples to use based on the inputs.
    async fn aselect_examples(
        &self,
        input_variables: HashMap<String, String>,
    ) -> Result<Vec<HashMap<String, Value>>> {
        self.select_examples(input_variables)
    }
}
