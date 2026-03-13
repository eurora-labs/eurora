use std::collections::HashMap;

use async_trait::async_trait;
use serde_json::Value;

use crate::Result;

#[async_trait]
pub trait BaseExampleSelector: Send + Sync {
    async fn add_example(&mut self, example: HashMap<String, String>) -> Result<Option<String>>;

    async fn select_examples(
        &self,
        input_variables: HashMap<String, String>,
    ) -> Result<Vec<HashMap<String, Value>>>;
}
