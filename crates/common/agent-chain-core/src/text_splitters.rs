use crate::documents::{BaseDocumentTransformer, Document};
use async_trait::async_trait;
use std::collections::HashMap;

#[async_trait]
pub trait TextSplitter: BaseDocumentTransformer {
    fn split_text(
        &self,
        text: &str,
    ) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>>;

    fn create_documents(
        &self,
        texts: &[String],
        metadatas: Option<&[HashMap<String, serde_json::Value>]>,
    ) -> Result<Vec<Document>, Box<dyn std::error::Error + Send + Sync>> {
        let empty: Vec<HashMap<String, serde_json::Value>> = vec![HashMap::new(); texts.len()];
        let metadatas = metadatas.unwrap_or(&empty);

        let mut documents = Vec::new();
        for (i, text) in texts.iter().enumerate() {
            let metadata = metadatas.get(i).cloned().unwrap_or_default();
            for chunk in self.split_text(text)? {
                let doc = Document::new(chunk).with_metadata(metadata.clone());
                documents.push(doc);
            }
        }
        Ok(documents)
    }

    fn split_documents(
        &self,
        documents: &[Document],
    ) -> Result<Vec<Document>, Box<dyn std::error::Error + Send + Sync>> {
        let texts: Vec<String> = documents.iter().map(|d| d.page_content.clone()).collect();
        let metadatas: Vec<HashMap<String, serde_json::Value>> =
            documents.iter().map(|d| d.metadata.clone()).collect();
        self.create_documents(&texts, Some(&metadatas))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct NewlineSplitter;

    #[async_trait]
    impl BaseDocumentTransformer for NewlineSplitter {
        fn transform_documents(
            &self,
            documents: Vec<Document>,
            _kwargs: HashMap<String, serde_json::Value>,
        ) -> Result<Vec<Document>, Box<dyn std::error::Error + Send + Sync>> {
            self.split_documents(&documents)
        }
    }

    #[async_trait]
    impl TextSplitter for NewlineSplitter {
        fn split_text(
            &self,
            text: &str,
        ) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
            Ok(text
                .split('\n')
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .collect())
        }
    }

    #[test]
    fn test_split_text() {
        let splitter = NewlineSplitter;
        let chunks = splitter.split_text("hello\nworld\n").unwrap();
        assert_eq!(chunks, vec!["hello", "world"]);
    }

    #[test]
    fn test_create_documents() {
        let splitter = NewlineSplitter;
        let texts = vec!["hello\nworld".to_string()];
        let docs = splitter.create_documents(&texts, None).unwrap();
        assert_eq!(docs.len(), 2);
        assert_eq!(docs[0].page_content, "hello");
        assert_eq!(docs[1].page_content, "world");
    }

    #[test]
    fn test_create_documents_with_metadata() {
        let splitter = NewlineSplitter;
        let texts = vec!["a\nb".to_string()];
        let metadata = vec![HashMap::from([(
            "source".to_string(),
            serde_json::json!("test.txt"),
        )])];
        let docs = splitter.create_documents(&texts, Some(&metadata)).unwrap();
        assert_eq!(docs.len(), 2);
        assert_eq!(docs[0].metadata["source"], "test.txt");
        assert_eq!(docs[1].metadata["source"], "test.txt");
    }

    #[test]
    fn test_split_documents() {
        let splitter = NewlineSplitter;
        let input_docs = vec![
            Document::new("hello\nworld"),
            Document::new("foo\nbar\nbaz"),
        ];
        let result = splitter.split_documents(&input_docs).unwrap();
        assert_eq!(result.len(), 5);
        assert_eq!(result[0].page_content, "hello");
        assert_eq!(result[1].page_content, "world");
        assert_eq!(result[2].page_content, "foo");
        assert_eq!(result[3].page_content, "bar");
        assert_eq!(result[4].page_content, "baz");
    }

    #[test]
    fn test_split_documents_preserves_metadata() {
        let splitter = NewlineSplitter;
        let mut metadata = HashMap::new();
        metadata.insert("key".to_string(), serde_json::json!("value"));
        let input_docs = vec![Document::new("a\nb").with_metadata(metadata)];
        let result = splitter.split_documents(&input_docs).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].metadata["key"], "value");
        assert_eq!(result[1].metadata["key"], "value");
    }

    #[test]
    fn test_transform_documents_delegates_to_split() {
        let splitter = NewlineSplitter;
        let docs = vec![Document::new("x\ny")];
        let result = splitter.transform_documents(docs, HashMap::new()).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].page_content, "x");
        assert_eq!(result[1].page_content, "y");
    }
}
