//! Format instructions for output parsers.
//!
//! This module contains format instruction templates used by output parsers
//! to guide LLM output formatting.
//! Mirrors `langchain_core.output_parsers.format_instructions`.

/// JSON format instructions template.
///
/// This is used by JSON output parsers to instruct the LLM how to format
/// its output as valid JSON that conforms to a schema.
pub const JSON_FORMAT_INSTRUCTIONS: &str = r#"STRICT OUTPUT FORMAT:
- Return only the JSON value that conforms to the schema. Do not include any additional text, explanations, headings, or separators.
- Do not wrap the JSON in Markdown or code fences (no ``` or ```json).
- Do not prepend or append any text (e.g., do not write "Here is the JSON:").
- The response must be a single top-level JSON value exactly as required by the schema (object/array/etc.), with no trailing commas or comments.

The output should be formatted as a JSON instance that conforms to the JSON schema below.

As an example, for the schema {{"properties": {{"foo": {{"title": "Foo", "description": "a list of strings", "type": "array", "items": {{"type": "string"}}}}}}, "required": ["foo"]}} the object {{"foo": ["bar", "baz"]}} is a well-formatted instance of the schema. The object {{"properties": {{"foo": ["bar", "baz"]}}}} is not well-formatted.

Here is the output schema (shown in a code block for readability only — do not include any backticks or Markdown in your output):
```
{schema}
```"#;

/// Pydantic-style format instructions template.
///
/// This is used by Pydantic output parsers (struct validators in Rust)
/// to instruct the LLM how to format its output.
pub const PYDANTIC_FORMAT_INSTRUCTIONS: &str = r#"The output should be formatted as a JSON instance that conforms to the JSON schema below.

As an example, for the schema {{"properties": {{"foo": {{"title": "Foo", "description": "a list of strings", "type": "array", "items": {{"type": "string"}}}}}}, "required": ["foo"]}}
the object {{"foo": ["bar", "baz"]}} is a well-formatted instance of the schema. The object {{"properties": {{"foo": ["bar", "baz"]}}}} is not well-formatted.

Here is the output schema:
```
{schema}
```"#;

/// XML format instructions template.
///
/// This is used by XML output parsers to instruct the LLM how to format
/// its output as valid XML.
pub const XML_FORMAT_INSTRUCTIONS: &str = r#"The output should be formatted as a XML file.
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

/// Format the JSON format instructions with a schema.
///
/// # Arguments
///
/// * `schema` - The JSON schema string to insert into the template.
///
/// # Returns
///
/// The formatted instructions string.
pub fn format_json_instructions(schema: &str) -> String {
    JSON_FORMAT_INSTRUCTIONS.replace("{schema}", schema)
}

/// Format the Pydantic format instructions with a schema.
///
/// # Arguments
///
/// * `schema` - The JSON schema string to insert into the template.
///
/// # Returns
///
/// The formatted instructions string.
pub fn format_pydantic_instructions(schema: &str) -> String {
    PYDANTIC_FORMAT_INSTRUCTIONS.replace("{schema}", schema)
}

/// Format the XML format instructions with tags.
///
/// # Arguments
///
/// * `tags` - The tags string to insert into the template.
///
/// # Returns
///
/// The formatted instructions string.
pub fn format_xml_instructions(tags: &str) -> String {
    XML_FORMAT_INSTRUCTIONS.replace("{tags}", tags)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_json_instructions() {
        let schema = r#"{"type": "object", "properties": {"name": {"type": "string"}}}"#;
        let result = format_json_instructions(schema);
        assert!(result.contains(schema));
        assert!(result.contains("STRICT OUTPUT FORMAT"));
    }

    #[test]
    fn test_format_pydantic_instructions() {
        let schema = r#"{"type": "object"}"#;
        let result = format_pydantic_instructions(schema);
        assert!(result.contains(schema));
    }

    #[test]
    fn test_format_xml_instructions() {
        let tags = r#"["foo", "bar"]"#;
        let result = format_xml_instructions(tags);
        assert!(result.contains(tags));
        assert!(result.contains("XML file"));
    }
}
