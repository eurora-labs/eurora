use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use bon::bon;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::load::Serializable;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct BaseMedia {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    #[serde(default)]
    pub metadata: HashMap<String, Value>,
}

#[bon]
impl BaseMedia {
    #[builder]
    pub fn new(id: Option<String>, #[builder(default)] metadata: HashMap<String, Value>) -> Self {
        Self { id, metadata }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Blob {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    #[serde(default)]
    pub metadata: HashMap<String, Value>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<BlobData>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mimetype: Option<String>,

    #[serde(default = "default_encoding")]
    pub encoding: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<PathBuf>,
}

fn default_encoding() -> String {
    "utf-8".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum BlobData {
    Text(String),
    #[serde(with = "serde_bytes_base64")]
    Bytes(Vec<u8>),
}

mod serde_bytes_base64 {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    use serde::{self, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(bytes: &Vec<u8>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = STANDARD.encode(bytes);
        serializer.serialize_str(&s)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        STANDARD.decode(&s).map_err(serde::de::Error::custom)
    }
}

impl Blob {
    pub fn builder() -> BlobBuilder {
        BlobBuilder::default()
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

    pub fn source(&self) -> Option<String> {
        if let Some(Value::String(source)) = self.metadata.get("source") {
            return Some(source.clone());
        }
        self.path.as_ref().map(|p| p.to_string_lossy().to_string())
    }

    pub fn as_string(&self) -> io::Result<String> {
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
                        format!("Unable to get string for blob {:?}", self),
                    ))
                }
            }
        }
    }

    pub fn as_bytes(&self) -> io::Result<Vec<u8>> {
        match &self.data {
            Some(BlobData::Bytes(b)) => Ok(b.clone()),
            Some(BlobData::Text(s)) => Ok(s.as_bytes().to_vec()),
            None => {
                if let Some(path) = &self.path {
                    fs::read(path)
                } else {
                    Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("Unable to get bytes for blob {:?}", self),
                    ))
                }
            }
        }
    }

    pub fn as_bytes_io(&self) -> io::Result<Box<dyn Read>> {
        match &self.data {
            Some(BlobData::Bytes(b)) => Ok(Box::new(std::io::Cursor::new(b.clone()))),
            Some(BlobData::Text(s)) => Ok(Box::new(std::io::Cursor::new(s.as_bytes().to_vec()))),
            None => {
                if let Some(path) = &self.path {
                    let file = fs::File::open(path)?;
                    Ok(Box::new(std::io::BufReader::new(file)))
                } else {
                    Err(io::Error::other(format!(
                        "Unable to convert blob {:?}",
                        self
                    )))
                }
            }
        }
    }
}

impl fmt::Display for Blob {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Blob {:p}", self)?;
        if let Some(source) = self.source() {
            write!(f, " {}", source)?;
        }
        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct BlobBuilder {
    id: Option<String>,
    metadata: HashMap<String, Value>,
    data: Option<BlobData>,
    mimetype: Option<String>,
    encoding: String,
    path: Option<PathBuf>,
}

impl BlobBuilder {
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    pub fn metadata(mut self, metadata: HashMap<String, Value>) -> Self {
        self.metadata = metadata;
        self
    }

    pub fn data(mut self, data: impl Into<String>) -> Self {
        self.data = Some(BlobData::Text(data.into()));
        self
    }

    pub fn bytes(mut self, data: Vec<u8>) -> Self {
        self.data = Some(BlobData::Bytes(data));
        self
    }

    pub fn mime_type(mut self, mime_type: impl Into<String>) -> Self {
        self.mimetype = Some(mime_type.into());
        self
    }

    pub fn encoding(mut self, encoding: impl Into<String>) -> Self {
        self.encoding = encoding.into();
        self
    }

    pub fn path(mut self, path: impl AsRef<Path>) -> Self {
        self.path = Some(path.as_ref().to_path_buf());
        self
    }

    pub fn build(self) -> Result<Blob, &'static str> {
        if self.data.is_none() && self.path.is_none() {
            return Err("Either data or path must be provided");
        }

        Ok(Blob {
            id: self.id,
            metadata: self.metadata,
            data: self.data,
            mimetype: self.mimetype,
            encoding: if self.encoding.is_empty() {
                "utf-8".to_string()
            } else {
                self.encoding
            },
            path: self.path,
        })
    }
}

fn guess_mime_type(path: &Path) -> Option<String> {
    path.extension().and_then(|ext| {
        let ext = ext.to_string_lossy().to_lowercase();
        match ext.as_str() {
            "txt" => Some("text/plain".to_string()),
            "html" | "htm" => Some("text/html".to_string()),
            "css" => Some("text/css".to_string()),
            "js" => Some("application/javascript".to_string()),
            "json" => Some("application/json".to_string()),
            "xml" => Some("application/xml".to_string()),
            "pdf" => Some("application/pdf".to_string()),
            "png" => Some("image/png".to_string()),
            "jpg" | "jpeg" => Some("image/jpeg".to_string()),
            "gif" => Some("image/gif".to_string()),
            "svg" => Some("image/svg+xml".to_string()),
            "mp3" => Some("audio/mpeg".to_string()),
            "wav" => Some("audio/wav".to_string()),
            "mp4" => Some("video/mp4".to_string()),
            "webm" => Some("video/webm".to_string()),
            "zip" => Some("application/zip".to_string()),
            "gz" | "gzip" => Some("application/gzip".to_string()),
            "tar" => Some("application/x-tar".to_string()),
            "csv" => Some("text/csv".to_string()),
            "md" => Some("text/markdown".to_string()),
            "yaml" | "yml" => Some("application/x-yaml".to_string()),
            "toml" => Some("application/toml".to_string()),
            "rs" => Some("text/x-rust".to_string()),
            "py" => Some("text/x-python".to_string()),
            _ => None,
        }
    })
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Document {
    pub page_content: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    #[serde(default)]
    pub metadata: HashMap<String, Value>,

    #[serde(rename = "type", default = "document_type_default")]
    pub type_: String,
}

fn document_type_default() -> String {
    "Document".to_string()
}

#[bon]
impl Document {
    #[builder]
    pub fn new(
        page_content: impl Into<String>,
        id: Option<String>,
        #[builder(default)] metadata: HashMap<String, Value>,
    ) -> Self {
        Self {
            page_content: page_content.into(),
            id,
            metadata,
            type_: "Document".to_string(),
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_document_creation() {
        let doc = Document::new("Hello, world!");
        assert_eq!(doc.page_content, "Hello, world!");
        assert!(doc.id.is_none());
        assert!(doc.metadata.is_empty());
        assert_eq!(doc.type_, "Document");
    }

    #[test]
    fn test_document_with_metadata() {
        let doc = Document::new("Test content")
            .with_id("doc-123")
            .with_metadata(HashMap::from([(
                "source".to_string(),
                Value::String("test.txt".to_string()),
            )]));

        assert_eq!(doc.id, Some("doc-123".to_string()));
        assert_eq!(
            doc.metadata.get("source"),
            Some(&Value::String("test.txt".to_string()))
        );
    }

    #[test]
    fn test_document_display() {
        let doc = Document::new("Hello");
        assert_eq!(format!("{}", doc), "page_content='Hello'");

        let doc_with_meta = Document::new("Hello")
            .with_metadata(HashMap::from([("key".to_string(), Value::Bool(true))]));
        let display = format!("{}", doc_with_meta);
        assert!(display.contains("page_content='Hello'"));
        assert!(display.contains("metadata="));
    }

    #[test]
    fn test_blob_from_data() {
        let blob = Blob::from_data("Hello, world!");
        assert_eq!(blob.as_string().unwrap(), "Hello, world!");
        assert_eq!(blob.as_bytes().unwrap(), b"Hello, world!");
    }

    #[test]
    fn test_blob_from_bytes() {
        let blob = Blob::builder()
            .bytes(b"Hello, bytes!".to_vec())
            .build()
            .unwrap();
        assert_eq!(blob.as_bytes().unwrap(), b"Hello, bytes!");
        assert_eq!(blob.as_string().unwrap(), "Hello, bytes!");
    }

    #[test]
    fn test_blob_builder() {
        let blob = Blob::builder()
            .data("Test data")
            .mime_type("text/plain")
            .encoding("utf-8")
            .build()
            .unwrap();

        assert_eq!(blob.as_string().unwrap(), "Test data");
        assert_eq!(blob.mimetype, Some("text/plain".to_string()));
        assert_eq!(blob.encoding, "utf-8");
    }

    #[test]
    fn test_blob_builder_error() {
        let result = Blob::builder().build();
        assert!(result.is_err());
    }

    #[test]
    fn test_blob_source() {
        let blob = Blob::from_path("/test/path.txt", None, None, None);
        assert_eq!(blob.source(), Some("/test/path.txt".to_string()));

        let blob_with_source = Blob::builder()
            .data("test")
            .metadata(HashMap::from([(
                "source".to_string(),
                Value::String("custom_source".to_string()),
            )]))
            .build()
            .unwrap();
        assert_eq!(blob_with_source.source(), Some("custom_source".to_string()));
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
        let doc = Document::new("Test content")
            .with_id("doc-123")
            .with_metadata(HashMap::from([(
                "source".to_string(),
                Value::String("test.txt".to_string()),
            )]));

        let json = serde_json::to_string(&doc).unwrap();
        let deserialized: Document = serde_json::from_str(&json).unwrap();

        assert_eq!(doc, deserialized);
    }
}
