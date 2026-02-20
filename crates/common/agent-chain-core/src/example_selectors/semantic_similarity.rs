use std::collections::HashMap;

use serde_json::Value;

use crate::Result;
use crate::documents::Document;
use crate::example_selectors::BaseExampleSelector;
use crate::vectorstores::VectorStore;

pub fn sorted_values(values: &HashMap<String, String>) -> Vec<String> {
    let mut keys: Vec<&String> = values.keys().collect();
    keys.sort();
    keys.into_iter().map(|k| values[k].clone()).collect()
}

fn example_to_text(example: &HashMap<String, String>, input_keys: &Option<Vec<String>>) -> String {
    if let Some(keys) = input_keys {
        let filtered: HashMap<String, String> = keys
            .iter()
            .filter_map(|k| example.get(k).map(|v| (k.clone(), v.clone())))
            .collect();
        sorted_values(&filtered).join(" ")
    } else {
        sorted_values(example).join(" ")
    }
}

fn documents_to_examples(
    documents: &[Document],
    example_keys: &Option<Vec<String>>,
) -> Vec<HashMap<String, Value>> {
    let mut examples: Vec<HashMap<String, Value>> =
        documents.iter().map(|doc| doc.metadata.clone()).collect();

    if let Some(keys) = example_keys {
        examples = examples
            .into_iter()
            .map(|example| {
                keys.iter()
                    .filter_map(|k| example.get(k).map(|v| (k.clone(), v.clone())))
                    .collect()
            })
            .collect();
    }

    examples
}

pub struct SemanticSimilarityExampleSelector {
    pub vectorstore: Box<dyn VectorStore>,
    pub k: usize,
    pub example_keys: Option<Vec<String>>,
    pub input_keys: Option<Vec<String>>,
}

impl SemanticSimilarityExampleSelector {
    pub fn new(
        vectorstore: Box<dyn VectorStore>,
        k: usize,
        input_keys: Option<Vec<String>>,
        example_keys: Option<Vec<String>>,
    ) -> Self {
        Self {
            vectorstore,
            k,
            example_keys,
            input_keys,
        }
    }
}

#[async_trait::async_trait]
impl BaseExampleSelector for SemanticSimilarityExampleSelector {
    fn add_example(&mut self, example: HashMap<String, String>) -> Result<Option<String>> {
        let text = example_to_text(&example, &self.input_keys);
        let metadata: HashMap<String, Value> = example
            .into_iter()
            .map(|(k, v)| (k, Value::String(v)))
            .collect();
        let ids = self
            .vectorstore
            .add_texts(vec![text], Some(vec![metadata]), None)?;
        Ok(ids.into_iter().next())
    }

    fn select_examples(
        &self,
        input_variables: HashMap<String, String>,
    ) -> Result<Vec<HashMap<String, Value>>> {
        let text = example_to_text(&input_variables, &self.input_keys);
        let example_docs = self.vectorstore.similarity_search(&text, self.k, None)?;
        Ok(documents_to_examples(&example_docs, &self.example_keys))
    }

    async fn aselect_examples(
        &self,
        input_variables: HashMap<String, String>,
    ) -> Result<Vec<HashMap<String, Value>>> {
        let text = example_to_text(&input_variables, &self.input_keys);
        let example_docs = self.vectorstore.similarity_search(&text, self.k, None)?;
        Ok(documents_to_examples(&example_docs, &self.example_keys))
    }
}

pub struct MaxMarginalRelevanceExampleSelector {
    pub vectorstore: Box<dyn VectorStore>,
    pub k: usize,
    pub fetch_k: usize,
    pub example_keys: Option<Vec<String>>,
    pub input_keys: Option<Vec<String>>,
}

impl MaxMarginalRelevanceExampleSelector {
    pub fn new(
        vectorstore: Box<dyn VectorStore>,
        k: usize,
        fetch_k: usize,
        input_keys: Option<Vec<String>>,
        example_keys: Option<Vec<String>>,
    ) -> Self {
        Self {
            vectorstore,
            k,
            fetch_k,
            example_keys,
            input_keys,
        }
    }
}

#[async_trait::async_trait]
impl BaseExampleSelector for MaxMarginalRelevanceExampleSelector {
    fn add_example(&mut self, example: HashMap<String, String>) -> Result<Option<String>> {
        let text = example_to_text(&example, &self.input_keys);
        let metadata: HashMap<String, Value> = example
            .into_iter()
            .map(|(k, v)| (k, Value::String(v)))
            .collect();
        let ids = self
            .vectorstore
            .add_texts(vec![text], Some(vec![metadata]), None)?;
        Ok(ids.into_iter().next())
    }

    fn select_examples(
        &self,
        input_variables: HashMap<String, String>,
    ) -> Result<Vec<HashMap<String, Value>>> {
        let text = example_to_text(&input_variables, &self.input_keys);
        let example_docs = self.vectorstore.max_marginal_relevance_search(
            &text,
            self.k,
            self.fetch_k,
            0.5,
            None,
        )?;
        Ok(documents_to_examples(&example_docs, &self.example_keys))
    }

    async fn aselect_examples(
        &self,
        input_variables: HashMap<String, String>,
    ) -> Result<Vec<HashMap<String, Value>>> {
        let text = example_to_text(&input_variables, &self.input_keys);
        let example_docs = self.vectorstore.max_marginal_relevance_search(
            &text,
            self.k,
            self.fetch_k,
            0.5,
            None,
        )?;
        Ok(documents_to_examples(&example_docs, &self.example_keys))
    }
}
