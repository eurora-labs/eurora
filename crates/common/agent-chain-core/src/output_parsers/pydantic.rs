use std::fmt::Debug;
use std::marker::PhantomData;

use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::error::{Error, Result};
use crate::outputs::Generation;
use crate::utils::json::parse_json_markdown;

use super::base::{BaseOutputParser, OutputParserError};
use super::transform::{BaseCumulativeTransformOutputParser, BaseTransformOutputParser};

use futures::stream::BoxStream;

use crate::messages::BaseMessage;
use crate::utils::json::parse_partial_json;

#[derive(Debug, Clone)]
pub struct PydanticOutputParser<T> {
    name: String,
    schema: Value,
    _marker: PhantomData<T>,
}

impl<T: DeserializeOwned + Send + Sync + Clone + Debug + PartialEq> PydanticOutputParser<T> {
    pub fn new(name: impl Into<String>, schema: Value) -> Self {
        Self {
            name: name.into(),
            schema,
            _marker: PhantomData,
        }
    }

    pub fn parse_obj(&self, obj: &Value) -> Result<T> {
        serde_json::from_value::<T>(obj.clone()).map_err(|e| self.parser_exception(&e, obj))
    }

    pub fn parser_exception(&self, error: &dyn std::fmt::Display, json_object: &Value) -> Error {
        let json_string = serde_json::to_string(json_object).unwrap_or_default();
        let message = format!(
            "Failed to parse {} from completion {}. Got: {}",
            self.name, json_string, error
        );
        OutputParserError::parse_error(message, json_string).into()
    }

    pub fn get_schema(&self) -> &Value {
        &self.schema
    }

    pub fn output_type_name(&self) -> &str {
        &self.name
    }
}

impl<T: DeserializeOwned + Send + Sync + Clone + Debug + PartialEq> BaseOutputParser
    for PydanticOutputParser<T>
{
    type Output = T;

    fn parse(&self, text: &str) -> Result<T> {
        let text = text.trim();
        let json_object = parse_json_markdown(text).map_err(|e| {
            let message = format!("Invalid json output: {}. Error: {}", text, e);
            Error::from(OutputParserError::parse_error(&message, text))
        })?;
        self.parse_obj(&json_object)
    }

    fn parse_result(&self, result: &[Generation], partial: bool) -> Result<T> {
        if result.is_empty() {
            return Err(Error::Other("No generations to parse".to_string()));
        }

        let text = result[0].text.trim();

        if partial {
            let json_object = match parse_json_markdown(text) {
                Ok(value) => value,
                Err(_) => match parse_partial_json(text, false) {
                    Ok(value) => value,
                    Err(e) => {
                        return Err(Error::Other(format!("Partial parse failed: {}", e)));
                    }
                },
            };
            self.parse_obj(&json_object)
                .map_err(|_| Error::Other("Partial parse: validation failed".to_string()))
        } else {
            let json_object = match parse_json_markdown(text) {
                Ok(value) => value,
                Err(e) => {
                    return Err(OutputParserError::parse_error(
                        format!("Invalid json output: {}", e),
                        text,
                    )
                    .into());
                }
            };
            self.parse_obj(&json_object)
        }
    }

    fn get_format_instructions(&self) -> Result<String> {
        let mut schema_copy = self.schema.clone();

        if let Value::Object(ref mut map) = schema_copy {
            map.remove("title");
            map.remove("type");
        }

        let schema_str = serde_json::to_string(&schema_copy).unwrap_or_else(|_| "{}".to_string());
        Ok(_PYDANTIC_FORMAT_INSTRUCTIONS.replace("{schema}", &schema_str))
    }

    fn parser_type(&self) -> &str {
        "pydantic"
    }
}

impl<T: DeserializeOwned + Send + Sync + Clone + Debug + PartialEq + 'static>
    BaseTransformOutputParser for PydanticOutputParser<T>
{
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

impl<T: DeserializeOwned + Send + Sync + Clone + Debug + PartialEq + 'static>
    BaseCumulativeTransformOutputParser for PydanticOutputParser<T>
{
}

const _PYDANTIC_FORMAT_INSTRUCTIONS: &str = r#"The output should be formatted as a JSON instance that conforms to the JSON schema below.

As an example, for the schema {{"properties": {{"foo": {{"title": "Foo", "description": "a list of strings", "type": "array", "items": {{"type": "string"}}}}}}, "required": ["foo"]}}
the object {{"foo": ["bar", "baz"]}} is a well-formatted instance of the schema. The object {{"properties": {{"foo": ["bar", "baz"]}}}} is not well-formatted.

Here is the output schema:
```
{schema}
```"#;

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, Clone, Deserialize, PartialEq)]
    struct Person {
        name: String,
        age: i64,
    }

    fn person_parser() -> PydanticOutputParser<Person> {
        PydanticOutputParser::<Person>::new(
            "Person",
            serde_json::json!({
                "properties": {
                    "name": {"type": "string"},
                    "age": {"type": "integer"}
                },
                "required": ["name", "age"]
            }),
        )
    }

    #[test]
    fn test_pydantic_parser_simple() {
        let parser = person_parser();
        let result = parser.parse(r#"{"name": "Alice", "age": 30}"#).unwrap();
        assert_eq!(result.name, "Alice");
        assert_eq!(result.age, 30);
    }

    #[test]
    fn test_pydantic_parser_markdown() {
        let parser = person_parser();
        let input = "```json\n{\"name\": \"Bob\", \"age\": 25}\n```";
        let result = parser.parse(input).unwrap();
        assert_eq!(result.name, "Bob");
        assert_eq!(result.age, 25);
    }

    #[test]
    fn test_pydantic_parser_invalid_json() {
        let parser = person_parser();
        let result = parser.parse("not json");
        assert!(result.is_err());
    }

    #[test]
    fn test_pydantic_parser_wrong_type() {
        let parser = person_parser();
        let result = parser.parse(r#"{"name": "Alice", "age": "not a number"}"#);
        assert!(result.is_err());
    }

    #[test]
    fn test_pydantic_parse_result_partial() {
        let parser = person_parser();
        let generations = vec![Generation::builder().text(r#"{"name": "Alice"#).build()];
        let result = parser.parse_result(&generations, true);
        assert!(result.is_err());
    }

    #[test]
    fn test_pydantic_parse_result_partial_complete() {
        let parser = person_parser();
        let generations = vec![
            Generation::builder()
                .text(r#"{"name": "Alice", "age": 30}"#)
                .build(),
        ];
        let result = parser.parse_result(&generations, true).unwrap();
        assert_eq!(result.name, "Alice");
        assert_eq!(result.age, 30);
    }

    #[test]
    fn test_pydantic_format_instructions() {
        let parser = person_parser();
        let instructions = parser.get_format_instructions().unwrap();
        assert!(instructions.contains("JSON"));
        assert!(instructions.contains("name"));
        assert!(instructions.contains("age"));
    }

    #[test]
    fn test_pydantic_parser_type() {
        let parser = person_parser();
        assert_eq!(parser.parser_type(), "pydantic");
    }

    #[tokio::test]
    async fn test_pydantic_cumulative_transform() {
        use crate::messages::HumanMessage;
        use futures::StreamExt;

        let parser = person_parser();
        let messages: Vec<BaseMessage> = vec![
            BaseMessage::Human(HumanMessage::builder().content("{\"name\":").build()),
            BaseMessage::Human(HumanMessage::builder().content(" \"Alice\", ").build()),
            BaseMessage::Human(HumanMessage::builder().content("\"age\": 30}").build()),
        ];
        let stream = futures::stream::iter(messages);
        let boxed: BoxStream<BaseMessage> = Box::pin(stream);
        let mut output_stream = parser.transform(boxed);

        let mut results = Vec::new();
        while let Some(result) = output_stream.next().await {
            if let Ok(person) = result {
                results.push(person);
            }
        }
        assert!(!results.is_empty());
        let last = results.last().unwrap();
        assert_eq!(last.name, "Alice");
        assert_eq!(last.age, 30);
    }

    #[test]
    fn test_pydantic_output_type_name() {
        let parser = person_parser();
        assert_eq!(parser.output_type_name(), "Person");
    }
}
