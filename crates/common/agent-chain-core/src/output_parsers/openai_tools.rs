use std::collections::HashMap;
use std::fmt::Debug;

use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::error::{Error, Result};
use crate::messages::ToolCall;
use crate::messages::{AIMessage, BaseMessage};
use crate::messages::{InvalidToolCall, invalid_tool_call};
use crate::outputs::ChatGeneration;
use crate::runnables::base::Runnable;
use crate::runnables::config::RunnableConfig;
use crate::utils::json::parse_partial_json;

pub fn parse_tool_call(
    raw_tool_call: &Value,
    partial: bool,
    strict: bool,
    return_id: bool,
) -> Result<Option<ToolCall>> {
    let function = match raw_tool_call.get("function") {
        Some(f) => f,
        None => return Ok(None),
    };

    let arguments = function.get("arguments");
    let name = function.get("name").and_then(|n| n.as_str()).unwrap_or("");

    let function_args = if partial {
        let args_str = match arguments.and_then(|a| a.as_str()) {
            Some(s) => s,
            None => return Ok(None),
        };
        match parse_partial_json(args_str, strict) {
            Ok(v) => v,
            Err(_) => return Ok(None),
        }
    } else if arguments.is_none()
        || arguments == Some(&Value::Null)
        || arguments.and_then(|a| a.as_str()) == Some("")
    {
        Value::Object(serde_json::Map::new())
    } else {
        let args_str = arguments
            .and_then(|a| a.as_str())
            .ok_or_else(|| Error::output_parser_simple("Tool call arguments is not a string"))?;

        if strict {
            serde_json::from_str::<Value>(args_str).map_err(|e| {
                Error::output_parser_simple(format!(
                    "Function {} arguments:\n\n{}\n\nare not valid JSON. Received JSONDecodeError {}",
                    name, args_str, e
                ))
            })?
        } else {
            parse_partial_json(args_str, false).map_err(|e| {
                Error::output_parser_simple(format!(
                    "Function {} arguments:\n\n{}\n\nare not valid JSON. Received JSONDecodeError {:?}",
                    name, args_str, e
                ))
            })?
        }
    };

    let args = match function_args {
        Value::Null => Value::Object(serde_json::Map::new()),
        other => other,
    };

    let id = if return_id {
        raw_tool_call
            .get("id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    } else {
        None
    };

    Ok(Some(
        ToolCall::builder()
            .name(name)
            .args(args)
            .maybe_id(id)
            .build(),
    ))
}

pub fn make_invalid_tool_call(raw_tool_call: &Value, error_msg: Option<&str>) -> InvalidToolCall {
    let function = raw_tool_call.get("function");

    let name = function
        .and_then(|f| f.get("name"))
        .and_then(|n| n.as_str())
        .map(|s| s.to_string());

    let args = function
        .and_then(|f| f.get("arguments"))
        .and_then(|a| a.as_str())
        .map(|s| s.to_string());

    let id = raw_tool_call
        .get("id")
        .and_then(|i| i.as_str())
        .map(|s| s.to_string());

    invalid_tool_call(name, args, id, error_msg.map(|s| s.to_string()))
}

pub fn parse_tool_calls(
    raw_tool_calls: &[Value],
    partial: bool,
    strict: bool,
    return_id: bool,
) -> Result<Vec<ToolCall>> {
    let mut final_tools = Vec::new();
    let mut exceptions = Vec::new();

    for tool_call in raw_tool_calls {
        match parse_tool_call(tool_call, partial, strict, return_id) {
            Ok(Some(parsed)) => final_tools.push(parsed),
            Ok(None) => {}
            Err(e) => {
                exceptions.push(e.to_string());
            }
        }
    }

    if !exceptions.is_empty() {
        return Err(Error::output_parser_simple(exceptions.join("\n\n")));
    }

    Ok(final_tools)
}

fn extract_tool_calls_from_generation(
    generation: &ChatGeneration,
    partial: bool,
    strict: bool,
    return_id: bool,
) -> Result<Vec<ToolCall>> {
    let message = &generation.message;

    let tool_calls = if !message.tool_calls().is_empty() {
        message
            .tool_calls()
            .iter()
            .map(|tc| {
                let id = if return_id { tc.id.clone() } else { None };
                ToolCall::builder()
                    .name(&tc.name)
                    .args(tc.args.clone())
                    .maybe_id(id)
                    .build()
            })
            .collect()
    } else {
        let raw_tool_calls = message
            .additional_kwargs()
            .get("tool_calls")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        if raw_tool_calls.is_empty() {
            vec![]
        } else {
            parse_tool_calls(&raw_tool_calls, partial, strict, return_id)?
        }
    };

    Ok(tool_calls)
}

#[derive(Debug, Clone, Default)]
pub struct JsonOutputToolsParser {
    strict: bool,
    return_id: bool,
    first_tool_only: bool,
}

#[bon::bon]
impl JsonOutputToolsParser {
    #[builder]
    pub fn new(
        #[builder(default)] strict: bool,
        #[builder(default)] return_id: bool,
        #[builder(default)] first_tool_only: bool,
    ) -> Self {
        Self {
            strict,
            return_id,
            first_tool_only,
        }
    }

    pub fn parse_result(&self, result: &[ChatGeneration], partial: bool) -> Result<Vec<ToolCall>> {
        let generation = result.first().ok_or_else(|| {
            Error::output_parser_simple(
                "This output parser can only be used with a chat generation.",
            )
        })?;

        extract_tool_calls_from_generation(generation, partial, self.strict, self.return_id)
    }
}

#[async_trait::async_trait]
impl Runnable for JsonOutputToolsParser {
    type Input = AIMessage;
    type Output = Value;

    fn invoke(&self, input: Self::Input, _config: Option<RunnableConfig>) -> Result<Self::Output> {
        let message = crate::messages::AnyMessage::AIMessage(input);
        let generation = ChatGeneration::builder().message(message).build();
        let tool_calls = self.parse_result(&[generation], false)?;
        if self.first_tool_only {
            let first = tool_calls
                .into_iter()
                .next()
                .map(|tc| serde_json::to_value(tc).unwrap_or(Value::Null))
                .unwrap_or(Value::Null);
            Ok(first)
        } else {
            Ok(serde_json::to_value(tool_calls).unwrap_or(Value::Null))
        }
    }
}

#[derive(Debug, Clone)]
pub struct JsonOutputKeyToolsParser {
    key_name: String,
    strict: bool,
    return_id: bool,
    first_tool_only: bool,
}

#[bon::bon]
impl JsonOutputKeyToolsParser {
    #[builder]
    pub fn new(
        #[builder(into)] key_name: String,
        #[builder(default)] strict: bool,
        #[builder(default)] return_id: bool,
        #[builder(default)] first_tool_only: bool,
    ) -> Self {
        Self {
            key_name,
            strict,
            return_id,
            first_tool_only,
        }
    }

    pub fn parse_result(&self, result: &[ChatGeneration], partial: bool) -> Result<Vec<ToolCall>> {
        let generation = result.first().ok_or_else(|| {
            Error::output_parser_simple(
                "This output parser can only be used with a chat generation.",
            )
        })?;

        let tool_calls =
            extract_tool_calls_from_generation(generation, partial, self.strict, self.return_id)?;

        let matching: Vec<ToolCall> = tool_calls
            .into_iter()
            .filter(|tc| tc.name == self.key_name)
            .collect();

        Ok(matching)
    }
}

#[async_trait::async_trait]
impl Runnable for JsonOutputKeyToolsParser {
    type Input = AIMessage;
    type Output = Value;

    fn invoke(&self, input: Self::Input, _config: Option<RunnableConfig>) -> Result<Self::Output> {
        let message = crate::messages::AnyMessage::AIMessage(input);
        let generation = ChatGeneration::builder().message(message).build();
        let tool_calls = self.parse_result(&[generation], false)?;
        if self.first_tool_only {
            let first = tool_calls
                .into_iter()
                .next()
                .map(|tc| serde_json::to_value(tc).unwrap_or(Value::Null))
                .unwrap_or(Value::Null);
            Ok(first)
        } else {
            Ok(serde_json::to_value(tool_calls).unwrap_or(Value::Null))
        }
    }
}

#[derive(Clone)]
pub struct PydanticToolsParser {
    name_dict: HashMap<String, DeserializerFn>,
    first_tool_only: bool,
    inner: JsonOutputToolsParser,
}

type DeserializerFn = std::sync::Arc<dyn Fn(&Value) -> Result<Value> + Send + Sync>;

impl Debug for PydanticToolsParser {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PydanticToolsParser")
            .field("first_tool_only", &self.first_tool_only)
            .field("tool_names", &self.name_dict.keys().collect::<Vec<_>>())
            .finish()
    }
}

impl PydanticToolsParser {
    pub fn new(tools: Vec<(String, DeserializerFn)>, first_tool_only: bool) -> Self {
        let name_dict = tools.into_iter().collect();
        Self {
            name_dict,
            first_tool_only,
            inner: JsonOutputToolsParser::builder().build(),
        }
    }

    pub fn with_tool<T>(mut self, name: impl Into<String>) -> Self
    where
        T: DeserializeOwned + serde::Serialize + 'static,
    {
        let tool_name = name.into();
        self.name_dict.insert(
            tool_name,
            std::sync::Arc::new(|args: &Value| {
                let deserialized: T = serde_json::from_value(args.clone()).map_err(|e| {
                    Error::output_parser_simple(format!("Tool arguments validation failed: {}", e))
                })?;
                serde_json::to_value(deserialized)
                    .map_err(|e| Error::output_parser_simple(format!("Failed to serialize: {e}")))
            }),
        );
        self
    }

    pub fn parse_result(&self, result: &[ChatGeneration], partial: bool) -> Result<Value> {
        let tool_calls = self.inner.parse_result(result, partial)?;

        if tool_calls.is_empty() {
            if self.first_tool_only {
                return Ok(Value::Null);
            }
            return Ok(Value::Array(vec![]));
        }

        let mut pydantic_objects = Vec::new();

        for tc in &tool_calls {
            let args = &tc.args;
            let type_name = &tc.name;

            let args = match args {
                Value::Object(_) => args,
                _ if partial => continue,
                other => {
                    return Err(Error::output_parser_simple(format!(
                        "Tool arguments must be specified as a dict, received: {}",
                        other
                    )));
                }
            };

            if let Some(deserializer) = self.name_dict.get(type_name.as_str()) {
                match deserializer(args) {
                    Ok(validated) => pydantic_objects.push(validated),
                    Err(_) if partial => continue,
                    Err(e) => return Err(e),
                }
            } else if partial {
                continue;
            } else {
                return Err(Error::output_parser_simple(format!(
                    "Unknown tool type: {}",
                    type_name
                )));
            }
        }

        if self.first_tool_only {
            Ok(pydantic_objects.into_iter().next().unwrap_or(Value::Null))
        } else {
            Ok(Value::Array(pydantic_objects))
        }
    }
}

#[async_trait::async_trait]
impl Runnable for PydanticToolsParser {
    type Input = AIMessage;
    type Output = Value;

    fn invoke(&self, input: Self::Input, _config: Option<RunnableConfig>) -> Result<Self::Output> {
        let message = crate::messages::AnyMessage::AIMessage(input);
        let generation = ChatGeneration::builder().message(message).build();
        self.parse_result(&[generation], false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_tool_call_basic() {
        let raw = serde_json::json!({
            "function": {"arguments": r#"{"param": "value"}"#, "name": "myTool"},
            "id": "call_456",
            "type": "function"
        });

        let result = parse_tool_call(&raw, false, false, true).unwrap().unwrap();
        assert_eq!(result.name, "myTool");
        assert_eq!(result.args["param"], "value");
        assert_eq!(result.id.as_deref(), Some("call_456"));
    }

    #[test]
    fn test_parse_tool_call_no_function_key() {
        let raw = serde_json::json!({"id": "call_123"});
        let result = parse_tool_call(&raw, false, false, true).unwrap();
        assert!(result.is_none());
    }
}
