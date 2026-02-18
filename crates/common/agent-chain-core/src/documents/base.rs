//! Base classes for media and documents.
//!
//! This module contains core abstractions for **data retrieval and processing workflows**:
//!
//! - [`BaseMedia`]: Base struct providing `id` and `metadata` fields
//! - [`Blob`]: Raw data loading (files, binary data) - used by document loaders
//! - [`Document`]: Text content for retrieval (RAG, vector stores, semantic search)
//!
//! These structs are for data processing pipelines, not LLM I/O. For multimodal
//! content in chat messages (images, audio in threads), see the `messages`
//! module content blocks instead.

use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::load::Serializable;

/// Base struct for content used in retrieval and data processing workflows.
///
/// Provides common fields for content that needs to be stored, indexed, or searched.
///
/// For multimodal content in **chat messages** (images, audio sent to/from LLMs),
/// use the `messages` module content blocks instead.

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct BaseMedia {
    /// An optional identifier for the document.
    ///
    /// Ideally this should be unique across the document collection and formatted
    /// as a UUID, but this will not be enforced.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Arbitrary metadata associated with the content.
    #[serde(default)]
    pub metadata: HashMap<String, Value>,
}

impl BaseMedia {
    /// Create a new BaseMedia with the given ID and metadata.
    pub fn new(id: Option<String>, metadata: HashMap<String, Value>) -> Self {
        Self { id, metadata }
    }
}

/// Raw data abstraction for document loading and file processing.
///
/// Represents raw bytes or text, either in-memory or by file reference. Used
/// primarily by document loaders to decouple data loading from parsing.
///
/// Inspired by [Mozilla's `Blob`](https://developer.mozilla.org/en-US/docs/Web/API/Blob)
///
/// # Examples
///
/// Initialize a blob from in-memory data:
///
/// ```
/// use agent_chain_core::documents::Blob;
///
/// let blob = Blob::from_data("Hello, world!");
///
/// // Read the blob as a string
/// assert_eq!(blob.as_string().unwrap(), "Hello, world!");
///
/// // Read the blob as bytes
/// assert_eq!(blob.as_bytes().unwrap(), b"Hello, world!");
/// ```
///
/// Load from memory and specify MIME type and metadata:
///
/// ```
/// use agent_chain_core::documents::Blob;
/// use std::collections::HashMap;
///
/// let blob = Blob::builder()
///     .data("Hello, world!")
///     .mime_type("text/plain")
///     .metadata(HashMap::from([("source".to_string(), serde_json::json!("https://example.com"))]))
///     .build()
///     .unwrap();
/// ```

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Blob {
    /// An optional identifier for the blob.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Arbitrary metadata associated with the blob.
    #[serde(default)]
    pub metadata: HashMap<String, Value>,

    /// Raw data associated with the blob (bytes or string).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<BlobData>,

    /// MIME type, not to be confused with a file extension.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mimetype: Option<String>,

    /// Encoding to use if decoding the bytes into a string.
    /// Uses `utf-8` as default encoding if decoding to string.
    #[serde(default = "default_encoding")]
    pub encoding: String,

    /// Location where the original content was found.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<PathBuf>,
}

fn default_encoding() -> String {
    "utf-8".to_string()
}

/// Data stored in a Blob.

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum BlobData {
    /// Text data.
    Text(String),
    /// Binary data.
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
    /// Create a new Blob builder.
    pub fn builder() -> BlobBuilder {
        BlobBuilder::default()
    }

    /// Create a Blob from in-memory data (string).
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

    /// Load the blob from a path.
    ///
    /// The data is not loaded immediately - the blob treats the path as a
    /// reference to the underlying data.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the file
    /// * `mime_type` - Optional MIME type (will be guessed from extension if not provided)
    /// * `encoding` - Encoding to use (defaults to "utf-8")
    /// * `metadata` - Optional metadata
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

    /// The source location of the blob as string if known otherwise none.
    ///
    /// If a path is associated with the Blob, it will default to the path location.
    /// Unless explicitly set via a metadata field called `'source'`, in which
    /// case that value will be used instead.
    pub fn source(&self) -> Option<String> {
        if let Some(Value::String(source)) = self.metadata.get("source") {
            return Some(source.clone());
        }
        self.path.as_ref().map(|p| p.to_string_lossy().to_string())
    }

    /// Read data as a string.
    ///
    /// # Errors
    ///
    /// Returns an error if the blob cannot be represented as a string.
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

    /// Read data as bytes.
    ///
    /// # Errors
    ///
    /// Returns an error if the blob cannot be represented as bytes.
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

    /// Read data as a byte stream (returns a reader).
    ///
    /// # Errors
    ///
    /// Returns an error if the blob cannot be represented as a byte stream.
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

/// Builder for creating Blob instances.
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
    /// Set the ID.
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Set the metadata.
    pub fn metadata(mut self, metadata: HashMap<String, Value>) -> Self {
        self.metadata = metadata;
        self
    }

    /// Set the data from a string.
    pub fn data(mut self, data: impl Into<String>) -> Self {
        self.data = Some(BlobData::Text(data.into()));
        self
    }

    /// Set the data from bytes.
    pub fn bytes(mut self, data: Vec<u8>) -> Self {
        self.data = Some(BlobData::Bytes(data));
        self
    }

    /// Set the MIME type.
    pub fn mime_type(mut self, mime_type: impl Into<String>) -> Self {
        self.mimetype = Some(mime_type.into());
        self
    }

    /// Set the encoding.
    pub fn encoding(mut self, encoding: impl Into<String>) -> Self {
        self.encoding = encoding.into();
        self
    }

    /// Set the path.
    pub fn path(mut self, path: impl AsRef<Path>) -> Self {
        self.path = Some(path.as_ref().to_path_buf());
        self
    }

    /// Build the Blob.
    ///
    /// # Errors
    ///
    /// Returns an error if neither data nor path is provided.
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

/// Guess MIME type from file extension.
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

/// Class for storing a piece of text and associated metadata.
///
/// [`Document`] is for **retrieval workflows**, not chat I/O. For sending text
/// to an LLM in a thread, use message types from the `messages` module.
///
/// # Example
///
/// ```
/// use agent_chain_core::documents::Document;
/// use std::collections::HashMap;
///
/// let document = Document::new("Hello, world!")
///     .with_metadata(HashMap::from([
///         ("source".to_string(), serde_json::json!("https://example.com"))
///     ]));
/// ```

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Document {
    /// String text content of the document.
    pub page_content: String,

    /// An optional identifier for the document.
    ///
    /// Ideally this should be unique across the document collection and formatted
    /// as a UUID, but this will not be enforced.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Arbitrary metadata associated with the content.
    #[serde(default)]
    pub metadata: HashMap<String, Value>,

    /// Type identifier, always "Document".
    #[serde(rename = "type", default = "document_type_default")]
    pub type_: String,
}

fn document_type_default() -> String {
    "Document".to_string()
}

impl Document {
    /// Create a new Document with the given page content.
    pub fn new(page_content: impl Into<String>) -> Self {
        Self {
            page_content: page_content.into(),
            id: None,
            metadata: HashMap::new(),
            type_: "Document".to_string(),
        }
    }

    /// Set the ID.
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Set the metadata.
    pub fn with_metadata(mut self, metadata: HashMap<String, Value>) -> Self {
        self.metadata = metadata;
        self
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
