use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{self, BufWriter, Write};
use std::path::Path;
use std::sync::Mutex;

use uuid::Uuid;

use super::base::{BaseCallbackHandler, resolve_chain_name};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileMode {
    Write,
    Append,
    CreateNew,
}

impl FileMode {
    fn open(&self, path: &Path) -> io::Result<File> {
        match self {
            FileMode::Write => File::create(path),
            FileMode::Append => OpenOptions::new().create(true).append(true).open(path),
            FileMode::CreateNew => OpenOptions::new().create_new(true).write(true).open(path),
        }
    }
}

#[derive(Debug)]
pub struct FileCallbackHandler {
    filename: String,
    mode: FileMode,
    file: Mutex<Option<BufWriter<File>>>,
}

impl FileCallbackHandler {
    pub fn new<P: AsRef<Path>>(filename: P, append: bool) -> io::Result<Self> {
        let mode = if append {
            FileMode::Append
        } else {
            FileMode::Write
        };
        Self::with_mode(filename, mode)
    }

    pub fn with_mode<P: AsRef<Path>>(filename: P, mode: FileMode) -> io::Result<Self> {
        let file = mode.open(filename.as_ref())?;

        Ok(Self {
            filename: filename.as_ref().to_string_lossy().to_string(),
            mode,
            file: Mutex::new(Some(BufWriter::new(file))),
        })
    }

    pub fn filename(&self) -> &str {
        &self.filename
    }

    pub fn mode(&self) -> FileMode {
        self.mode
    }

    pub fn close(&self) {
        if let Some(mut writer) = self.file.lock().expect("file lock poisoned").take()
            && let Err(e) = writer.flush()
        {
            tracing::warn!("FileCallbackHandler close flush error: {e}");
        }
    }

    fn write(&self, text: &str, end: &str) {
        if let Some(ref mut writer) = *self.file.lock().expect("file lock poisoned")
            && let Err(e) = write!(writer, "{}{}", text, end)
        {
            tracing::warn!("FileCallbackHandler write error: {e}");
        }
    }

    pub fn flush(&self) -> io::Result<()> {
        if let Some(ref mut writer) = *self.file.lock().expect("file lock poisoned") {
            writer.flush()
        } else {
            Ok(())
        }
    }
}

impl Drop for FileCallbackHandler {
    fn drop(&mut self) {
        if let Ok(mut guard) = self.file.lock()
            && let Some(mut writer) = guard.take()
            && let Err(e) = writer.flush()
        {
            eprintln!("FileCallbackHandler drop flush error: {e}");
        }
    }
}

impl BaseCallbackHandler for FileCallbackHandler {
    fn name(&self) -> &str {
        "FileCallbackHandler"
    }

    fn on_tool_end(
        &self,
        output: &str,
        _run_id: Uuid,
        _parent_run_id: Option<Uuid>,
        _color: Option<&str>,
        observation_prefix: Option<&str>,
        llm_prefix: Option<&str>,
    ) {
        if let Some(prefix) = observation_prefix {
            self.write(&format!("\n{}", prefix), "");
        }
        self.write(output, "");
        if let Some(prefix) = llm_prefix {
            self.write(&format!("\n{}", prefix), "");
        }
    }

    fn on_text(
        &self,
        text: &str,
        _run_id: Uuid,
        _parent_run_id: Option<Uuid>,
        _color: Option<&str>,
        end: &str,
    ) {
        self.write(text, end);
    }

    fn on_chain_start(
        &self,
        serialized: &HashMap<String, serde_json::Value>,
        _inputs: &HashMap<String, serde_json::Value>,
        _run_id: Uuid,
        _parent_run_id: Option<Uuid>,
        _tags: Option<&[String]>,
        _metadata: Option<&HashMap<String, serde_json::Value>>,
        name: Option<&str>,
    ) {
        let name = resolve_chain_name(serialized, name);
        self.write(&format!("\n\n> Entering new {} chain...", name), "\n");
    }

    fn on_chain_end(
        &self,
        _outputs: &HashMap<String, serde_json::Value>,
        _run_id: Uuid,
        _parent_run_id: Option<Uuid>,
    ) {
        self.write("\n> Finished chain.", "\n");
    }

    fn on_agent_action(
        &self,
        action: &serde_json::Value,
        _run_id: Uuid,
        _parent_run_id: Option<Uuid>,
        _color: Option<&str>,
    ) {
        if let Some(log) = action.get("log").and_then(|v| v.as_str()) {
            self.write(log, "");
        }
    }

    fn on_agent_finish(
        &self,
        finish: &serde_json::Value,
        _run_id: Uuid,
        _parent_run_id: Option<Uuid>,
        _color: Option<&str>,
    ) {
        if let Some(log) = finish.get("log").and_then(|v| v.as_str()) {
            self.write(log, "\n");
        }
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
        assert_eq!(handler.mode(), FileMode::Write);
    }

    #[test]
    fn test_file_handler_with_mode() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test_mode.txt");

        let handler = FileCallbackHandler::with_mode(&file_path, FileMode::Write);
        assert!(handler.is_ok());
        let handler = handler.unwrap();
        assert_eq!(handler.mode(), FileMode::Write);

        let handler = FileCallbackHandler::with_mode(&file_path, FileMode::Append);
        assert!(handler.is_ok());
        let handler = handler.unwrap();
        assert_eq!(handler.mode(), FileMode::Append);

        let handler = FileCallbackHandler::with_mode(&file_path, FileMode::CreateNew);
        assert!(handler.is_err());
    }

    #[test]
    fn test_file_handler_write() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test_write.txt");

        {
            let handler = FileCallbackHandler::new(&file_path, false).unwrap();
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
            let handler = FileCallbackHandler::new(&file_path, false).unwrap();
            handler.write("First line", "\n");
            handler.flush().unwrap();
        }

        {
            let handler = FileCallbackHandler::new(&file_path, true).unwrap();
            handler.write("Second line", "\n");
            handler.flush().unwrap();
        }

        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "First line\nSecond line\n");
    }

    #[test]
    fn test_file_handler_close() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test_close.txt");

        let handler = FileCallbackHandler::new(&file_path, false).unwrap();
        handler.write("Before close", "\n");

        handler.close();

        handler.write("After close", "\n");

        handler.close();

        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "Before close\n");
    }

    #[test]
    fn test_file_handler_chain_callbacks() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test_chain.txt");

        {
            let handler = FileCallbackHandler::new(&file_path, false).unwrap();

            let mut serialized = HashMap::new();
            serialized.insert(
                "name".to_string(),
                serde_json::Value::String("TestChain".to_string()),
            );

            let run_id = Uuid::new_v4();
            handler.on_chain_start(&serialized, &HashMap::new(), run_id, None, None, None, None);
            handler.on_chain_end(&HashMap::new(), run_id, None);
            handler.flush().unwrap();
        }

        let content = fs::read_to_string(&file_path).unwrap();
        assert!(content.contains("Entering new TestChain chain"));
        assert!(content.contains("Finished chain"));
    }

    #[test]
    fn test_file_handler_agent_callbacks() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test_agent.txt");

        {
            let handler = FileCallbackHandler::new(&file_path, false).unwrap();
            let run_id = Uuid::new_v4();

            let action = serde_json::json!({
                "log": "Agent thinking...",
                "tool": "search",
                "tool_input": "query"
            });
            handler.on_agent_action(&action, run_id, None, None);

            let finish = serde_json::json!({
                "log": "Agent finished.",
                "return_values": {"output": "result"}
            });
            handler.on_agent_finish(&finish, run_id, None, None);

            handler.flush().unwrap();
        }

        let content = fs::read_to_string(&file_path).unwrap();
        assert!(content.contains("Agent thinking..."));
        assert!(content.contains("Agent finished."));
    }

    #[test]
    fn test_file_handler_tool_and_text_callbacks() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test_tool_text.txt");

        {
            let handler = FileCallbackHandler::new(&file_path, false).unwrap();
            let run_id = Uuid::new_v4();

            handler.on_tool_end("Tool output here", run_id, None, None, None, None);

            handler.on_text("Some text output", run_id, None, None, "");

            handler.flush().unwrap();
        }

        let content = fs::read_to_string(&file_path).unwrap();
        assert!(content.contains("Tool output here"));
        assert!(content.contains("Some text output"));
    }
}
