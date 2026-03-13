use futures::stream::BoxStream;
use serde_json::Value;

use crate::error::{Error, Result};
use crate::outputs::ChatGeneration;
use crate::utils::json::{parse_json_markdown, parse_partial_json};

use crate::messages::AnyMessage;

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

pub(crate) fn parse_json_result(text: &str, partial: bool) -> Result<Value> {
    parse_json_markdown(text).or_else(|e| {
        if partial {
            parse_partial_json(text, false)
                .map_err(|e| Error::output_parser_simple(format!("Partial parse failed: {e}")))
        } else {
            Err(Error::output_parser_with_output(
                format!("Invalid json output: {e}"),
                text,
            ))
        }
    })
}

pub(crate) fn parse_json_result_partial(text: &str) -> Result<Option<Value>> {
    match parse_json_markdown(text) {
        Ok(v) => Ok(Some(v)),
        Err(_) => match parse_partial_json(text, false) {
            Ok(v) => Ok(Some(v)),
            Err(_) => Ok(None),
        },
    }
}

fn format_schema_instructions(schema: &Value) -> String {
    let mut schema_copy = schema.clone();
    if let Value::Object(ref mut map) = schema_copy {
        map.remove("title");
        map.remove("type");
    }
    let schema_str = serde_json::to_string(&schema_copy).unwrap_or_else(|_| "{}".to_string());
    JSON_FORMAT_INSTRUCTIONS.replace("{schema}", &schema_str)
}

impl BaseOutputParser for JsonOutputParser {
    type Output = Value;

    fn parse(&self, text: &str) -> Result<Value> {
        parse_json_result(text.trim(), false)
    }

    fn parse_result(&self, result: &[ChatGeneration], partial: bool) -> Result<Value> {
        let first = result
            .first()
            .ok_or_else(|| Error::output_parser_simple("No generations to parse"))?;
        parse_json_result(first.message.text().trim(), partial)
    }

    fn get_format_instructions(&self) -> Result<String> {
        Ok(match self.get_schema() {
            Some(schema) => format_schema_instructions(schema),
            None => "Return a JSON object.".to_string(),
        })
    }

    fn parser_type(&self) -> &str {
        "simple_json_output_parser"
    }
}

impl BaseTransformOutputParser for JsonOutputParser {
    fn transform<'a>(
        &'a self,
        input: BoxStream<'a, AnyMessage>,
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

    fn parse_result_partial(&self, result: &[ChatGeneration]) -> Result<Option<Value>> {
        let first = result
            .first()
            .ok_or_else(|| Error::output_parser_simple("No generations to parse"))?;
        parse_json_result_partial(first.message.text().trim())
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
        let msg = crate::messages::AIMessage::builder()
            .content(r#"{"key": "val"#)
            .build();
        let generations = vec![ChatGeneration::builder().message(msg.into()).build()];
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
