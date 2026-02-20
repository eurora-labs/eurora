use std::fmt::Debug;

use regex::Regex;
use serde_json::Value;

use crate::error::{Error, Result};
use crate::messages::BaseMessage;
use crate::outputs::Generation;
use crate::runnables::AddableDict;

use super::base::BaseOutputParser;
use super::transform::BaseTransformOutputParser;

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

pub(crate) struct StreamingParser {
    buffer: String,
    xml_started: bool,
    xml_start_re: Regex,
    yielded_count: usize,
    current_path: Vec<String>,
    current_path_has_children: bool,
}

impl StreamingParser {
    fn new() -> Self {
        Self {
            buffer: String::new(),
            xml_started: false,
            xml_start_re: Regex::new(r"<[a-zA-Z:_]").expect("Invalid regex"),
            yielded_count: 0,
            current_path: Vec::new(),
            current_path_has_children: false,
        }
    }

    fn parse(&mut self, chunk: &str) -> Vec<AddableDict> {
        self.buffer.push_str(chunk);

        if !self.xml_started {
            if let Some(m) = self.xml_start_re.find(&self.buffer) {
                self.buffer = self.buffer[m.start()..].to_string();
                self.xml_started = true;
            } else {
                return Vec::new();
            }
        }

        let mut reader = quick_xml::Reader::from_str(&self.buffer);
        reader.config_mut().trim_text(false);

        let mut all_results = Vec::new();
        let mut current_text = String::new();
        let mut path: Vec<String> = Vec::new();
        let mut path_has_children = false;

        loop {
            match reader.read_event() {
                Ok(quick_xml::events::Event::Start(ref e)) => {
                    let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    path.push(tag);
                    path_has_children = false;
                    current_text.clear();
                }
                Ok(quick_xml::events::Event::End(ref e)) => {
                    let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    path.pop();

                    if !path_has_children {
                        let text = current_text.trim().to_string();
                        let text_opt = if text.is_empty() {
                            None
                        } else {
                            Some(text.as_str())
                        };
                        all_results.push(nested_element(&path, &tag, text_opt));
                    }

                    if !path.is_empty() {
                        path_has_children = true;
                    }
                    current_text.clear();
                }
                Ok(quick_xml::events::Event::Text(ref e)) => {
                    if let Ok(text) = e.unescape() {
                        current_text.push_str(&text);
                    }
                }
                Ok(quick_xml::events::Event::CData(ref e)) => {
                    if let Ok(text) = std::str::from_utf8(e.as_ref()) {
                        current_text.push_str(text);
                    }
                }
                Ok(quick_xml::events::Event::Empty(ref e)) => {
                    let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    all_results.push(nested_element(&path, &tag, None));
                    if !path.is_empty() {
                        path_has_children = true;
                    }
                }
                Ok(quick_xml::events::Event::Eof) => break,
                Err(_) => {
                    if path.is_empty() {
                        self.buffer.clear();
                        self.xml_started = false;
                    }
                    break;
                }
                _ => {}
            }
        }

        self.current_path = path;
        self.current_path_has_children = path_has_children;
        if self.current_path.is_empty() && !all_results.is_empty() {
            self.xml_started = false;
        }

        let new_results = if self.yielded_count < all_results.len() {
            all_results[self.yielded_count..].to_vec()
        } else {
            Vec::new()
        };
        self.yielded_count = all_results.len();

        new_results
    }

    fn close(&mut self) {
        self.buffer.clear();
    }
}

#[derive(Debug, Clone)]
pub struct XMLOutputParser {
    pub tags: Option<Vec<String>>,

    encoding_matcher: Regex,
}

impl XMLOutputParser {
    pub fn new() -> Self {
        Self {
            tags: None,
            encoding_matcher: Regex::new(r"(?s)<([^>]*encoding[^>]*)>\n(.*)")
                .expect("Invalid regex pattern"),
        }
    }

    pub fn with_tags(tags: Vec<String>) -> Self {
        Self {
            tags: Some(tags),
            ..Self::new()
        }
    }

    fn parse_xml(&self, text: &str) -> Result<Value> {
        let text = self.preprocess_xml(text);

        let mut reader = quick_xml::Reader::from_str(&text);
        reader.config_mut().trim_text(true);

        self.read_root(&mut reader)
    }

    fn read_root(&self, reader: &mut quick_xml::Reader<&[u8]>) -> Result<Value> {
        loop {
            match reader.read_event() {
                Ok(quick_xml::events::Event::Start(ref e)) => {
                    let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    let value = self.read_element_content(reader, &tag)?;
                    let mut result = serde_json::Map::new();
                    result.insert(tag, value);
                    return Ok(Value::Object(result));
                }
                Ok(quick_xml::events::Event::Empty(ref e)) => {
                    let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    let mut result = serde_json::Map::new();
                    result.insert(tag, Value::Null);
                    return Ok(Value::Object(result));
                }
                Ok(quick_xml::events::Event::Eof) => {
                    return Ok(Value::Object(Default::default()));
                }
                Err(e) => {
                    return Err(Error::Other(format!(
                        "Failed to parse XML format from completion {}. Got: {}",
                        "<input>", e
                    )));
                }
                _ => {
                    continue;
                }
            }
        }
    }

    #[allow(clippy::only_used_in_recursion)]
    fn read_element_content(
        &self,
        reader: &mut quick_xml::Reader<&[u8]>,
        parent_tag: &str,
    ) -> Result<Value> {
        let mut text_content = String::new();
        let mut children: Vec<Value> = Vec::new();
        let mut has_children = false;

        loop {
            match reader.read_event() {
                Ok(quick_xml::events::Event::Start(ref e)) => {
                    has_children = true;
                    let child_tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    let child_value = self.read_element_content(reader, &child_tag)?;
                    let mut child_map = serde_json::Map::new();
                    child_map.insert(child_tag, child_value);
                    children.push(Value::Object(child_map));
                }
                Ok(quick_xml::events::Event::Empty(ref e)) => {
                    has_children = true;
                    let child_tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    let mut child_map = serde_json::Map::new();
                    child_map.insert(child_tag, Value::Null);
                    children.push(Value::Object(child_map));
                }
                Ok(quick_xml::events::Event::Text(ref e)) => {
                    if let Ok(t) = e.unescape() {
                        text_content.push_str(&t);
                    }
                }
                Ok(quick_xml::events::Event::CData(ref e)) => {
                    if let Ok(t) = std::str::from_utf8(e.as_ref()) {
                        text_content.push_str(t);
                    }
                }
                Ok(quick_xml::events::Event::End(ref e)) => {
                    let end_tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    if end_tag == parent_tag {
                        break;
                    }
                }
                Ok(quick_xml::events::Event::Eof) => break,
                Err(e) => {
                    return Err(Error::Other(format!("XML parse error: {}", e)));
                }
                _ => continue,
            }
        }

        if has_children {
            Ok(Value::Array(children))
        } else {
            let trimmed = text_content.trim().to_string();
            if trimmed.is_empty() {
                Ok(Value::Null)
            } else {
                Ok(Value::String(trimmed))
            }
        }
    }

    fn preprocess_xml(&self, text: &str) -> String {
        let mut text = text.to_string();

        let re = Regex::new(r"(?s)```(?:xml)?(.*?)```").expect("Invalid regex");
        if let Some(caps) = re.captures(&text)
            && let Some(m) = caps.get(1)
        {
            text = m.as_str().to_string();
        }

        if let Some(caps) = self.encoding_matcher.captures(&text)
            && let Some(m) = caps.get(2)
        {
            text = m.as_str().to_string();
        }

        text.trim().to_string()
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

impl BaseTransformOutputParser for XMLOutputParser {
    fn transform<'a>(
        &'a self,
        input: futures::stream::BoxStream<'a, BaseMessage>,
    ) -> futures::stream::BoxStream<'a, Result<Self::Output>>
    where
        Self::Output: 'a,
    {
        Box::pin(async_stream::stream! {
            use futures::StreamExt;

            let mut streaming_parser = StreamingParser::new();
            let mut stream = input;
            while let Some(message) = stream.next().await {
                let chunk_text = message.text().to_string();
                for dict in streaming_parser.parse(&chunk_text) {
                    match serde_json::to_value(&dict) {
                        Ok(value) => yield Ok(value),
                        Err(e) => yield Err(Error::Other(format!("XML serialization error: {}", e))),
                    }
                }
            }
            streaming_parser.close();
        })
    }

    fn atransform<'a>(
        &'a self,
        input: futures::stream::BoxStream<'a, BaseMessage>,
    ) -> futures::stream::BoxStream<'a, Result<Self::Output>>
    where
        Self::Output: 'a,
    {
        self.transform(input)
    }
}

pub fn nested_element(path: &[String], tag: &str, text: Option<&str>) -> AddableDict {
    let inner_value = match text {
        Some(t) => Value::String(t.to_string()),
        None => Value::Null,
    };

    let mut inner = AddableDict::new();
    inner.0.insert(tag.to_string(), inner_value);

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
        let result = parser.parse("```xml\n<root>value</root>\n```").unwrap();
        assert_eq!(result["root"], "value");
    }

    #[test]
    fn test_xml_parser_with_multiline_markdown() {
        let parser = XMLOutputParser::new();
        let input = "```xml\n<root>\n  <child>value</child>\n</root>\n```";
        let result = parser.parse(input).unwrap();
        assert!(result["root"].is_array());
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
        assert!(result.0.contains_key("root"));
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

    #[test]
    fn test_xml_parser_self_closing() {
        let parser = XMLOutputParser::new();
        let result = parser.parse("<root><item/></root>").unwrap();
        assert!(result["root"].is_array());
        assert!(result["root"][0]["item"].is_null());
    }

    #[test]
    fn test_xml_parser_nested_same_name() {
        let parser = XMLOutputParser::new();
        let result = parser
            .parse("<root><item><item>inner</item></item></root>")
            .unwrap();
        assert!(result["root"].is_array());
    }

    #[test]
    fn test_streaming_parser_basic() {
        let mut parser = StreamingParser::new();
        let results = parser.parse("<root><item>value</item></root>");
        assert_eq!(results.len(), 1);
        assert_eq!(
            results[0].0.get("root"),
            Some(&Value::Array(vec![
                serde_json::to_value(&{
                    let mut d = AddableDict::new();
                    d.0.insert("item".to_string(), Value::String("value".to_string()));
                    d
                })
                .unwrap()
            ]))
        );
    }

    #[test]
    fn test_streaming_parser_chunks() {
        let mut parser = StreamingParser::new();
        let r1 = parser.parse("<root><ite");
        assert!(r1.is_empty());
        let r2 = parser.parse("m>val");
        assert!(r2.is_empty());
        let r3 = parser.parse("ue</item></root>");
        assert_eq!(r3.len(), 1);
    }

    #[test]
    fn test_streaming_parser_preamble() {
        let mut parser = StreamingParser::new();
        let r1 = parser.parse("Here is the XML: ");
        assert!(r1.is_empty());
        let r2 = parser.parse("<root><item>value</item></root>");
        assert_eq!(r2.len(), 1);
    }

    #[test]
    fn test_streaming_parser_multiple_children() {
        let mut parser = StreamingParser::new();
        let results = parser.parse("<root><a>1</a><b>2</b></root>");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_streaming_parser_nested_only_yields_leaves() {
        let mut parser = StreamingParser::new();
        let results = parser.parse("<root><parent><child>val</child></parent></root>");
        assert_eq!(results.len(), 1);
        let result = &results[0];
        assert!(result.0.contains_key("root"));
    }

    #[test]
    fn test_streaming_parser_self_closing() {
        let mut parser = StreamingParser::new();
        let results = parser.parse("<root><item/></root>");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_streaming_parser_close() {
        let mut parser = StreamingParser::new();
        parser.parse("<root><item>partial");
        parser.close();
    }

    #[tokio::test]
    async fn test_xml_transform_stream() {
        use crate::messages::HumanMessage;
        use futures::StreamExt;

        let parser = XMLOutputParser::new();
        let messages: Vec<BaseMessage> = vec![
            BaseMessage::Human(HumanMessage::builder().content("<root>").build()),
            BaseMessage::Human(
                HumanMessage::builder()
                    .content("<item>hello</item>")
                    .build(),
            ),
            BaseMessage::Human(HumanMessage::builder().content("</root>").build()),
        ];
        let stream = futures::stream::iter(messages);
        let boxed: futures::stream::BoxStream<BaseMessage> = Box::pin(stream);
        let mut output_stream = parser.transform(boxed);

        let mut results = Vec::new();
        while let Some(result) = output_stream.next().await {
            results.push(result.unwrap());
        }
        assert_eq!(results.len(), 1);
        assert_eq!(results[0]["root"][0]["item"], "hello");
    }
}
