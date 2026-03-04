use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use bon::bon;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_with::serde_as;

use crate::error::Error;
use crate::load::Serializable;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Blob {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    id: Option<String>,

    #[serde(default)]
    metadata: HashMap<String, Value>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    data: Option<BlobData>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    mimetype: Option<String>,

    #[serde(default = "default_encoding")]
    encoding: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    path: Option<PathBuf>,
}

fn default_encoding() -> String {
    "utf-8".to_string()
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum BlobData {
    Text(String),
    Bytes(#[serde_as(as = "serde_with::base64::Base64")] Vec<u8>),
}

#[bon]
impl Blob {
    #[builder]
    pub fn new(
        id: Option<String>,
        #[builder(default)] metadata: HashMap<String, Value>,
        #[builder(into)] text: Option<String>,
        bytes: Option<Vec<u8>>,
        #[builder(into)] mimetype: Option<String>,
        #[builder(into, default = default_encoding())] encoding: String,
        path: Option<PathBuf>,
    ) -> crate::error::Result<Self> {
        let data = match (text, bytes) {
            (Some(t), None) => Some(BlobData::Text(t)),
            (None, Some(b)) => Some(BlobData::Bytes(b)),
            (Some(_), Some(_)) => {
                return Err(Error::ValidationError(
                    "Cannot provide both text and bytes".into(),
                ));
            }
            (None, None) => None,
        };
        if data.is_none() && path.is_none() {
            return Err(Error::ValidationError(
                "Either data/bytes or path must be provided".into(),
            ));
        }
        Ok(Self {
            id,
            metadata,
            data,
            mimetype,
            encoding,
            path,
        })
    }

    pub fn from_data(data: impl Into<String>) -> Self {
        Self {
            id: None,
            metadata: HashMap::new(),
            data: Some(BlobData::Text(data.into())),
            mimetype: None,
            encoding: "utf-8".to_string(),
            path: None,
        }
    }

    pub fn from_path(
        path: impl AsRef<Path>,
        mime_type: Option<String>,
        encoding: Option<String>,
        metadata: Option<HashMap<String, Value>>,
    ) -> Self {
        let path = path.as_ref();
        let mimetype = mime_type.or_else(|| guess_mime_type(path));

        Self {
            id: None,
            metadata: metadata.unwrap_or_default(),
            data: None,
            mimetype,
            encoding: encoding.unwrap_or_else(|| "utf-8".to_string()),
            path: Some(path.to_path_buf()),
        }
    }

    pub fn id(&self) -> Option<&str> {
        self.id.as_deref()
    }

    pub fn metadata(&self) -> &HashMap<String, Value> {
        &self.metadata
    }

    pub fn data(&self) -> Option<&BlobData> {
        self.data.as_ref()
    }

    pub fn mimetype(&self) -> Option<&str> {
        self.mimetype.as_deref()
    }

    pub fn encoding(&self) -> &str {
        &self.encoding
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    pub fn source(&self) -> Option<Cow<'_, str>> {
        if let Some(Value::String(source)) = self.metadata.get("source") {
            return Some(Cow::Borrowed(source));
        }
        self.path.as_ref().map(|p| p.to_string_lossy())
    }

    pub fn read_to_string(&self) -> io::Result<String> {
        match &self.data {
            Some(BlobData::Text(s)) => Ok(s.clone()),
            Some(BlobData::Bytes(b)) => String::from_utf8(b.clone())
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e)),
            None => {
                if let Some(path) = &self.path {
                    fs::read_to_string(path)
                } else {
                    Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("Unable to get string for blob {self}"),
                    ))
                }
            }
        }
    }

    pub fn read_to_bytes(&self) -> io::Result<Vec<u8>> {
        match &self.data {
            Some(BlobData::Bytes(b)) => Ok(b.clone()),
            Some(BlobData::Text(s)) => Ok(s.as_bytes().to_vec()),
            None => {
                if let Some(path) = &self.path {
                    fs::read(path)
                } else {
                    Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("Unable to get bytes for blob {self}"),
                    ))
                }
            }
        }
    }

    pub fn reader(&self) -> io::Result<Box<dyn Read>> {
        match &self.data {
            Some(BlobData::Bytes(b)) => Ok(Box::new(std::io::Cursor::new(b.clone()))),
            Some(BlobData::Text(s)) => Ok(Box::new(std::io::Cursor::new(s.as_bytes().to_vec()))),
            None => {
                if let Some(path) = &self.path {
                    let file = fs::File::open(path)?;
                    Ok(Box::new(std::io::BufReader::new(file)))
                } else {
                    Err(io::Error::other(format!(
                        "Unable to create reader for blob {self}"
                    )))
                }
            }
        }
    }
}

impl fmt::Display for Blob {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Blob")?;
        if let Some(source) = self.source() {
            write!(f, " source={source}")?;
        }
        if let Some(mime) = &self.mimetype {
            write!(f, " mimetype={mime}")?;
        }
        Ok(())
    }
}

fn guess_mime_type(path: &Path) -> Option<String> {
    mime_guess::from_path(path).first().map(|m| m.to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Document {
    pub page_content: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    #[serde(default)]
    pub metadata: HashMap<String, Value>,

    #[serde(rename = "type", default = "document_type_default")]
    type_: String,
}

fn document_type_default() -> String {
    "Document".to_string()
}

#[bon]
impl Document {
    #[builder]
    pub fn new(
        page_content: impl Into<String>,
        #[builder(into)] id: Option<String>,
        #[builder(default)] metadata: HashMap<String, Value>,
    ) -> Self {
        Self {
            page_content: page_content.into(),
            id,
            metadata,
            type_: "Document".to_string(),
        }
    }

    pub fn type_name(&self) -> &str {
        &self.type_
    }
}

impl fmt::Display for Document {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.metadata.is_empty() {
            write!(f, "page_content='{}'", self.page_content)
        } else {
            write!(
                f,
                "page_content='{}' metadata={:?}",
                self.page_content, self.metadata
            )
        }
    }
}

impl Serializable for Document {
    fn is_lc_serializable() -> bool
    where
        Self: Sized,
    {
        true
    }

    fn get_lc_namespace() -> Vec<String>
    where
        Self: Sized,
    {
        vec![
            "langchain".to_string(),
            "schema".to_string(),
            "document".to_string(),
        ]
    }
}

submit_constructor!(Document);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_document_creation() {
        let doc = Document::builder().page_content("Hello, world!").build();
        assert_eq!(doc.page_content, "Hello, world!");
        assert!(doc.id.is_none());
        assert!(doc.metadata.is_empty());
        assert_eq!(doc.type_name(), "Document");
    }

    #[test]
    fn test_document_with_metadata() {
        let doc = Document::builder()
            .page_content("Test content")
            .id("doc-123")
            .metadata(HashMap::from([(
                "source".to_string(),
                Value::String("test.txt".to_string()),
            )]))
            .build();

        assert_eq!(doc.id, Some("doc-123".to_string()));
        assert_eq!(
            doc.metadata.get("source"),
            Some(&Value::String("test.txt".to_string()))
        );
    }

    #[test]
    fn test_document_display() {
        let doc = Document::builder().page_content("Hello").build();
        assert_eq!(format!("{}", doc), "page_content='Hello'");

        let doc_with_meta = Document::builder()
            .page_content("Hello")
            .metadata(HashMap::from([("key".to_string(), Value::Bool(true))]))
            .build();
        let display = format!("{}", doc_with_meta);
        assert!(display.contains("page_content='Hello'"));
        assert!(display.contains("metadata="));
    }

    #[test]
    fn test_blob_from_data() {
        let blob = Blob::from_data("Hello, world!");
        assert_eq!(blob.read_to_string().unwrap(), "Hello, world!");
        assert_eq!(blob.read_to_bytes().unwrap(), b"Hello, world!");
    }

    #[test]
    fn test_blob_from_bytes() {
        let blob = Blob::builder()
            .bytes(b"Hello, bytes!".to_vec())
            .build()
            .unwrap();
        assert_eq!(blob.read_to_bytes().unwrap(), b"Hello, bytes!");
        assert_eq!(blob.read_to_string().unwrap(), "Hello, bytes!");
    }

    #[test]
    fn test_blob_builder() {
        let blob = Blob::builder()
            .text("Test data")
            .mimetype("text/plain")
            .encoding("utf-8")
            .build()
            .unwrap();

        assert_eq!(blob.read_to_string().unwrap(), "Test data");
        assert_eq!(blob.mimetype(), Some("text/plain"));
        assert_eq!(blob.encoding(), "utf-8");
    }

    #[test]
    fn test_blob_builder_error() {
        let result = Blob::builder().build();
        assert!(result.is_err());
    }

    #[test]
    fn test_blob_source() {
        let blob = Blob::from_path("/test/path.txt", None, None, None);
        assert_eq!(blob.source().as_deref(), Some("/test/path.txt"));

        let blob_with_source = Blob::builder()
            .text("test")
            .metadata(HashMap::from([(
                "source".to_string(),
                Value::String("custom_source".to_string()),
            )]))
            .build()
            .unwrap();
        assert_eq!(blob_with_source.source().as_deref(), Some("custom_source"));
    }

    #[test]
    fn test_guess_mime_type() {
        assert_eq!(
            guess_mime_type(Path::new("test.txt")),
            Some("text/plain".to_string())
        );
        assert_eq!(
            guess_mime_type(Path::new("test.json")),
            Some("application/json".to_string())
        );
        assert_eq!(
            guess_mime_type(Path::new("test.png")),
            Some("image/png".to_string())
        );
        assert_eq!(guess_mime_type(Path::new("test.unknown")), None);
    }

    #[test]
    fn test_document_serialization() {
        let doc = Document::builder()
            .page_content("Test content")
            .id("doc-123")
            .metadata(HashMap::from([(
                "source".to_string(),
                Value::String("test.txt".to_string()),
            )]))
            .build();

        let json = serde_json::to_string(&doc).unwrap();
        let deserialized: Document = serde_json::from_str(&json).unwrap();

        assert_eq!(doc, deserialized);
    }
}
