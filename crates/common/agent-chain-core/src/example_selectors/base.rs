use std::collections::HashMap;

use async_trait::async_trait;
use serde_json::Value;

use crate::Result;

#[async_trait]
pub trait BaseExampleSelector: Send + Sync {
    fn add_example(&mut self, example: HashMap<String, String>) -> Result<Option<String>>;

    async fn aadd_example(&mut self, example: HashMap<String, String>) -> Result<Option<String>> {
        self.add_example(example)
    }

    fn select_examples(
        &self,
        input_variables: HashMap<String, String>,
    ) -> Result<Vec<HashMap<String, Value>>>;

    async fn aselect_examples(
        &self,
        input_variables: HashMap<String, String>,
    ) -> Result<Vec<HashMap<String, Value>>> {
        self.select_examples(input_variables)
    }
}
