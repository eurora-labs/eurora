use std::collections::HashMap;
use std::fmt::Debug;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use bon::bon;
use serde_json::Value;

use crate::callbacks::Callbacks;
use crate::callbacks::manager::CallbackManagerForToolRun;
use crate::error::{Error, Result};
use crate::runnables::RunnableConfig;

use super::base::{
    ArgsSchema, BaseTool, ErrorHandler, ResponseFormat, ToolInput, ToolMeta, ToolOutput,
};

pub type ToolFunc = Arc<dyn Fn(String) -> Result<String> + Send + Sync>;

pub type AsyncToolFunc =
    Arc<dyn Fn(String) -> Pin<Box<dyn Future<Output = Result<String>> + Send>> + Send + Sync>;

pub struct Tool {
    meta: ToolMeta,
    func: Option<ToolFunc>,
    coroutine: Option<AsyncToolFunc>,
    args_schema: Option<ArgsSchema>,
}

impl Debug for Tool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Tool").field("meta", &self.meta).finish()
    }
}

#[bon]
impl Tool {
    #[builder]
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        func: Option<ToolFunc>,
        coroutine: Option<AsyncToolFunc>,
        args_schema: Option<ArgsSchema>,
        #[builder(default)] return_direct: bool,
        #[builder(default)] verbose: bool,
        #[builder(default)] handle_tool_error: ErrorHandler,
        #[builder(default)] handle_validation_error: ErrorHandler,
        #[builder(default)] response_format: ResponseFormat,
        tags: Option<Vec<String>>,
        metadata: Option<HashMap<String, Value>>,
        extras: Option<HashMap<String, Value>>,
        callbacks: Option<Callbacks>,
    ) -> Self {
        Self {
            meta: ToolMeta {
                name: name.into(),
                description: description.into(),
                return_direct,
                verbose,
                handle_tool_error,
                handle_validation_error,
                response_format,
                tags,
                metadata,
                extras,
                callbacks,
            },
            func,
            coroutine,
            args_schema,
        }
    }

    pub fn from_function<F>(
        func: F,
        name: impl Into<String>,
        description: impl Into<String>,
    ) -> Self
    where
        F: Fn(String) -> Result<String> + Send + Sync + 'static,
    {
        Self::builder()
            .name(name)
            .description(description)
            .func(Arc::new(func))
            .build()
    }

    pub fn from_function_with_async<F, AF, Fut>(
        func: F,
        coroutine: AF,
        name: impl Into<String>,
        description: impl Into<String>,
    ) -> Self
    where
        F: Fn(String) -> Result<String> + Send + Sync + 'static,
        AF: Fn(String) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<String>> + Send + 'static,
    {
        Self::builder()
            .name(name)
            .description(description)
            .func(Arc::new(func))
            .coroutine(Arc::new(move |input| Box::pin(coroutine(input))))
            .build()
    }

    fn extract_single_input(&self, input: ToolInput) -> Result<String> {
        match input {
            ToolInput::String(s) => Ok(s),
            ToolInput::Dict(d) => self.extract_single_value_from_map(d),
            ToolInput::ToolCall(tc) => match tc.args {
                Value::Object(obj) => {
                    let map: HashMap<String, Value> = obj.into_iter().collect();
                    self.extract_single_value_from_map(map)
                }
                Value::String(s) => Ok(s),
                other => Ok(other.to_string()),
            },
        }
    }

    fn extract_single_value_from_map(&self, map: HashMap<String, Value>) -> Result<String> {
        if map.len() != 1 {
            return Err(Error::ToolInvocation(format!(
                "Too many arguments to single-input tool {}. \
                 Consider using StructuredTool instead.",
                self.meta.name,
            )));
        }
        let value = map.into_values().next().expect("checked len == 1");
        Ok(match value {
            Value::String(s) => s,
            other => other.to_string(),
        })
    }
}

#[async_trait]
impl BaseTool for Tool {
    impl_base_tool_getters!();

    fn args_schema(&self) -> Option<&ArgsSchema> {
        self.args_schema.as_ref()
    }

    fn args(&self) -> HashMap<String, Value> {
        if let Some(schema) = &self.args_schema {
            return schema.properties();
        }
        HashMap::from([(
            "tool_input".to_string(),
            serde_json::json!({"type": "string"}),
        )])
    }

    async fn tool_run(
        &self,
        input: ToolInput,
        _run_manager: Option<&CallbackManagerForToolRun>,
        _config: &RunnableConfig,
    ) -> Result<ToolOutput> {
        if let Some(coroutine) = &self.coroutine {
            let string_input = self.extract_single_input(input)?;
            coroutine(string_input).await.map(ToolOutput::String)
        } else {
            let string_input = self.extract_single_input(input)?;
            let func = self.func.as_ref().ok_or_else(|| {
                Error::ToolInvocation("Tool has no function or coroutine.".to_string())
            })?;
            func(string_input).map(ToolOutput::String)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_creation() {
        let tool = Tool::from_function(
            |input| Ok(format!("Echo: {}", input)),
            "echo",
            "Echoes the input",
        );

        assert_eq!(tool.name(), "echo");
        assert_eq!(tool.description(), "Echoes the input");
    }

    #[tokio::test]
    async fn test_tool_run() {
        let tool = Tool::from_function(
            |input| Ok(format!("Hello, {}!", input)),
            "greet",
            "Greets the user",
        );

        let result = tool
            .run(ToolInput::String("World".to_string()), None, None)
            .await
            .unwrap();
        match result {
            ToolOutput::String(s) => assert_eq!(s, "Hello, World!"),
            _ => panic!("Expected String output"),
        }
    }

    #[tokio::test]
    async fn test_tool_run_with_dict() {
        let tool = Tool::from_function(
            |input| Ok(format!("Got: {}", input)),
            "process",
            "Processes input",
        );

        let mut dict = HashMap::new();
        dict.insert("query".to_string(), Value::String("test".to_string()));

        let result = tool.run(ToolInput::Dict(dict), None, None).await.unwrap();
        match result {
            ToolOutput::String(s) => assert_eq!(s, "Got: test"),
            _ => panic!("Expected String output"),
        }
    }

    #[test]
    fn test_tool_args() {
        let tool = Tool::from_function(Ok, "identity", "Returns input unchanged");

        let args = tool.args();
        assert!(args.contains_key("tool_input"));
    }

    #[tokio::test]
    async fn test_tool_run_async() {
        let tool = Tool::from_function(
            |input| Ok(format!("Sync: {}", input)),
            "sync_tool",
            "A sync tool",
        );

        let result = tool
            .run(ToolInput::String("test".to_string()), None, None)
            .await
            .unwrap();
        match result {
            ToolOutput::String(s) => assert_eq!(s, "Sync: test"),
            _ => panic!("Expected String output"),
        }
    }
}
