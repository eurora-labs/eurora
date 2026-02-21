use std::io::{self, Write};
use std::sync::{Arc, Mutex};

use uuid::Uuid;

use super::base::{
    BaseCallbackHandler, CallbackManagerMixin, ChainManagerMixin, LLMManagerMixin,
    RetrieverManagerMixin, RunManagerMixin, ToolManagerMixin,
};

#[derive(Clone)]
pub struct StreamingStdOutCallbackHandler {
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
}

impl std::fmt::Debug for StreamingStdOutCallbackHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StreamingStdOutCallbackHandler").finish()
    }
}

impl Default for StreamingStdOutCallbackHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl StreamingStdOutCallbackHandler {
    pub fn new() -> Self {
        Self {
            writer: Arc::new(Mutex::new(Box::new(io::stdout()))),
        }
    }

    pub fn with_writer(writer: Arc<Mutex<Box<dyn Write + Send>>>) -> Self {
        Self { writer }
    }
}

impl LLMManagerMixin for StreamingStdOutCallbackHandler {
    fn on_llm_new_token(
        &self,
        token: &str,
        _run_id: Uuid,
        _parent_run_id: Option<Uuid>,
        _chunk: Option<&serde_json::Value>,
    ) {
        if let Ok(mut w) = self.writer.lock() {
            if let Err(e) = write!(w, "{}", token) {
                tracing::warn!("StreamingStdOutCallbackHandler write error: {e}");
            }
            if let Err(e) = w.flush() {
                tracing::warn!("StreamingStdOutCallbackHandler flush error: {e}");
            }
        }
    }
}

impl ChainManagerMixin for StreamingStdOutCallbackHandler {}
impl ToolManagerMixin for StreamingStdOutCallbackHandler {}
impl RetrieverManagerMixin for StreamingStdOutCallbackHandler {}
impl CallbackManagerMixin for StreamingStdOutCallbackHandler {}
impl RunManagerMixin for StreamingStdOutCallbackHandler {}

impl BaseCallbackHandler for StreamingStdOutCallbackHandler {
    fn name(&self) -> &str {
        "StreamingStdOutCallbackHandler"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_handler_creation() {
        let handler = StreamingStdOutCallbackHandler::new();
        assert_eq!(handler.name(), "StreamingStdOutCallbackHandler");
    }
}
