mod base;
mod format_instructions;
mod json;
mod list;
mod openai_functions;
pub mod openai_tools;
mod pydantic;
mod string;
mod transform;
mod xml;

pub use base::{
    BaseGenerationOutputParser, BaseLLMOutputParser, BaseOutputParser, OutputParserError,
    RunnableOutputParser,
};

pub use format_instructions::JSON_FORMAT_INSTRUCTIONS;

pub use string::StrOutputParser;

pub use transform::{BaseCumulativeTransformOutputParser, BaseTransformOutputParser};

pub use json::{JsonOutputParser, SimpleJsonOutputParser};

pub use list::{
    CommaSeparatedListOutputParser, ListOutputParser, MarkdownListOutputParser,
    NumberedListOutputParser, ParseMatch, drop_last_n,
};

pub use pydantic::PydanticOutputParser;

pub use xml::{XMLOutputParser, nested_element};

pub use openai_functions::{
    JsonKeyOutputFunctionsParser, JsonOutputFunctionsParser, OutputFunctionsParser,
    PydanticAttrOutputFunctionsParser, PydanticOutputFunctionsParser, PydanticSchema,
};

pub use openai_tools::{
    JsonOutputKeyToolsParser, JsonOutputToolsParser, PydanticToolsParser, make_invalid_tool_call,
    parse_tool_call, parse_tool_calls,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_str_parser_export() {
        let parser = StrOutputParser::new();
        let result = parser.parse("test").unwrap();
        assert_eq!(result, "test");
    }

    #[test]
    fn test_json_parser_export() {
        let parser = JsonOutputParser::builder().build();
        let result = parser.parse(r#"{"key": "value"}"#).unwrap();
        assert_eq!(result["key"], "value");
    }

    #[test]
    fn test_list_parser_export() {
        let parser = CommaSeparatedListOutputParser::new();
        let result = parser.parse("a, b, c").unwrap();
        assert_eq!(result, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_xml_parser_export() {
        let parser = XMLOutputParser::new();
        let result = parser.parse("<root>value</root>").unwrap();
        assert_eq!(result["root"], "value");
    }
}
