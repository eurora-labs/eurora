use std::collections::HashMap;

use serde_json::{self, Map, Value};

use crate::documents::Document;

pub struct RecursiveJsonSplitter {
    max_chunk_size: usize,
    min_chunk_size: usize,
}

impl RecursiveJsonSplitter {
    pub fn new(max_chunk_size: usize, min_chunk_size: Option<usize>) -> Self {
        let min_chunk_size =
            min_chunk_size.unwrap_or_else(|| max_chunk_size.saturating_sub(200).max(50));
        Self {
            max_chunk_size,
            min_chunk_size,
        }
    }

    fn json_size(data: &Value) -> usize {
        serde_json::to_string(data).map(|s| s.len()).unwrap_or(0)
    }

    fn set_nested_dict(destination: &mut Value, path: &[String], value: Value) {
        if path.is_empty() {
            return;
        }
        let mut current = destination;
        for key in &path[..path.len() - 1] {
            if !current.is_object() {
                *current = Value::Object(Map::new());
            }
            current = current
                .as_object_mut()
                .expect("ensured object above")
                .entry(key.clone())
                .or_insert_with(|| Value::Object(Map::new()));
        }
        if let Some(last_key) = path.last() {
            if !current.is_object() {
                *current = Value::Object(Map::new());
            }
            current
                .as_object_mut()
                .expect("ensured object above")
                .insert(last_key.clone(), value);
        }
    }

    fn list_to_dict_preprocessing(data: &Value) -> Value {
        match data {
            Value::Object(map) => {
                let new_map: Map<String, Value> = map
                    .iter()
                    .map(|(k, v)| (k.clone(), Self::list_to_dict_preprocessing(v)))
                    .collect();
                Value::Object(new_map)
            }
            Value::Array(arr) => {
                let new_map: Map<String, Value> = arr
                    .iter()
                    .enumerate()
                    .map(|(i, v)| (i.to_string(), Self::list_to_dict_preprocessing(v)))
                    .collect();
                Value::Object(new_map)
            }
            other => other.clone(),
        }
    }

    fn json_split(&self, data: &Value, current_path: &[String], chunks: &mut Vec<Value>) {
        if let Value::Object(map) = data {
            for (key, value) in map {
                let mut new_path = current_path.to_vec();
                new_path.push(key.clone());

                let chunk_size =
                    Self::json_size(chunks.last().unwrap_or(&Value::Object(Map::new())));
                let pair = serde_json::json!({ key: value });
                let size = Self::json_size(&pair);
                let remaining = self.max_chunk_size.saturating_sub(chunk_size);

                if size < remaining {
                    if chunks.is_empty() {
                        chunks.push(Value::Object(Map::new()));
                    }
                    let last_idx = chunks.len() - 1;
                    Self::set_nested_dict(&mut chunks[last_idx], &new_path, value.clone());
                } else {
                    if chunk_size >= self.min_chunk_size {
                        chunks.push(Value::Object(Map::new()));
                    }
                    self.json_split(value, &new_path, chunks);
                }
            }
        } else {
            if chunks.is_empty() {
                chunks.push(Value::Object(Map::new()));
            }
            let last_idx = chunks.len() - 1;
            Self::set_nested_dict(&mut chunks[last_idx], current_path, data.clone());
        }
    }

    pub fn split_json(
        &self,
        json_data: &Value,
        convert_lists: bool,
    ) -> Result<Vec<Value>, Box<dyn std::error::Error + Send + Sync>> {
        let data = if convert_lists {
            Self::list_to_dict_preprocessing(json_data)
        } else {
            json_data.clone()
        };

        let mut chunks = vec![Value::Object(Map::new())];
        self.json_split(&data, &[], &mut chunks);

        if let Some(last) = chunks.last() {
            if last == &Value::Object(Map::new()) {
                chunks.pop();
            }
        }

        Ok(chunks)
    }

    pub fn split_text(
        &self,
        json_data: &Value,
        convert_lists: bool,
    ) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
        let chunks = self.split_json(json_data, convert_lists)?;
        Ok(chunks
            .into_iter()
            .map(|chunk| serde_json::to_string(&chunk).unwrap_or_default())
            .collect())
    }

    pub fn create_documents(
        &self,
        texts: &[Value],
        convert_lists: bool,
        metadatas: Option<&[HashMap<String, Value>]>,
    ) -> Result<Vec<Document>, Box<dyn std::error::Error + Send + Sync>> {
        let empty: Vec<HashMap<String, Value>> = vec![HashMap::new(); texts.len()];
        let metadatas = metadatas.unwrap_or(&empty);

        let mut documents = Vec::new();
        for (i, text) in texts.iter().enumerate() {
            let metadata = metadatas.get(i).cloned().unwrap_or_default();
            for chunk in self.split_text(text, convert_lists)? {
                let doc = Document::builder()
                    .page_content(chunk)
                    .metadata(metadata.clone())
                    .build();
                documents.push(doc);
            }
        }
        Ok(documents)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recursive_json_splitter_basic() {
        let splitter = RecursiveJsonSplitter::new(100, None);
        let data = serde_json::json!({
            "name": "John",
            "age": 30,
            "address": {
                "street": "123 Main St",
                "city": "Anytown"
            }
        });
        let chunks = splitter.split_json(&data, false).unwrap();
        assert!(!chunks.is_empty());
    }

    #[test]
    fn test_recursive_json_splitter_large() {
        let splitter = RecursiveJsonSplitter::new(50, None);
        let data = serde_json::json!({
            "key1": "a".repeat(30),
            "key2": "b".repeat(30),
            "key3": "c".repeat(30),
        });
        let chunks = splitter.split_json(&data, false).unwrap();
        assert!(chunks.len() > 1);
    }

    #[test]
    fn test_recursive_json_splitter_convert_lists() {
        let splitter = RecursiveJsonSplitter::new(200, None);
        let data = serde_json::json!({
            "items": ["apple", "banana", "cherry"]
        });
        let chunks = splitter.split_json(&data, true).unwrap();
        assert!(!chunks.is_empty());
        // When converting lists, the array should become a dict with numeric keys
        let first = &chunks[0];
        assert!(first.is_object());
    }

    #[test]
    fn test_recursive_json_splitter_split_text() {
        let splitter = RecursiveJsonSplitter::new(200, None);
        let data = serde_json::json!({"key": "value"});
        let texts = splitter.split_text(&data, false).unwrap();
        assert_eq!(texts.len(), 1);
        assert!(texts[0].contains("key"));
        assert!(texts[0].contains("value"));
    }

    #[test]
    fn test_recursive_json_splitter_create_documents() {
        let splitter = RecursiveJsonSplitter::new(200, None);
        let data = vec![serde_json::json!({"key": "value"})];
        let docs = splitter.create_documents(&data, false, None).unwrap();
        assert_eq!(docs.len(), 1);
        assert!(docs[0].page_content.contains("key"));
    }

    #[test]
    fn test_set_nested_dict() {
        let mut root = Value::Object(Map::new());
        RecursiveJsonSplitter::set_nested_dict(
            &mut root,
            &["a".to_string(), "b".to_string(), "c".to_string()],
            Value::String("deep".to_string()),
        );
        assert_eq!(root["a"]["b"]["c"], "deep");
    }

    #[test]
    fn test_list_to_dict_preprocessing() {
        let data = serde_json::json!(["a", "b", ["c", "d"]]);
        let result = RecursiveJsonSplitter::list_to_dict_preprocessing(&data);
        assert!(result.is_object());
        assert_eq!(result["0"], "a");
        assert_eq!(result["1"], "b");
        assert!(result["2"].is_object());
        assert_eq!(result["2"]["0"], "c");
    }
}
