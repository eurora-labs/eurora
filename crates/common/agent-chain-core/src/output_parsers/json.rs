use std::fmt::Debug;

use futures::stream::BoxStream;
use serde_json::Value;

use crate::error::{Error, Result};
use crate::messages::BaseMessage;
use crate::outputs::Generation;
use crate::utils::json::{parse_json_markdown, parse_partial_json};

use super::base::BaseOutputParser;
use super::format_instructions::JSON_FORMAT_INSTRUCTIONS;
use super::transform::{BaseCumulativeTransformOutputParser, BaseTransformOutputParser};

#[derive(Debug, Clone, Default)]
pub struct JsonOutputParser {
    schema: Option<Value>,
    diff: bool,
}

#[bon::bon]
impl JsonOutputParser {
    #[builder]
    pub fn new(schema: Option<Value>, #[builder(default)] diff: bool) -> Self {
        Self { schema, diff }
    }

    pub fn with_schema(schema: Value) -> Self {
        Self::builder().schema(schema).build()
    }

    pub fn get_schema(&self) -> Option<&Value> {
        self.schema.as_ref()
    }
}

impl BaseOutputParser for JsonOutputParser {
    type Output = Value;

    fn parse(&self, text: &str) -> Result<Value> {
        let text = text.trim();

        match parse_json_markdown(text) {
            Ok(value) => Ok(value),
            Err(e) => Err(Error::Other(format!(
                "Invalid json output: {}. Error: {}",
                text, e
            ))),
        }
    }

    fn parse_result(&self, result: &[Generation], partial: bool) -> Result<Value> {
        if result.is_empty() {
            return Err(Error::Other("No generations to parse".to_string()));
        }

        let text = result[0].text.trim();

        if partial {
            match parse_json_markdown(text) {
                Ok(value) => Ok(value),
                Err(_) => parse_partial_json(text, false)
                    .map_err(|e| Error::Other(format!("Partial parse failed: {}", e))),
            }
        } else {
            match parse_json_markdown(text) {
                Ok(value) => Ok(value),
                Err(e) => Err(Error::output_parser_with_output(
                    format!("Invalid json output: {}", e),
                    text,
                )),
            }
        }
    }

    fn get_format_instructions(&self) -> Result<String> {
        match self.get_schema() {
            Some(schema) => {
                let mut schema_copy = schema.clone();

                if let Value::Object(ref mut map) = schema_copy {
                    map.remove("title");
                    map.remove("type");
                }

                let schema_str =
                    serde_json::to_string(&schema_copy).unwrap_or_else(|_| "{}".to_string());

                Ok(JSON_FORMAT_INSTRUCTIONS.replace("{schema}", &schema_str))
            }
            None => Ok("Return a JSON object.".to_string()),
        }
    }

    fn parser_type(&self) -> &str {
        "simple_json_output_parser"
    }
}

impl BaseTransformOutputParser for JsonOutputParser {
    fn transform<'a>(
        &'a self,
        input: BoxStream<'a, BaseMessage>,
    ) -> BoxStream<'a, Result<Self::Output>>
    where
        Self::Output: 'a,
    {
        self.cumulative_transform(input, None)
    }

    fn atransform<'a>(
        &'a self,
        input: BoxStream<'a, BaseMessage>,
    ) -> BoxStream<'a, Result<Self::Output>>
    where
        Self::Output: 'a,
    {
        self.cumulative_transform(input, None)
    }
}

impl BaseCumulativeTransformOutputParser for JsonOutputParser {
    fn diff_mode(&self) -> bool {
        self.diff
    }

    fn compute_diff(&self, prev: Option<&Value>, next: Value) -> Result<Value> {
        Ok(match prev {
            Some(prev_value) => {
                let patch = json_patch::diff(prev_value, &next);
                serde_json::to_value(&patch).unwrap_or_default()
            }
            None => Value::Array(vec![serde_json::json!({
                "op": "replace",
                "path": "",
                "value": next,
            })]),
        })
    }
}

pub type SimpleJsonOutputParser = JsonOutputParser;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_output_parser_simple() {
        let parser = JsonOutputParser::builder().build();
        let result = parser.parse(r#"{"key": "value"}"#).unwrap();
        assert_eq!(result["key"], "value");
    }

    #[test]
    fn test_json_output_parser_markdown() {
        let parser = JsonOutputParser::builder().build();
        let result = parser
            .parse(
                r#"```json
{"key": "value"}
```"#,
            )
            .unwrap();
        assert_eq!(result["key"], "value");
    }

    #[test]
    fn test_json_output_parser_array() {
        let parser = JsonOutputParser::builder().build();
        let result = parser.parse(r#"[1, 2, 3]"#).unwrap();
        assert!(result.is_array());
        assert_eq!(result[0], 1);
    }

    #[test]
    fn test_json_output_parser_nested() {
        let parser = JsonOutputParser::builder().build();
        let result = parser.parse(r#"{"outer": {"inner": "value"}}"#).unwrap();
        assert_eq!(result["outer"]["inner"], "value");
    }

    #[test]
    fn test_json_output_parser_invalid() {
        let parser = JsonOutputParser::builder().build();
        let result = parser.parse("not json");
        assert!(result.is_err());
    }

    #[test]
    fn test_json_output_parser_format_instructions_no_schema() {
        let parser = JsonOutputParser::builder().build();
        let instructions = parser
            .get_format_instructions()
            .expect("should return format instructions");
        assert_eq!(instructions, "Return a JSON object.");
    }

    #[test]
    fn test_json_output_parser_format_instructions_with_schema() {
        let schema = serde_json::json!({
            "title": "Person",
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer"}
            }
        });
        let parser = JsonOutputParser::with_schema(schema);
        let instructions = parser
            .get_format_instructions()
            .expect("should return format instructions");
        assert!(instructions.contains("properties"));
        assert!(instructions.contains("name"));
    }

    #[test]
    fn test_json_output_parser_partial() {
        let parser = JsonOutputParser::builder().build();
        let generations = vec![Generation::builder().text(r#"{"key": "val"#).build()];
        let result = parser.parse_result(&generations, true).unwrap();
        assert_eq!(result["key"], "val");
    }

    #[test]
    fn test_json_diff() {
        let prev = serde_json::json!({"a": 1, "b": 2});
        let next = serde_json::json!({"a": 1, "b": 3, "c": 4});
        let patch = json_patch::diff(&prev, &next);
        let diff = serde_json::to_value(&patch).unwrap();

        assert!(diff.is_array());
        let patches = diff.as_array().unwrap();
        assert!(!patches.is_empty());
    }

    #[test]
    fn test_parser_type() {
        let parser = JsonOutputParser::builder().build();
        assert_eq!(parser.parser_type(), "simple_json_output_parser");
    }
}
