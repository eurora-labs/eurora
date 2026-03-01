use crate::documents::Document;
use scraper::{Html, Node};
use std::collections::HashMap;

/// Splits HTML content into Documents based on specified header tags.
///
/// Performs a DFS traversal of the DOM tree, tracking active headers and their
/// hierarchy. When a header is encountered, the current chunk is finalized and
/// the header metadata is updated.
pub struct HTMLHeaderTextSplitter {
    header_mapping: HashMap<String, String>,
    header_tags: Vec<String>,
    return_each_element: bool,
}

impl HTMLHeaderTextSplitter {
    pub fn new(headers_to_split_on: Vec<(String, String)>, return_each_element: bool) -> Self {
        let mut sorted = headers_to_split_on;
        sorted.sort_by_key(|(tag, _)| tag[1..].parse::<u32>().unwrap_or(9999));
        let header_mapping: HashMap<String, String> = sorted.iter().cloned().collect();
        let header_tags: Vec<String> = sorted.iter().map(|(tag, _)| tag.clone()).collect();
        Self {
            header_mapping,
            header_tags,
            return_each_element,
        }
    }

    pub fn split_text(&self, text: &str) -> Vec<Document> {
        self.generate_documents(text)
    }

    fn generate_documents(&self, html_content: &str) -> Vec<Document> {
        let document = Html::parse_document(html_content);

        // active_headers: header_name -> (header_text, level, dom_depth)
        let mut active_headers: HashMap<String, (String, u32, usize)> = HashMap::new();
        let mut current_chunk: Vec<String> = Vec::new();
        let mut results: Vec<Document> = Vec::new();

        // Find the body element or use root
        let body = document
            .root_element()
            .descendent_elements()
            .find(|el| el.value().name() == "body");
        let start = body.unwrap_or_else(|| document.root_element());

        // Collect all element descendants for DFS-order traversal
        let elements: Vec<scraper::ElementRef> = start.descendent_elements().collect();

        for element_ref in &elements {
            let el = element_ref.value();
            let tag_name = el.name().to_lowercase();

            // Get direct text children only (non-recursive)
            let text_parts: Vec<String> = element_ref
                .children()
                .filter_map(|child| match child.value() {
                    Node::Text(text) => {
                        let trimmed = text.text.trim().to_string();
                        if trimmed.is_empty() {
                            None
                        } else {
                            Some(trimmed)
                        }
                    }
                    _ => None,
                })
                .collect();

            let node_text = text_parts.join(" ");
            if node_text.is_empty() {
                continue;
            }

            // Calculate DOM depth by counting ancestors
            let dom_depth = element_ref.ancestors().count();

            if self.header_tags.contains(&tag_name) {
                if !self.return_each_element
                    && let Some(doc) = finalize_chunk(&mut current_chunk, &active_headers)
                {
                    results.push(doc);
                }

                let level = tag_name[1..].parse::<u32>().unwrap_or(9999);
                active_headers.retain(|_, (_, lvl, _)| *lvl < level);

                if let Some(header_name) = self.header_mapping.get(&tag_name) {
                    active_headers
                        .insert(header_name.clone(), (node_text.clone(), level, dom_depth));
                }

                let header_meta = build_metadata(&active_headers);
                results.push(
                    Document::builder()
                        .page_content(node_text)
                        .metadata(header_meta)
                        .build(),
                );
            } else {
                active_headers.retain(|_, (_, _, d)| dom_depth >= *d);

                if self.return_each_element {
                    let meta = build_metadata(&active_headers);
                    results.push(
                        Document::builder()
                            .page_content(node_text)
                            .metadata(meta)
                            .build(),
                    );
                } else {
                    current_chunk.push(node_text);
                }
            }
        }

        if !self.return_each_element
            && let Some(doc) = finalize_chunk(&mut current_chunk, &active_headers)
        {
            results.push(doc);
        }

        results
    }
}

fn build_metadata(
    active_headers: &HashMap<String, (String, u32, usize)>,
) -> HashMap<String, serde_json::Value> {
    active_headers
        .iter()
        .map(|(k, (text, _, _))| (k.clone(), serde_json::Value::String(text.clone())))
        .collect()
}

fn finalize_chunk(
    chunk: &mut Vec<String>,
    headers: &HashMap<String, (String, u32, usize)>,
) -> Option<Document> {
    if chunk.is_empty() {
        return None;
    }
    let final_text: String = chunk
        .iter()
        .filter(|line| !line.trim().is_empty())
        .cloned()
        .collect::<Vec<_>>()
        .join("  \n");
    chunk.clear();
    if final_text.trim().is_empty() {
        return None;
    }
    let meta = build_metadata(headers);
    Some(
        Document::builder()
            .page_content(final_text)
            .metadata(meta)
            .build(),
    )
}

/// Splits HTML files based on specified headers and font sizes.
///
/// Uses libxml for DOM manipulation to convert elements with font-size > 20px
/// to h1 headers (equivalent to the Python XSLT transformation), then splits
/// the resulting HTML by header tags.
pub struct HTMLSectionSplitter {
    headers_to_split_on: HashMap<String, String>,
}

impl HTMLSectionSplitter {
    pub fn new(headers_to_split_on: Vec<(String, String)>) -> Self {
        Self {
            headers_to_split_on: headers_to_split_on.into_iter().collect(),
        }
    }

    pub fn split_text(
        &self,
        text: &str,
    ) -> Result<Vec<Document>, Box<dyn std::error::Error + Send + Sync>> {
        let converted = self.convert_possible_tags_to_header(text)?;
        let sections = self.split_html_by_headers(&converted);

        Ok(sections
            .into_iter()
            .map(|section| {
                let tag_name = section.tag_name.as_deref().unwrap_or("h1");
                let header_name = self
                    .headers_to_split_on
                    .get(tag_name)
                    .cloned()
                    .unwrap_or_else(|| "Header 1".to_string());

                let mut metadata = HashMap::new();
                metadata.insert(
                    header_name,
                    serde_json::Value::String(section.header.unwrap_or_default()),
                );

                Document::builder()
                    .page_content(section.content)
                    .metadata(metadata)
                    .build()
            })
            .collect())
    }

    /// Convert elements with font-size > 20px to h1 headers using libxml.
    fn convert_possible_tags_to_header(
        &self,
        html_content: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let parser = libxml::parser::Parser::default_html();
        let doc = parser.parse_string(html_content.as_bytes()).map_err(|e| {
            Box::new(crate::Error::ValidationError(format!(
                "Failed to parse HTML: {:?}",
                e
            ))) as Box<dyn std::error::Error + Send + Sync>
        })?;

        let root = match doc.get_root_element() {
            Some(r) => r,
            None => return Ok(html_content.to_string()),
        };

        let mut nodes_to_rename: Vec<libxml::tree::Node> = Vec::new();
        collect_font_size_nodes(&root, &mut nodes_to_rename);

        for mut node in nodes_to_rename {
            node.set_name("h1").map_err(|_| {
                Box::new(crate::Error::ValidationError(
                    "Failed to set node name".to_string(),
                )) as Box<dyn std::error::Error + Send + Sync>
            })?;
        }

        Ok(doc.node_to_string(&root))
    }

    /// Split HTML by header tags into sections.
    fn split_html_by_headers(&self, html_doc: &str) -> Vec<HtmlSection> {
        let document = Html::parse_document(html_doc);
        let header_names: Vec<&str> = self
            .headers_to_split_on
            .keys()
            .map(|s| s.as_str())
            .collect();

        // Collect body and header elements in document order
        let root = document.root_element();
        let all_elements: Vec<scraper::ElementRef> = std::iter::once(root)
            .chain(root.descendent_elements())
            .collect();

        let target_indices: Vec<(usize, String)> = all_elements
            .iter()
            .enumerate()
            .filter_map(|(idx, el)| {
                let name = el.value().name().to_lowercase();
                if name == "body" || header_names.contains(&name.as_str()) {
                    Some((idx, name))
                } else {
                    None
                }
            })
            .collect();

        // Collect all text nodes in document order for slicing
        let all_text_nodes: Vec<(usize, String)> = {
            let mut result = Vec::new();
            for (idx, el) in all_elements.iter().enumerate() {
                for child in el.children() {
                    if let Node::Text(text) = child.value() {
                        result.push((idx, text.text.to_string()));
                    }
                }
            }
            result
        };

        let mut sections: Vec<HtmlSection> = Vec::new();

        for (i, (target_idx, tag_name)) in target_indices.iter().enumerate() {
            let (current_header, current_header_tag) = if i == 0 {
                ("#TITLE#".to_string(), "h1".to_string())
            } else {
                let el = all_elements[*target_idx];
                let header_text = el_text_content(&el).trim().to_string();
                (header_text, tag_name.clone())
            };

            // Find the range of elements between this target and the next
            let next_target_idx = if i + 1 < target_indices.len() {
                Some(target_indices[i + 1].0)
            } else {
                None
            };

            // Collect text from elements in this section's range
            let section_text: Vec<&str> = all_text_nodes
                .iter()
                .filter(|(elem_idx, _)| {
                    *elem_idx >= *target_idx && next_target_idx.is_none_or(|next| *elem_idx < next)
                })
                .map(|(_, text)| text.as_str())
                .collect();

            let content = section_text.join(" ").trim().to_string();
            if !content.is_empty() {
                sections.push(HtmlSection {
                    header: Some(current_header),
                    content,
                    tag_name: Some(current_header_tag),
                });
            }
        }

        sections
    }
}

struct HtmlSection {
    header: Option<String>,
    content: String,
    tag_name: Option<String>,
}

/// Recursively collect nodes that have font-size > 20px in their style attribute.
fn collect_font_size_nodes(node: &libxml::tree::Node, result: &mut Vec<libxml::tree::Node>) {
    if let Some(style) = node.get_attribute("style")
        && let Some(font_size) = extract_font_size_px(&style)
        && font_size > 20.0
    {
        result.push(node.clone());
    }
    for child in node.get_child_elements() {
        collect_font_size_nodes(&child, result);
    }
}

/// Extract font-size value in px from a CSS style string.
fn extract_font_size_px(style: &str) -> Option<f64> {
    let lower = style.to_lowercase();
    let idx = lower.find("font-size")?;
    let rest = &lower[idx + "font-size".len()..];
    let rest = rest.trim_start().strip_prefix(':')?;
    let rest = rest.trim_start();
    let px_idx = rest.find("px")?;
    rest[..px_idx].trim().parse::<f64>().ok()
}

/// Get all text content from an ElementRef recursively.
fn el_text_content(el: &scraper::ElementRef) -> String {
    el.text().collect::<Vec<_>>().join("")
}
