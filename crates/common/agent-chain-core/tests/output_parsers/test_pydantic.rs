use agent_chain_core::output_parsers::{BaseOutputParser, PydanticOutputParser};
use agent_chain_core::outputs::Generation;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
struct SimpleModel {
    name: String,
    age: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
struct OptionalFieldModel {
    required_field: String,
    optional_field: Option<String>,
    #[serde(default = "default_field_value")]
    default_field: i64,
}

fn default_field_value() -> i64 {
    42
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
struct NestedAddress {
    street: String,
    city: String,
    zip_code: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
struct PersonWithAddress {
    name: String,
    age: i64,
    address: NestedAddress,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
enum ColorEnum {
    Red,
    Green,
    Blue,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
struct WithEnumModel {
    name: String,
    color: ColorEnum,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
struct WithLiteralModel {
    status: Status,
    name: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(try_from = "String")]
enum Status {
    Active,
    Inactive,
    Pending,
}

impl TryFrom<String> for Status {
    type Error = String;

    fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
        match value.as_str() {
            "active" => Ok(Status::Active),
            "inactive" => Ok(Status::Inactive),
            "pending" => Ok(Status::Pending),
            other => Err(format!(
                "Invalid status '{}', expected one of: active, inactive, pending",
                other
            )),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
struct WithListModel {
    tags: Vec<String>,
    #[serde(default)]
    scores: Vec<i64>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
struct DeepNested {
    level3: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
struct MidNested {
    level2: DeepNested,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
struct TopNested {
    level1: MidNested,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
struct UnicodeModel {
    title: String,
    author: String,
}

fn simple_model_parser() -> PydanticOutputParser<SimpleModel> {
    PydanticOutputParser::new(
        "SimpleModel",
        json!({
            "title": "SimpleModel",
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer"}
            },
            "required": ["name", "age"]
        }),
    )
}

fn simple_model_schema() -> Value {
    json!({
        "title": "SimpleModel",
        "type": "object",
        "properties": {
            "name": {"type": "string"},
            "age": {"type": "integer"}
        },
        "required": ["name", "age"]
    })
}

#[test]
fn test_parse_simple_model() {
    let parser = simple_model_parser();
    let result = parser.parse(r#"{"name": "Alice", "age": 30}"#).unwrap();
    assert_eq!(result.name, "Alice");
    assert_eq!(result.age, 30);
}

#[test]
fn test_parse_in_code_block() {
    let parser = simple_model_parser();
    let text = "```json\n{\"name\": \"Bob\", \"age\": 25}\n```";
    let result = parser.parse(text).unwrap();
    assert_eq!(result.name, "Bob");
    assert_eq!(result.age, 25);
}

#[test]
fn test_parse_optional_fields_provided() {
    let parser = PydanticOutputParser::<OptionalFieldModel>::new(
        "OptionalFieldModel",
        json!({
            "title": "OptionalFieldModel",
            "type": "object",
            "properties": {
                "required_field": {"type": "string"},
                "optional_field": {"type": "string"},
                "default_field": {"type": "integer"}
            },
            "required": ["required_field"]
        }),
    );
    let text = r#"{"required_field": "hello", "optional_field": "world", "default_field": 100}"#;
    let result = parser.parse(text).unwrap();
    assert_eq!(result.required_field, "hello");
    assert_eq!(result.optional_field, Some("world".to_string()));
    assert_eq!(result.default_field, 100);
}

#[test]
fn test_parse_optional_fields_omitted() {
    let parser = PydanticOutputParser::<OptionalFieldModel>::new(
        "OptionalFieldModel",
        json!({
            "title": "OptionalFieldModel",
            "type": "object",
            "properties": {
                "required_field": {"type": "string"},
                "optional_field": {"type": "string"},
                "default_field": {"type": "integer"}
            },
            "required": ["required_field"]
        }),
    );
    let text = r#"{"required_field": "hello"}"#;
    let result = parser.parse(text).unwrap();
    assert_eq!(result.required_field, "hello");
    assert_eq!(result.optional_field, None);
    assert_eq!(result.default_field, 42);
}

#[test]
fn test_parse_nested_model() {
    let parser = PydanticOutputParser::<PersonWithAddress>::new(
        "PersonWithAddress",
        json!({
            "title": "PersonWithAddress",
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer"},
                "address": {
                    "type": "object",
                    "properties": {
                        "street": {"type": "string"},
                        "city": {"type": "string"},
                        "zip_code": {"type": "string"}
                    }
                }
            },
            "required": ["name", "age", "address"]
        }),
    );
    let text = serde_json::to_string(&json!({
        "name": "Alice",
        "age": 30,
        "address": {
            "street": "123 Main St",
            "city": "Springfield",
            "zip_code": "12345"
        }
    }))
    .unwrap();
    let result = parser.parse(&text).unwrap();
    assert_eq!(result.name, "Alice");
    assert_eq!(result.address.city, "Springfield");
}

#[test]
fn test_parse_enum_field() {
    let parser = PydanticOutputParser::<WithEnumModel>::new(
        "WithEnumModel",
        json!({
            "title": "WithEnumModel",
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "color": {"type": "string", "enum": ["red", "green", "blue"]}
            },
            "required": ["name", "color"]
        }),
    );
    let text = r#"{"name": "test", "color": "red"}"#;
    let result = parser.parse(text).unwrap();
    assert_eq!(result.color, ColorEnum::Red);
}

#[test]
fn test_parse_literal_field() {
    let parser = PydanticOutputParser::<WithLiteralModel>::new(
        "WithLiteralModel",
        json!({
            "title": "WithLiteralModel",
            "type": "object",
            "properties": {
                "status": {"type": "string", "enum": ["active", "inactive", "pending"]},
                "name": {"type": "string"}
            },
            "required": ["status", "name"]
        }),
    );
    let text = r#"{"status": "active", "name": "test"}"#;
    let result = parser.parse(text).unwrap();
    assert_eq!(result.status, Status::Active);
}

#[test]
fn test_parse_list_field() {
    let parser = PydanticOutputParser::<WithListModel>::new(
        "WithListModel",
        json!({
            "title": "WithListModel",
            "type": "object",
            "properties": {
                "tags": {"type": "array", "items": {"type": "string"}},
                "scores": {"type": "array", "items": {"type": "integer"}}
            },
            "required": ["tags"]
        }),
    );
    let text = r#"{"tags": ["a", "b", "c"], "scores": [1, 2, 3]}"#;
    let result = parser.parse(text).unwrap();
    assert_eq!(result.tags, vec!["a", "b", "c"]);
    assert_eq!(result.scores, vec![1, 2, 3]);
}

#[test]
fn test_parse_deeply_nested() {
    let parser = PydanticOutputParser::<TopNested>::new(
        "TopNested",
        json!({
            "title": "TopNested",
            "type": "object",
            "properties": {
                "level1": {
                    "type": "object",
                    "properties": {
                        "level2": {
                            "type": "object",
                            "properties": {
                                "level3": {"type": "string"}
                            }
                        }
                    }
                }
            }
        }),
    );
    let text = r#"{"level1": {"level2": {"level3": "deep_value"}}}"#;
    let result = parser.parse(text).unwrap();
    assert_eq!(result.level1.level2.level3, "deep_value");
}

#[test]
fn test_invalid_json_raises() {
    let parser = simple_model_parser();
    let result = parser.parse("not json");
    assert!(result.is_err());
    let err_msg = format!("{}", result.unwrap_err());
    assert!(
        err_msg.contains("Invalid json output"),
        "Expected 'Invalid json output' in error: {}",
        err_msg
    );
}

#[test]
fn test_validation_error_raises() {
    let parser = simple_model_parser();
    let result = parser.parse(r#"{"name": "Alice", "age": "not_a_number"}"#);
    assert!(result.is_err());
    let err_msg = format!("{}", result.unwrap_err());
    assert!(
        err_msg.contains("Failed to parse SimpleModel"),
        "Expected 'Failed to parse SimpleModel' in error: {}",
        err_msg
    );
}

#[test]
fn test_missing_required_field_raises() {
    let parser = simple_model_parser();
    let result = parser.parse(r#"{"name": "Alice"}"#);
    assert!(result.is_err());
    let err_msg = format!("{}", result.unwrap_err());
    assert!(
        err_msg.contains("Failed to parse SimpleModel"),
        "Expected 'Failed to parse SimpleModel' in error: {}",
        err_msg
    );
}

#[test]
fn test_invalid_enum_raises() {
    let parser = PydanticOutputParser::<WithEnumModel>::new(
        "WithEnumModel",
        json!({
            "title": "WithEnumModel",
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "color": {"type": "string", "enum": ["red", "green", "blue"]}
            },
            "required": ["name", "color"]
        }),
    );
    let result = parser.parse(r#"{"name": "test", "color": "purple"}"#);
    assert!(result.is_err());
    let err_msg = format!("{}", result.unwrap_err());
    assert!(
        err_msg.contains("Failed to parse"),
        "Expected 'Failed to parse' in error: {}",
        err_msg
    );
}

#[test]
fn test_invalid_literal_raises() {
    let parser = PydanticOutputParser::<WithLiteralModel>::new(
        "WithLiteralModel",
        json!({
            "title": "WithLiteralModel",
            "type": "object",
            "properties": {
                "status": {"type": "string", "enum": ["active", "inactive", "pending"]},
                "name": {"type": "string"}
            },
            "required": ["status", "name"]
        }),
    );
    let result = parser.parse(r#"{"status": "unknown", "name": "test"}"#);
    assert!(result.is_err());
    let err_msg = format!("{}", result.unwrap_err());
    assert!(
        err_msg.contains("Failed to parse"),
        "Expected 'Failed to parse' in error: {}",
        err_msg
    );
}

#[test]
fn test_parser_exception_contains_llm_output() {
    let parser = simple_model_parser();
    let result = parser.parse(r#"{"name": 123, "age": "bad"}"#);
    assert!(result.is_err());
    let err_msg = format!("{}", result.unwrap_err());
    assert!(
        err_msg.contains("Failed to parse SimpleModel"),
        "Expected 'Failed to parse SimpleModel' in error: {}",
        err_msg
    );
}

#[test]
fn test_parse_result_valid() {
    let parser = simple_model_parser();
    let generation = Generation::builder()
        .text(r#"{"name": "Alice", "age": 30}"#)
        .build();
    let result = parser.parse_result(&[generation], false).unwrap();
    assert_eq!(result.name, "Alice");
}

#[test]
fn test_parse_result_partial_valid() {
    let parser = simple_model_parser();
    let generation = Generation::builder()
        .text(r#"{"name": "Alice", "age": 30}"#)
        .build();
    let result = parser.parse_result(&[generation], true).unwrap();
    assert_eq!(result.name, "Alice");
}

#[test]
fn test_parse_result_partial_invalid_returns_err() {
    let parser = simple_model_parser();
    let generation = Generation::builder().text("not json").build();
    let result = parser.parse_result(&[generation], true);
    assert!(result.is_err());
}

#[test]
fn test_parse_result_non_partial_invalid_raises() {
    let parser = simple_model_parser();
    let generation = Generation::builder().text("not json").build();
    let result = parser.parse_result(&[generation], false);
    assert!(result.is_err());
}

#[test]
fn test_parse_obj_valid() {
    let parser = simple_model_parser();
    let obj = json!({"name": "Alice", "age": 30});
    let result = parser.parse_obj(&obj).unwrap();
    assert_eq!(result.name, "Alice");
}

#[test]
fn test_parse_obj_invalid_raises() {
    let parser = simple_model_parser();
    let obj = json!({"name": "Alice", "age": "not_int"});
    let result = parser.parse_obj(&obj);
    assert!(result.is_err());
    let err_msg = format!("{}", result.unwrap_err());
    assert!(
        err_msg.contains("Failed to parse"),
        "Expected 'Failed to parse' in error: {}",
        err_msg
    );
}

#[test]
fn test_parser_exception_message() {
    let parser = simple_model_parser();
    let test_error = "test error";
    let json_object = json!({"name": "Alice", "age": "bad"});
    let exc = parser.parser_exception(&test_error, &json_object);
    let err_msg = format!("{}", exc);
    assert!(
        err_msg.contains("Failed to parse SimpleModel"),
        "Expected 'Failed to parse SimpleModel' in error: {}",
        err_msg
    );
    assert!(
        err_msg.contains("test error"),
        "Expected 'test error' in error: {}",
        err_msg
    );
}

#[test]
fn test_contains_schema_fields() {
    let parser = simple_model_parser();
    let instructions = parser.get_format_instructions().unwrap();
    assert!(
        instructions.contains("name"),
        "Expected 'name' in instructions"
    );
    assert!(
        instructions.contains("age"),
        "Expected 'age' in instructions"
    );
}

#[test]
fn test_does_not_contain_title() {
    let parser = simple_model_parser();
    let instructions = parser.get_format_instructions().unwrap();
    if let Some(start) = instructions.find("```\n") {
        let after_ticks = &instructions[start + 4..];
        if let Some(end) = after_ticks.find("\n```") {
            let json_part = &after_ticks[..end];
            let schema_dict: Value = serde_json::from_str(json_part).unwrap();
            assert!(
                schema_dict.get("title").is_none(),
                "Schema should not contain 'title'"
            );
            assert!(
                schema_dict.get("type").is_none(),
                "Schema should not contain 'type'"
            );
        }
    }
}

#[test]
fn test_unicode_preserved() {
    let parser = PydanticOutputParser::<UnicodeModel>::new(
        "UnicodeModel",
        json!({
            "title": "UnicodeModel",
            "type": "object",
            "properties": {
                "title": {"type": "string", "description": "科学文章的标题"},
                "author": {"type": "string", "description": "作者姓名"}
            },
            "required": ["title", "author"]
        }),
    );
    let instructions = parser.get_format_instructions().unwrap();
    assert!(
        instructions.contains("科学文章的标题"),
        "Expected Chinese characters in instructions"
    );
    assert!(
        instructions.contains("作者姓名"),
        "Expected Chinese characters in instructions"
    );
}

#[test]
fn test_instructions_contain_example() {
    let parser = simple_model_parser();
    let instructions = parser.get_format_instructions().unwrap();
    let lower = instructions.to_lowercase();
    assert!(
        lower.contains("example"),
        "Expected 'example' in instructions"
    );
    assert!(
        lower.contains("well-formatted"),
        "Expected 'well-formatted' in instructions"
    );
}

#[test]
fn test_does_not_alter_original_schema() {
    let original_schema = simple_model_schema();
    let parser = PydanticOutputParser::<SimpleModel>::new("SimpleModel", original_schema.clone());
    let _ = parser.get_format_instructions().unwrap();
    assert_eq!(
        *parser.get_schema(),
        original_schema,
        "get_format_instructions should not alter the stored schema"
    );
}

#[test]
fn test_type_property() {
    let parser = simple_model_parser();
    assert_eq!(parser.parser_type(), "pydantic");
}

#[test]
fn test_output_type_name() {
    let parser = simple_model_parser();
    assert_eq!(parser.output_type_name(), "SimpleModel");
}

#[test]
fn test_output_type_name_nested() {
    let parser = PydanticOutputParser::<PersonWithAddress>::new(
        "PersonWithAddress",
        json!({"title": "PersonWithAddress", "type": "object"}),
    );
    assert_eq!(parser.output_type_name(), "PersonWithAddress");
}
