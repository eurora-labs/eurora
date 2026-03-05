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

const DEFAULT_ENCODING: &str = "utf-8";
const DOCUMENT_TYPE: &str = "Document";

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
    encoding: Cow<'static, str>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    path: Option<PathBuf>,
}

fn default_encoding() -> Cow<'static, str> {
    Cow::Borrowed(DEFAULT_ENCODING)
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "value")]
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
        #[builder(into, default = default_encoding())] encoding: Cow<'static, str>,
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
            encoding: Cow::Borrowed(DEFAULT_ENCODING),
            path: None,
        }
    }

    pub fn from_path(path: impl AsRef<Path>) -> Self {
        let path = path.as_ref();
        let mimetype = guess_mime_type(path);

        Self {
            id: None,
            metadata: HashMap::new(),
            data: None,
            mimetype,
            encoding: Cow::Borrowed(DEFAULT_ENCODING),
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
            Some(BlobData::Bytes(b)) => decode_bytes(b, &self.encoding),
            None => {
                if let Some(path) = &self.path {
                    let bytes = fs::read(path)?;
                    decode_bytes(&bytes, &self.encoding)
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
            Some(BlobData::Text(s)) => encode_string(s, &self.encoding),
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

    pub fn reader(&self) -> io::Result<Box<dyn Read + '_>> {
        match &self.data {
            Some(BlobData::Bytes(b)) => Ok(Box::new(std::io::Cursor::new(b.as_slice()))),
            Some(BlobData::Text(s)) => Ok(Box::new(std::io::Cursor::new(s.as_bytes()))),
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

fn resolve_encoding(label: &str) -> io::Result<&'static encoding_rs::Encoding> {
    encoding_rs::Encoding::for_label(label.as_bytes()).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Unsupported encoding: {label}"),
        )
    })
}

fn decode_bytes(bytes: &[u8], encoding: &str) -> io::Result<String> {
    let enc = resolve_encoding(encoding)?;
    let (decoded, _, had_errors) = enc.decode(bytes);
    if had_errors {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Failed to decode bytes as {encoding}"),
        ));
    }
    Ok(decoded.into_owned())
}

fn encode_string(s: &str, encoding: &str) -> io::Result<Vec<u8>> {
    let enc = resolve_encoding(encoding)?;
    let (encoded, _, had_errors) = enc.encode(s);
    if had_errors {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Failed to encode string as {encoding}"),
        ));
    }
    Ok(encoded.into_owned())
}

fn guess_mime_type(path: &Path) -> Option<String> {
    mime_guess::from_path(path).first().map(|m| m.to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Document {
    page_content: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    id: Option<String>,

    #[serde(default)]
    metadata: HashMap<String, Value>,

    #[serde(rename = "type", default = "document_type_default")]
    type_: Cow<'static, str>,
}

fn document_type_default() -> Cow<'static, str> {
    Cow::Borrowed(DOCUMENT_TYPE)
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
            type_: Cow::Borrowed(DOCUMENT_TYPE),
        }
    }

    pub fn page_content(&self) -> &str {
        &self.page_content
    }

    pub fn id(&self) -> Option<&str> {
        self.id.as_deref()
    }

    pub fn metadata(&self) -> &HashMap<String, Value> {
        &self.metadata
    }

    pub fn page_content_mut(&mut self) -> &mut String {
        &mut self.page_content
    }

    pub fn metadata_mut(&mut self) -> &mut HashMap<String, Value> {
        &mut self.metadata
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
        assert_eq!(doc.page_content(), "Hello, world!");
        assert!(doc.id().is_none());
        assert!(doc.metadata().is_empty());
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

        assert_eq!(doc.id(), Some("doc-123"));
        assert_eq!(
            doc.metadata().get("source"),
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
        let blob = Blob::from_path("/test/path.txt");
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

    #[test]
    fn test_blob_bytes_roundtrip() {
        let original_bytes = vec![0xFF, 0xFE, 0x00, 0x01];
        let blob = Blob::builder()
            .bytes(original_bytes.clone())
            .mimetype("application/octet-stream")
            .build()
            .unwrap();

        let json = serde_json::to_string(&blob).unwrap();
        let deserialized: Blob = serde_json::from_str(&json).unwrap();

        assert_eq!(blob, deserialized);
        assert_eq!(deserialized.read_to_bytes().unwrap(), original_bytes);
    }

    #[test]
    fn test_blob_text_roundtrip() {
        let blob = Blob::from_data("Hello, world!");

        let json = serde_json::to_string(&blob).unwrap();
        let deserialized: Blob = serde_json::from_str(&json).unwrap();

        assert_eq!(blob, deserialized);
        assert_eq!(deserialized.read_to_string().unwrap(), "Hello, world!");
    }
}
