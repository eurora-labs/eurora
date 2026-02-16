/// Format instructions for output parsers.
///
/// Mirrors `langchain_core.output_parsers.format_instructions`.
///
/// JSON format instructions template.
///
/// Uses `{schema}` as the placeholder for the JSON schema, matching the
/// Python format string in `langchain_core.output_parsers.format_instructions`.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_format_instructions_exists_and_contains_expected_strings() {
        assert!(JSON_FORMAT_INSTRUCTIONS.contains("{schema}"));
        assert!(JSON_FORMAT_INSTRUCTIONS.contains("JSON instance"));
        assert!(JSON_FORMAT_INSTRUCTIONS.contains("well-formatted"));
    }
}
