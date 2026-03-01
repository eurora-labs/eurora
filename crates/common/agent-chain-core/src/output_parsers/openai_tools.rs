use std::collections::HashMap;
use std::fmt::Debug;

use serde::de::DeserializeOwned;
use serde_json::Value;

use super::base::OutputParserError;
use crate::error::{Error, Result};
use crate::messages::AIMessage;
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
) -> Result<Option<Value>> {
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
            .ok_or_else(|| OutputParserError::new("Tool call arguments is not a string"))?;

        if strict {
            serde_json::from_str::<Value>(args_str).map_err(|e| {
                Error::from(OutputParserError::new(format!(
                    "Function {} arguments:\n\n{}\n\nare not valid JSON. Received JSONDecodeError {}",
                    name, args_str, e
                )))
            })?
        } else {
            parse_partial_json(args_str, false).map_err(|e| {
                Error::from(OutputParserError::new(format!(
                    "Function {} arguments:\n\n{}\n\nare not valid JSON. Received JSONDecodeError {:?}",
                    name, args_str, e
                )))
            })?
        }
    };

    let args = match function_args {
        Value::Null => Value::Object(serde_json::Map::new()),
        other => other,
    };

    let mut parsed = serde_json::Map::new();
    parsed.insert("name".to_string(), Value::String(name.to_string()));
    parsed.insert("args".to_string(), args);

    if return_id {
        let id = raw_tool_call.get("id").cloned().unwrap_or(Value::Null);
        parsed.insert("id".to_string(), id);
    }

    Ok(Some(Value::Object(parsed)))
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
) -> Result<Vec<Value>> {
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
        return Err(OutputParserError::new(exceptions.join("\n\n")).into());
    }

    Ok(final_tools)
}

#[derive(Debug, Clone, Default)]
pub struct JsonOutputToolsParser {
    pub strict: bool,
    pub return_id: bool,
    pub first_tool_only: bool,
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

    pub fn parse_result(&self, result: &[ChatGeneration], partial: bool) -> Result<Value> {
        let generation = result.first().ok_or_else(|| {
            OutputParserError::new("This output parser can only be used with a chat generation.")
        })?;

        let message = &generation.message;

        let tool_calls = if !message.tool_calls().is_empty() {
            message
                .tool_calls()
                .iter()
                .map(|tc| {
                    let mut map = serde_json::Map::new();
                    map.insert("name".to_string(), Value::String(tc.name.clone()));
                    map.insert("args".to_string(), tc.args.clone());
                    if self.return_id {
                        map.insert(
                            "id".to_string(),
                            tc.id
                                .as_ref()
                                .map(|id| Value::String(id.clone()))
                                .unwrap_or(Value::Null),
                        );
                    }
                    Value::Object(map)
                })
                .collect()
        } else {
            let raw_tool_calls = message
                .additional_kwargs()
                .and_then(|kwargs| kwargs.get("tool_calls"))
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();

            if raw_tool_calls.is_empty() {
                vec![]
            } else {
                parse_tool_calls(&raw_tool_calls, partial, self.strict, self.return_id)?
            }
        };

        let tool_calls: Vec<Value> = tool_calls
            .into_iter()
            .map(|mut tc| {
                if let Value::Object(ref mut map) = tc
                    && let Some(name) = map.remove("name")
                {
                    map.insert("type".to_string(), name);
                }
                tc
            })
            .collect();

        if self.first_tool_only {
            Ok(tool_calls.into_iter().next().unwrap_or(Value::Null))
        } else {
            Ok(Value::Array(tool_calls))
        }
    }
}

#[async_trait::async_trait]
impl Runnable for JsonOutputToolsParser {
    type Input = AIMessage;
    type Output = Value;

    fn invoke(&self, input: Self::Input, _config: Option<RunnableConfig>) -> Result<Self::Output> {
        let message = crate::messages::BaseMessage::AI(input);
        let generation = ChatGeneration::new(message);
        self.parse_result(&[generation], false)
    }
}

#[derive(Debug, Clone)]
pub struct JsonOutputKeyToolsParser {
    pub key_name: String,
    pub strict: bool,
    pub return_id: bool,
    pub first_tool_only: bool,
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

    pub fn parse_result(&self, result: &[ChatGeneration], partial: bool) -> Result<Value> {
        let generation = result.first().ok_or_else(|| {
            OutputParserError::new("This output parser can only be used with a chat generation.")
        })?;

        let message = &generation.message;

        let parsed_tool_calls = if !message.tool_calls().is_empty() {
            message
                .tool_calls()
                .iter()
                .map(|tc| {
                    let mut map = serde_json::Map::new();
                    map.insert("name".to_string(), Value::String(tc.name.clone()));
                    map.insert("args".to_string(), tc.args.clone());
                    if self.return_id {
                        map.insert(
                            "id".to_string(),
                            tc.id
                                .as_ref()
                                .map(|id| Value::String(id.clone()))
                                .unwrap_or(Value::Null),
                        );
                    }
                    Value::Object(map)
                })
                .collect()
        } else {
            let raw_tool_calls = message
                .additional_kwargs()
                .and_then(|kwargs| kwargs.get("tool_calls"))
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();

            if raw_tool_calls.is_empty() {
                vec![]
            } else {
                parse_tool_calls(&raw_tool_calls, partial, self.strict, self.return_id)?
            }
        };

        let parsed_tool_calls: Vec<Value> = parsed_tool_calls
            .into_iter()
            .map(|mut tc| {
                if let Value::Object(ref mut map) = tc
                    && let Some(name) = map.remove("name")
                {
                    map.insert("type".to_string(), name);
                }
                tc
            })
            .collect();

        if self.first_tool_only {
            let matching: Vec<&Value> = parsed_tool_calls
                .iter()
                .filter(|tc| {
                    tc.get("type")
                        .and_then(|t| t.as_str())
                        .map(|t| t == self.key_name)
                        .unwrap_or(false)
                })
                .collect();

            let single_result = matching.first().cloned();

            if self.return_id {
                Ok(single_result.cloned().unwrap_or(Value::Null))
            } else if let Some(result) = single_result {
                Ok(result.get("args").cloned().unwrap_or(Value::Null))
            } else {
                Ok(Value::Null)
            }
        } else if self.return_id {
            let filtered: Vec<Value> = parsed_tool_calls
                .into_iter()
                .filter(|tc| {
                    tc.get("type")
                        .and_then(|t| t.as_str())
                        .map(|t| t == self.key_name)
                        .unwrap_or(false)
                })
                .collect();
            Ok(Value::Array(filtered))
        } else {
            let filtered: Vec<Value> = parsed_tool_calls
                .iter()
                .filter(|tc| {
                    tc.get("type")
                        .and_then(|t| t.as_str())
                        .map(|t| t == self.key_name)
                        .unwrap_or(false)
                })
                .filter_map(|tc| tc.get("args").cloned())
                .collect();
            Ok(Value::Array(filtered))
        }
    }
}

#[async_trait::async_trait]
impl Runnable for JsonOutputKeyToolsParser {
    type Input = AIMessage;
    type Output = Value;

    fn invoke(&self, input: Self::Input, _config: Option<RunnableConfig>) -> Result<Self::Output> {
        let message = crate::messages::BaseMessage::AI(input);
        let generation = ChatGeneration::new(message);
        self.parse_result(&[generation], false)
    }
}

#[derive(Clone)]
pub struct PydanticToolsParser {
    name_dict: HashMap<String, DeserializerFn>,
    pub first_tool_only: bool,
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
                    Error::from(OutputParserError::new(format!(
                        "Tool arguments validation failed: {}",
                        e
                    )))
                })?;
                serde_json::to_value(deserialized)
                    .map_err(|e| Error::Other(format!("Failed to serialize: {}", e)))
            }),
        );
        self
    }

    pub fn parse_result(&self, result: &[ChatGeneration], partial: bool) -> Result<Value> {
        let json_results = self.inner.parse_result(result, partial)?;

        if json_results.is_null()
            || (json_results.is_array() && json_results.as_array().is_some_and(|a| a.is_empty()))
        {
            if self.first_tool_only {
                return Ok(Value::Null);
            }
            return Ok(Value::Array(vec![]));
        }

        let items: Vec<&Value> = match json_results.as_array() {
            Some(arr) => arr.iter().collect(),
            None => return Err(OutputParserError::new("Expected array of tool calls").into()),
        };

        let mut pydantic_objects = Vec::new();

        for res in items {
            let args = res.get("args");
            let type_name = res.get("type").and_then(|t| t.as_str()).unwrap_or("");

            let args = match args {
                Some(Value::Object(_)) => args,
                Some(_) if partial => continue,
                Some(other) => {
                    return Err(OutputParserError::new(format!(
                        "Tool arguments must be specified as a dict, received: {}",
                        other
                    ))
                    .into());
                }
                None if partial => continue,
                None => {
                    return Err(OutputParserError::new("Tool call missing 'args' field").into());
                }
            };

            let args = args
                .cloned()
                .unwrap_or(Value::Object(serde_json::Map::new()));

            if let Some(deserializer) = self.name_dict.get(type_name) {
                match deserializer(&args) {
                    Ok(validated) => pydantic_objects.push(validated),
                    Err(_) if partial => continue,
                    Err(e) => return Err(e),
                }
            } else if partial {
                continue;
            } else {
                return Err(
                    OutputParserError::new(format!("Unknown tool type: {}", type_name)).into(),
                );
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
        let message = crate::messages::BaseMessage::AI(input);
        let generation = ChatGeneration::new(message);
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
        assert_eq!(result["name"], "myTool");
        assert_eq!(result["args"]["param"], "value");
        assert_eq!(result["id"], "call_456");
    }

    #[test]
    fn test_parse_tool_call_no_function_key() {
        let raw = serde_json::json!({"id": "call_123"});
        let result = parse_tool_call(&raw, false, false, true).unwrap();
        assert!(result.is_none());
    }
}
