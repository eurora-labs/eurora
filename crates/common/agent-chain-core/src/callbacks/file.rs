//! Callback Handler that writes to a file.
//!
//! This module provides a callback handler for writing output to a file,
//! following the Python LangChain FileCallbackHandler pattern.

use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{self, BufWriter, Write};
use std::path::Path;

use uuid::Uuid;

use super::base::{
    BaseCallbackHandler, CallbackManagerMixin, ChainManagerMixin, LLMManagerMixin,
    RetrieverManagerMixin, RunManagerMixin, ToolManagerMixin,
};

/// Callback Handler that writes to a file.
///
/// This handler supports writing callback output to a file. It can be used
/// to log chain execution to a file for debugging or auditing purposes.
///
/// # Example
///
/// ```ignore
/// use agent_chain_core::callbacks::FileCallbackHandler;
///
/// let handler = FileCallbackHandler::new("output.txt", false)?;
/// // Use handler with your chain
/// ```
#[derive(Debug)]
pub struct FileCallbackHandler {
    /// The file path.
    path: String,
    /// The buffered writer.
    writer: BufWriter<File>,
    /// The color to use for the text (not used for file output but kept for API compatibility).
    pub color: Option<String>,
}

impl FileCallbackHandler {
    /// Create a new FileCallbackHandler.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the output file.
    /// * `append` - Whether to append to the file or truncate it.
    ///
    /// # Returns
    ///
    /// A Result containing the FileCallbackHandler or an IO error.
    pub fn new<P: AsRef<Path>>(path: P, append: bool) -> io::Result<Self> {
        let file = if append {
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(path.as_ref())?
        } else {
            File::create(path.as_ref())?
        };

        Ok(Self {
            path: path.as_ref().to_string_lossy().to_string(),
            writer: BufWriter::new(file),
            color: None,
        })
    }

    /// Create a new FileCallbackHandler with a specific color.
    pub fn with_color<P: AsRef<Path>>(path: P, append: bool, color: impl Into<String>) -> io::Result<Self> {
        let mut handler = Self::new(path, append)?;
        handler.color = Some(color.into());
        Ok(handler)
    }

    /// Get the file path.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Write text to the file.
    fn write(&mut self, text: &str, end: &str) {
        let _ = write!(self.writer, "{}{}", text, end);
        let _ = self.writer.flush();
    }

    /// Flush the writer.
    pub fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}

impl LLMManagerMixin for FileCallbackHandler {}
impl RetrieverManagerMixin for FileCallbackHandler {}

impl ToolManagerMixin for FileCallbackHandler {
    fn on_tool_end(&mut self, output: &str, _run_id: Uuid, _parent_run_id: Option<Uuid>) {
        self.write(output, "");
    }
}

impl RunManagerMixin for FileCallbackHandler {
    fn on_text(&mut self, text: &str, _run_id: Uuid, _parent_run_id: Option<Uuid>) {
        self.write(text, "");
    }
}

impl CallbackManagerMixin for FileCallbackHandler {
    fn on_chain_start(
        &mut self,
        serialized: &HashMap<String, serde_json::Value>,
        _inputs: &HashMap<String, serde_json::Value>,
        _run_id: Uuid,
        _parent_run_id: Option<Uuid>,
        _tags: Option<&[String]>,
        _metadata: Option<&HashMap<String, serde_json::Value>>,
    ) {
        let name = serialized
            .get("name")
            .and_then(|v| v.as_str())
            .or_else(|| {
                serialized.get("id").and_then(|v| {
                    v.as_array()
                        .and_then(|arr| arr.last())
                        .and_then(|v| v.as_str())
                })
            })
            .unwrap_or("<unknown>");

        self.write(&format!("\n\n> Entering new {} chain...", name), "\n");
    }
}

impl ChainManagerMixin for FileCallbackHandler {
    fn on_chain_end(
        &mut self,
        _outputs: &HashMap<String, serde_json::Value>,
        _run_id: Uuid,
        _parent_run_id: Option<Uuid>,
    ) {
        self.write("\n> Finished chain.", "\n");
    }

    fn on_agent_action(
        &mut self,
        action: &serde_json::Value,
        _run_id: Uuid,
        _parent_run_id: Option<Uuid>,
    ) {
        if let Some(log) = action.get("log").and_then(|v| v.as_str()) {
            self.write(log, "");
        }
    }

    fn on_agent_finish(
        &mut self,
        finish: &serde_json::Value,
        _run_id: Uuid,
        _parent_run_id: Option<Uuid>,
    ) {
        if let Some(log) = finish.get("log").and_then(|v| v.as_str()) {
            self.write(log, "\n");
        }
    }
}

impl BaseCallbackHandler for FileCallbackHandler {
    fn name(&self) -> &str {
        "FileCallbackHandler"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_file_handler_creation() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test_output.txt");

        let handler = FileCallbackHandler::new(&file_path, false);
        assert!(handler.is_ok());

        let handler = handler.unwrap();
        assert_eq!(handler.name(), "FileCallbackHandler");
        assert!(handler.color.is_none());
    }

    #[test]
    fn test_file_handler_write() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test_write.txt");

        {
            let mut handler = FileCallbackHandler::new(&file_path, false).unwrap();
            handler.write("Hello, World!", "\n");
            handler.flush().unwrap();
        }

        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "Hello, World!\n");
    }

    #[test]
    fn test_file_handler_append() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test_append.txt");

        {
            let mut handler = FileCallbackHandler::new(&file_path, false).unwrap();
            handler.write("First line", "\n");
            handler.flush().unwrap();
        }

        {
            let mut handler = FileCallbackHandler::new(&file_path, true).unwrap();
            handler.write("Second line", "\n");
            handler.flush().unwrap();
        }

        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "First line\nSecond line\n");
    }

    #[test]
    fn test_file_handler_chain_callbacks() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test_chain.txt");

        {
            let mut handler = FileCallbackHandler::new(&file_path, false).unwrap();

            let mut serialized = HashMap::new();
            serialized.insert(
                "name".to_string(),
                serde_json::Value::String("TestChain".to_string()),
            );

            let run_id = Uuid::new_v4();
            handler.on_chain_start(&serialized, &HashMap::new(), run_id, None, None, None);
            handler.on_chain_end(&HashMap::new(), run_id, None);
            handler.flush().unwrap();
        }

        let content = fs::read_to_string(&file_path).unwrap();
        assert!(content.contains("Entering new TestChain chain"));
        assert!(content.contains("Finished chain"));
    }
}