use std::fmt::Debug;

use crate::error::Result;
use crate::load::{Serializable, Serialized, SerializedConstructor};
use crate::outputs::Generation;

use super::base::BaseOutputParser;
use super::transform::BaseTransformOutputParser;

#[derive(Debug, Clone, Default)]
pub struct StrOutputParser {
    _private: (),
}

impl StrOutputParser {
    pub fn new() -> Self {
        Self { _private: () }
    }

    pub fn is_lc_serializable() -> bool {
        true
    }

    pub fn get_lc_namespace() -> Vec<&'static str> {
        vec!["langchain", "schema", "output_parser"]
    }
}

impl BaseOutputParser for StrOutputParser {
    type Output = String;

    fn parse(&self, text: &str) -> Result<String> {
        Ok(text.to_string())
    }

    fn parser_type(&self) -> &str {
        "default"
    }
}

impl BaseTransformOutputParser for StrOutputParser {
    fn parse_generation(&self, generation: &Generation) -> Result<Self::Output> {
        Ok(generation.text.clone())
    }
}

impl Serializable for StrOutputParser {
    fn is_lc_serializable() -> bool
    where
        Self: Sized,
    {
        true
    }

    fn get_lc_namespace() -> Vec<String>
    where
        Self: Sized,
    {
        vec![
            "langchain".to_string(),
            "schema".to_string(),
            "output_parser".to_string(),
        ]
    }

    fn to_json(&self) -> Serialized
    where
        Self: Sized + serde::Serialize,
    {
        SerializedConstructor::new(Self::lc_id(), std::collections::HashMap::new()).into()
    }
}

impl serde::Serialize for StrOutputParser {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let s = serializer.serialize_struct("StrOutputParser", 0)?;
        s.end()
    }
}

impl<'de> serde::Deserialize<'de> for StrOutputParser {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let _ = serde_json::Value::deserialize(deserializer)?;
        Ok(Self::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_str_output_parser() {
        let parser = StrOutputParser::new();
        let result = parser.parse("Hello, world!").unwrap();
        assert_eq!(result, "Hello, world!");
    }

    #[test]
    fn test_str_output_parser_empty() {
        let parser = StrOutputParser::new();
        let result = parser.parse("").unwrap();
        assert_eq!(result, "");
    }

    #[test]
    fn test_str_output_parser_multiline() {
        let parser = StrOutputParser::new();
        let result = parser.parse("line1\nline2\nline3").unwrap();
        assert_eq!(result, "line1\nline2\nline3");
    }

    #[test]
    fn test_parser_type() {
        let parser = StrOutputParser::new();
        assert_eq!(parser.parser_type(), "default");
    }
}
