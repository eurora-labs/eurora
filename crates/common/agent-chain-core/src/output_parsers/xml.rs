//! Output parser for XML format.
//!
//! This module contains the `XMLOutputParser` which parses LLM output as XML.
//! Mirrors `langchain_core.output_parsers.xml`.

use std::fmt::Debug;

use regex::Regex;
use serde_json::Value;

use crate::error::{Error, Result};
use crate::outputs::Generation;
use crate::runnables::AddableDict;

use super::base::BaseOutputParser;
use super::transform::BaseTransformOutputParser;

/// XML format instructions template, defined locally matching
/// `XML_FORMAT_INSTRUCTIONS` in `langchain_core.output_parsers.xml`.
const XML_FORMAT_INSTRUCTIONS: &str = r#"The output should be formatted as a XML file.
1. Output should conform to the tags below.
2. If tags are not given, make them on your own.
3. Remember to always open and close all the tags.

As an example, for the tags ["foo", "bar", "baz"]:
1. String "<foo>\n   <bar>\n      <baz></baz>\n   </bar>\n</foo>" is a well-formatted instance of the schema.
2. String "<foo>\n   <bar>\n   </foo>" is a badly-formatted instance.
3. String "<foo>\n   <tag>\n   </tag>\n</foo>" is a badly-formatted instance.

Here are the output tags:
```
{tags}
```"#;

/// Parse an output using XML format.
///
/// Returns a dictionary of tags.
///
/// # Example
///
/// ```ignore
/// use agent_chain_core::output_parsers::XMLOutputParser;
///
/// let parser = XMLOutputParser::new();
/// let result = parser.parse("<root><item>value</item></root>").unwrap();
/// ```
#[derive(Debug, Clone)]
pub struct XMLOutputParser {
    /// Tags to tell the LLM to expect in the XML output.
    pub tags: Option<Vec<String>>,

    /// Regex pattern to match encoding declarations.
    encoding_matcher: Regex,
}

impl XMLOutputParser {
    /// Create a new `XMLOutputParser` with no tag hints.
    pub fn new() -> Self {
        Self {
            tags: None,
            encoding_matcher: Regex::new(r"<([^>]*encoding[^>]*)>\n(.*)")
                .expect("Invalid regex pattern"),
        }
    }

    /// Create a parser with expected tags.
    pub fn with_tags(tags: Vec<String>) -> Self {
        Self {
            tags: Some(tags),
            ..Self::new()
        }
    }

    /// Parse XML string into a nested dictionary structure.
    fn parse_xml(&self, text: &str) -> Result<Value> {
        let text = self.preprocess_xml(text);

        // Simple XML parser using regex
        // For production, consider using a proper XML parser like quick-xml
        self.parse_xml_element(&text)
    }

    /// Preprocess XML text to handle code blocks and encoding.
    fn preprocess_xml(&self, text: &str) -> String {
        let mut text = text.to_string();

        // Try to find XML string within triple backticks
        let re = Regex::new(r"```(?:xml)?(.*)```").expect("Invalid regex");
        if let Some(caps) = re.captures(&text)
            && let Some(m) = caps.get(1)
        {
            text = m.as_str().to_string();
        }

        // Remove encoding declaration if present
        if let Some(caps) = self.encoding_matcher.captures(&text)
            && let Some(m) = caps.get(2)
        {
            text = m.as_str().to_string();
        }

        text.trim().to_string()
    }

    /// Parse an XML element into a Value.
    fn parse_xml_element(&self, text: &str) -> Result<Value> {
        let text = text.trim();

        if text.is_empty() {
            return Ok(Value::Object(Default::default()));
        }

        // Match opening tag
        let tag_re = Regex::new(r"<([a-zA-Z_][a-zA-Z0-9_:-]*)([^>]*)>").expect("Invalid regex");

        let Some(caps) = tag_re.captures(text) else {
            return Err(Error::Other(format!(
                "Failed to parse XML: no opening tag found in '{}'",
                text.chars().take(100).collect::<String>()
            )));
        };

        let tag_name = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        let tag_start = caps.get(0).map(|m| m.end()).unwrap_or(0);

        // Find closing tag
        let closing_tag = format!("</{}>", tag_name);
        let Some(closing_pos) = text.rfind(&closing_tag) else {
            return Err(Error::Other(format!(
                "Failed to parse XML: no closing tag found for '{}'",
                tag_name
            )));
        };

        let inner_content = &text[tag_start..closing_pos];
        let inner_trimmed = inner_content.trim();

        // Check if inner content contains child elements
        if inner_trimmed.starts_with('<') && inner_trimmed.contains("</") {
            // Parse children
            let children = self.parse_xml_children(inner_trimmed)?;
            let mut result = serde_json::Map::new();
            result.insert(tag_name.to_string(), children);
            Ok(Value::Object(result))
        } else {
            // Leaf node with text content
            let mut result = serde_json::Map::new();
            if inner_trimmed.is_empty() {
                result.insert(tag_name.to_string(), Value::Null);
            } else {
                result.insert(
                    tag_name.to_string(),
                    Value::String(inner_trimmed.to_string()),
                );
            }
            Ok(Value::Object(result))
        }
    }

    /// Parse multiple XML child elements.
    fn parse_xml_children(&self, text: &str) -> Result<Value> {
        let mut children = Vec::new();
        let tag_re = Regex::new(r"<([a-zA-Z_][a-zA-Z0-9_:-]*)([^>]*)>").expect("Invalid regex");

        let mut remaining = text;

        while let Some(caps) = tag_re.captures(remaining) {
            let tag_name = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            let tag_match = caps.get(0).unwrap();
            let tag_start = tag_match.start();
            let tag_end = tag_match.end();

            // Find closing tag
            let closing_tag = format!("</{}>", tag_name);
            if let Some(closing_pos) = remaining[tag_end..].find(&closing_tag) {
                let element_end = tag_end + closing_pos + closing_tag.len();
                let element = &remaining[tag_start..element_end];

                // Parse the element
                if let Ok(parsed) = self.parse_xml_element(element) {
                    children.push(parsed);
                }

                remaining = &remaining[element_end..];
            } else {
                break;
            }
        }

        Ok(Value::Array(children))
    }
}

impl Default for XMLOutputParser {
    fn default() -> Self {
        Self::new()
    }
}

impl BaseOutputParser for XMLOutputParser {
    type Output = Value;

    fn parse(&self, text: &str) -> Result<Value> {
        self.parse_xml(text)
    }

    fn parse_result(&self, result: &[Generation], _partial: bool) -> Result<Value> {
        if result.is_empty() {
            return Err(Error::Other("No generations to parse".to_string()));
        }
        self.parse(&result[0].text)
    }

    fn get_format_instructions(&self) -> Result<String> {
        match &self.tags {
            Some(tags) => {
                let tags_str = format!("{:?}", tags);
                Ok(XML_FORMAT_INSTRUCTIONS.replace("{tags}", &tags_str))
            }
            None => Ok(XML_FORMAT_INSTRUCTIONS.replace("{tags}", "[]")),
        }
    }

    fn parser_type(&self) -> &str {
        "xml"
    }
}

impl BaseTransformOutputParser for XMLOutputParser {}

/// Create a nested dictionary element from a path.
///
/// Used for streaming XML parsing.
pub fn nested_element(path: &[String], tag: &str, text: Option<&str>) -> AddableDict {
    let inner_value = match text {
        Some(t) => Value::String(t.to_string()),
        None => Value::Null,
    };

    let mut inner = AddableDict::new();
    inner.0.insert(tag.to_string(), inner_value);

    // Build nested structure from path
    let mut result = inner;
    for key in path.iter().rev() {
        let mut wrapper = AddableDict::new();
        wrapper.0.insert(
            key.clone(),
            Value::Array(vec![serde_json::to_value(&result).unwrap_or(Value::Null)]),
        );
        result = wrapper;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xml_parser_simple() {
        let parser = XMLOutputParser::new();
        let result = parser.parse("<root>value</root>").unwrap();
        assert_eq!(result["root"], "value");
    }

    #[test]
    fn test_xml_parser_nested() {
        let parser = XMLOutputParser::new();
        let result = parser.parse("<root><child>value</child></root>").unwrap();
        assert!(result["root"].is_array());
    }

    #[test]
    fn test_xml_parser_empty() {
        let parser = XMLOutputParser::new();
        let result = parser.parse("<root></root>").unwrap();
        assert!(result["root"].is_null());
    }

    #[test]
    fn test_xml_parser_with_markdown() {
        let parser = XMLOutputParser::new();
        let result = parser
            .parse(
                "```xml
<root>value</root>
```",
            )
            .unwrap();
        assert_eq!(result["root"], "value");
    }

    #[test]
    fn test_xml_parser_format_instructions() {
        let parser = XMLOutputParser::with_tags(vec!["foo".to_string(), "bar".to_string()]);
        let instructions = parser
            .get_format_instructions()
            .expect("should return format instructions");
        assert!(instructions.contains("foo"));
        assert!(instructions.contains("bar"));
        assert!(instructions.contains("XML"));
    }

    #[test]
    fn test_parser_type() {
        let parser = XMLOutputParser::new();
        assert_eq!(parser.parser_type(), "xml");
    }

    #[test]
    fn test_nested_element() {
        let path = vec!["root".to_string()];
        let result = nested_element(&path, "item", Some("value"));
        assert!(result.0.get("root").is_some());
    }

    #[test]
    fn test_nested_element_empty_path() {
        let path: Vec<String> = vec![];
        let result = nested_element(&path, "item", Some("value"));
        assert_eq!(
            result.0.get("item"),
            Some(&Value::String("value".to_string()))
        );
    }
}
