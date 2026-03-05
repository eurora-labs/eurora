use std::fmt::Debug;
use std::marker::PhantomData;

use futures::stream::BoxStream;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::error::{Error, Result};
use crate::outputs::Generation;

use super::base::{BaseOutputParser, ParserInput};
use super::json::parse_json_result;
use super::transform::{BaseCumulativeTransformOutputParser, BaseTransformOutputParser};

#[derive(Debug, Clone)]
pub struct PydanticOutputParser<T> {
    name: String,
    schema: Value,
    _phantom: PhantomData<T>,
}

impl<T: DeserializeOwned + Send + Sync + Clone + Debug + PartialEq> PydanticOutputParser<T> {
    pub fn new(name: impl Into<String>, schema: Value) -> Self {
        Self {
            name: name.into(),
            schema,
            _phantom: PhantomData,
        }
    }

    pub fn parse_obj(&self, obj: &Value) -> Result<T> {
        serde_json::from_value::<T>(obj.clone()).map_err(|e| {
            let json_string = serde_json::to_string(obj).unwrap_or_default();
            Error::output_parser_with_output(
                format!(
                    "Failed to parse {} from completion {json_string}. Got: {e}",
                    self.name
                ),
                json_string,
            )
        })
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
        let json_object = parse_json_result(text.trim(), false)?;
        self.parse_obj(&json_object)
    }

    fn parse_result(&self, result: &[Generation], partial: bool) -> Result<T> {
        let first = result
            .first()
            .ok_or_else(|| Error::output_parser_simple("No generations to parse"))?;
        let json_object = parse_json_result(first.text.trim(), partial)?;
        if partial {
            self.parse_obj(&json_object)
                .map_err(|_| Error::output_parser_simple("Partial parse: validation failed"))
        } else {
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
        Ok(PYDANTIC_FORMAT_INSTRUCTIONS.replace("{schema}", &schema_str))
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
        input: BoxStream<'a, ParserInput>,
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

const PYDANTIC_FORMAT_INSTRUCTIONS: &str = r#"The output should be formatted as a JSON instance that conforms to the JSON schema below.

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
        use futures::StreamExt;

        let parser = person_parser();
        let inputs: Vec<ParserInput> = vec![
            ParserInput::from("{\"name\":"),
            ParserInput::from(" \"Alice\", "),
            ParserInput::from("\"age\": 30}"),
        ];
        let stream = futures::stream::iter(inputs);
        let boxed: BoxStream<ParserInput> = Box::pin(stream);
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
