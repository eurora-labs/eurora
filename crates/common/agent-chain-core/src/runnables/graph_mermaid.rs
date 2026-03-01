use std::collections::{HashMap, HashSet};
use std::path::Path;

use base64::Engine;
use serde_json::Value;

use bon::builder;

use crate::error::{Error, Result};

use super::graph::{Edge, MermaidDrawMethod, Node, NodeStyles};

const MARKDOWN_SPECIAL_CHARS: &[char] = &['*', '_', '`'];

pub fn to_safe_id(label: &str) -> String {
    let mut out = String::with_capacity(label.len());
    for ch in label.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
            out.push(ch);
        } else {
            out.push('\\');
            out.push_str(&format!("{:x}", ch as u32));
        }
    }
    out
}

fn value_to_yaml(value: &Value, indent: usize) -> String {
    let prefix = " ".repeat(indent);
    match value {
        Value::Object(map) => {
            let mut lines = Vec::new();
            for (key, val) in map {
                match val {
                    Value::Object(_) => {
                        lines.push(format!("{}{}:", prefix, key));
                        let nested = value_to_yaml(val, indent + 2);
                        lines.push(nested);
                    }
                    _ => {
                        let val_str = scalar_to_yaml(val);
                        lines.push(format!("{}{}: {}", prefix, key, val_str));
                    }
                }
            }
            lines.join("\n")
        }
        _ => {
            format!("{}{}", prefix, scalar_to_yaml(value))
        }
    }
}

fn scalar_to_yaml(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => "null".to_string(),
        Value::Array(arr) => {
            let items: Vec<String> = arr.iter().map(scalar_to_yaml).collect();
            format!("[{}]", items.join(", "))
        }
        Value::Object(_) => value_to_yaml(value, 0),
    }
}

#[builder]
pub fn draw_mermaid(
    nodes: &HashMap<String, Node>,
    edges: &[Edge],
    first_node: Option<&str>,
    last_node: Option<&str>,
    #[builder(default = true)] with_styles: bool,
    curve_style: &super::graph::CurveStyle,
    node_styles: &NodeStyles,
    #[builder(default = 10)] wrap_label_n_words: usize,
    frontmatter_config: Option<&HashMap<String, Value>>,
) -> Result<String> {
    let original_config = frontmatter_config.cloned().unwrap_or_default();

    let mut mermaid_graph = if with_styles {
        let mut config_obj = match original_config.get("config") {
            Some(Value::Object(m)) => m.clone(),
            _ => serde_json::Map::new(),
        };
        let mut flowchart_obj = match config_obj.get("flowchart") {
            Some(Value::Object(m)) => m.clone(),
            _ => serde_json::Map::new(),
        };
        flowchart_obj.insert(
            "curve".to_string(),
            Value::String(curve_style.value().to_string()),
        );
        config_obj.insert("flowchart".to_string(), Value::Object(flowchart_obj));

        let mut full_config = original_config.clone();
        full_config.insert("config".to_string(), Value::Object(config_obj));

        let yaml_str = value_to_yaml(&serde_json::to_value(&full_config)?, 0);
        format!("---\n{}\n---\ngraph TD;\n", yaml_str)
    } else {
        "graph TD;\n".to_string()
    };

    let mut subgraph_nodes: HashMap<String, Vec<(&String, &Node)>> = HashMap::new();
    let mut regular_nodes: Vec<(&String, &Node)> = Vec::new();

    for (key, node) in nodes {
        if key.contains(':') {
            let parts: Vec<&str> = key.split(':').collect();
            let prefix = parts[..parts.len() - 1].join(":");
            subgraph_nodes.entry(prefix).or_default().push((key, node));
        } else {
            regular_nodes.push((key, node));
        }
    }

    regular_nodes.sort_by_key(|(key, _)| (*key).clone());

    let render_node = |key: &str, node: &Node, indent: &str| -> String {
        let node_name = node.name.split(':').next_back().unwrap_or(&node.name);
        let label = if node_name.starts_with(MARKDOWN_SPECIAL_CHARS)
            && node_name.ends_with(MARKDOWN_SPECIAL_CHARS)
        {
            format!("<p>{}</p>", node_name)
        } else {
            node_name.to_string()
        };

        let label = if let Some(ref metadata) = node.metadata {
            let meta_str: Vec<String> = metadata
                .iter()
                .map(|(k, v)| {
                    let val_str = match v {
                        Value::String(s) => s.clone(),
                        other => other.to_string(),
                    };
                    format!("{} = {}", k, val_str)
                })
                .collect();
            format!(
                "{}<hr/><small><em>{}</em></small>",
                label,
                meta_str.join("\n")
            )
        } else {
            label
        };

        let node_label = if Some(key) == first_node {
            format!("{}([{}]):::first", to_safe_id(key), label)
        } else if Some(key) == last_node {
            format!("{}([{}]):::last", to_safe_id(key), label)
        } else {
            format!("{}({})", to_safe_id(key), label)
        };

        format!("{}{}\n", indent, node_label)
    };

    if with_styles {
        for (key, node) in &regular_nodes {
            mermaid_graph += &render_node(key, node, "\t");
        }
    }

    let mut edge_groups: HashMap<String, Vec<&Edge>> = HashMap::new();
    for edge in edges {
        let src_parts: Vec<&str> = edge.source.split(':').collect();
        let tgt_parts: Vec<&str> = edge.target.split(':').collect();
        let common: Vec<&str> = src_parts
            .iter()
            .zip(tgt_parts.iter())
            .take_while(|(s, t)| s == t)
            .map(|(s, _)| *s)
            .collect();
        let common_prefix = common.join(":");
        edge_groups.entry(common_prefix).or_default().push(edge);
    }

    let mut seen_subgraphs: HashSet<String> = HashSet::new();

    #[allow(clippy::too_many_arguments)]
    fn add_subgraph(
        mermaid_graph: &mut String,
        edge_groups: &HashMap<String, Vec<&Edge>>,
        subgraph_nodes: &HashMap<String, Vec<(&String, &Node)>>,
        edges: &[&Edge],
        prefix: &str,
        _first_node: Option<&str>,
        _last_node: Option<&str>,
        with_styles: bool,
        wrap_label_n_words: usize,
        seen_subgraphs: &mut HashSet<String>,
        render_node: &dyn Fn(&str, &Node, &str) -> String,
    ) -> Result<()> {
        let self_loop = edges.len() == 1 && edges[0].source == edges[0].target;
        if !prefix.is_empty() && !self_loop {
            let subgraph = prefix.rsplit(':').next().unwrap_or(prefix);
            if seen_subgraphs.contains(subgraph) {
                return Err(Error::Other(format!(
                    "Found duplicate subgraph '{}' -- this likely means that you're reusing a subgraph node with the same name. Please adjust your graph to have subgraph nodes with unique names.",
                    subgraph
                )));
            }
            seen_subgraphs.insert(subgraph.to_string());
            mermaid_graph.push_str(&format!("\tsubgraph {}\n", subgraph));

            if with_styles && let Some(sub_nodes) = subgraph_nodes.get(prefix) {
                let mut sorted_nodes: Vec<_> = sub_nodes.clone();
                sorted_nodes.sort_by_key(|(key, _)| (*key).clone());
                for (key, node) in &sorted_nodes {
                    mermaid_graph.push_str(&render_node(key, node, "\t"));
                }
            }
        }

        for edge in edges {
            let edge_label = if let Some(ref data) = edge.data {
                let words: Vec<&str> = data.split_whitespace().collect();
                let wrapped = if words.len() > wrap_label_n_words {
                    words
                        .chunks(wrap_label_n_words)
                        .map(|chunk| chunk.join(" "))
                        .collect::<Vec<_>>()
                        .join("&nbsp<br>&nbsp")
                } else {
                    data.clone()
                };
                if edge.conditional {
                    format!(" -. &nbsp;{}&nbsp; .-> ", wrapped)
                } else {
                    format!(" -- &nbsp;{}&nbsp; --> ", wrapped)
                }
            } else if edge.conditional {
                " -.-> ".to_string()
            } else {
                " --> ".to_string()
            };

            mermaid_graph.push_str(&format!(
                "\t{}{}{};\n",
                to_safe_id(&edge.source),
                edge_label,
                to_safe_id(&edge.target)
            ));
        }

        let prefix_with_colon = if prefix.is_empty() {
            String::new()
        } else {
            format!("{}:", prefix)
        };

        let mut nested_prefixes: Vec<&String> = edge_groups
            .keys()
            .filter(|np| {
                if prefix.is_empty() {
                    return false;
                }
                np.starts_with(&prefix_with_colon)
                    && *np != prefix
                    && !np[prefix_with_colon.len()..].contains(':')
            })
            .collect();
        nested_prefixes.sort();

        for nested_prefix in nested_prefixes {
            if let Some(nested_edges) = edge_groups.get(nested_prefix.as_str()) {
                add_subgraph(
                    mermaid_graph,
                    edge_groups,
                    subgraph_nodes,
                    nested_edges,
                    nested_prefix,
                    _first_node,
                    _last_node,
                    with_styles,
                    wrap_label_n_words,
                    seen_subgraphs,
                    render_node,
                )?;
            }
        }

        if !prefix.is_empty() && !self_loop {
            mermaid_graph.push_str("\tend\n");
        }

        Ok(())
    }

    if let Some(top_edges) = edge_groups.get("") {
        add_subgraph(
            &mut mermaid_graph,
            &edge_groups,
            &subgraph_nodes,
            top_edges,
            "",
            first_node,
            last_node,
            with_styles,
            wrap_label_n_words,
            &mut seen_subgraphs,
            &render_node,
        )?;
    }

    let mut top_level_prefixes: Vec<&String> = edge_groups
        .keys()
        .filter(|p| !p.is_empty() && !p.contains(':') && !seen_subgraphs.contains(&p.to_string()))
        .collect();
    top_level_prefixes.sort();

    for prefix in top_level_prefixes {
        if let Some(prefix_edges) = edge_groups.get(prefix.as_str()) {
            add_subgraph(
                &mut mermaid_graph,
                &edge_groups,
                &subgraph_nodes,
                prefix_edges,
                prefix,
                first_node,
                last_node,
                with_styles,
                wrap_label_n_words,
                &mut seen_subgraphs,
                &render_node,
            )?;
        }
    }

    if with_styles {
        let mut empty_prefixes: Vec<&String> = subgraph_nodes
            .keys()
            .filter(|p| !p.contains(':') && !seen_subgraphs.contains(&p.to_string()))
            .collect();
        empty_prefixes.sort();

        for prefix in empty_prefixes {
            let subgraph = prefix.rsplit(':').next().unwrap_or(prefix);
            if seen_subgraphs.contains(subgraph) {
                return Err(Error::Other(format!(
                    "Found duplicate subgraph '{}' -- this likely means that you're reusing a subgraph node with the same name. Please adjust your graph to have subgraph nodes with unique names.",
                    subgraph
                )));
            }
            mermaid_graph.push_str(&format!("\tsubgraph {}\n", subgraph));
            if let Some(sub_nodes) = subgraph_nodes.get(prefix.as_str()) {
                let mut sorted = sub_nodes.clone();
                sorted.sort_by_key(|(key, _)| (*key).clone());
                for (key, node) in &sorted {
                    mermaid_graph.push_str(&render_node(key, node, "\t"));
                }
            }
            mermaid_graph.push_str("\tend\n");
            seen_subgraphs.insert(subgraph.to_string());
        }
    }

    if with_styles {
        mermaid_graph += &generate_mermaid_graph_styles(node_styles);
    }

    Ok(mermaid_graph)
}

pub fn generate_mermaid_graph_styles(node_colors: &NodeStyles) -> String {
    let mut styles = String::new();
    styles += &format!("\tclassDef default {}\n", node_colors.default);
    styles += &format!("\tclassDef first {}\n", node_colors.first);
    styles += &format!("\tclassDef last {}\n", node_colors.last);
    styles
}

#[builder]
pub async fn draw_mermaid_png(
    mermaid_syntax: &str,
    output_file_path: Option<&Path>,
    draw_method: MermaidDrawMethod,
    background_color: Option<&str>,
    #[builder(default = 3)] max_retries: usize,
    #[builder(default = 1.0)] retry_delay_secs: f64,
    base_url: Option<&str>,
) -> Result<Vec<u8>> {
    match draw_method {
        MermaidDrawMethod::Api => {
            render_mermaid_using_api(
                mermaid_syntax,
                output_file_path,
                background_color,
                max_retries,
                retry_delay_secs,
                base_url,
            )
            .await
        }
        MermaidDrawMethod::Pyppeteer => Err(Error::other(
            "Pyppeteer rendering is not available in Rust. Use MermaidDrawMethod::Api.",
        )),
    }
}

async fn render_mermaid_using_api(
    mermaid_syntax: &str,
    output_file_path: Option<&Path>,
    background_color: Option<&str>,
    max_retries: usize,
    retry_delay_secs: f64,
    base_url: Option<&str>,
) -> Result<Vec<u8>> {
    let base_url = base_url.unwrap_or("https://mermaid.ink");

    let encoded = base64::engine::general_purpose::STANDARD.encode(mermaid_syntax.as_bytes());

    let bg_color = match background_color {
        Some(color) => {
            let hex_pattern = regex::Regex::new(r"^#(?:[0-9a-fA-F]{3}){1,2}$")
                .map_err(|e| Error::other(format!("regex error: {e}")))?;
            if hex_pattern.is_match(color) {
                color.to_string()
            } else {
                format!("!{color}")
            }
        }
        None => "!white".to_string(),
    };

    let image_url = format!("{base_url}/img/{encoded}?type=png&bgColor={bg_color}");

    let client = reqwest::Client::new();

    for attempt in 0..=max_retries {
        match client
            .get(&image_url)
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
        {
            Ok(response) => {
                if response.status().is_success() {
                    let img_bytes = response
                        .bytes()
                        .await
                        .map_err(|e| Error::other(format!("Failed to read response bytes: {e}")))?
                        .to_vec();

                    if let Some(path) = output_file_path {
                        std::fs::write(path, &img_bytes)
                            .map_err(|e| Error::other(format!("Failed to write PNG file: {e}")))?;
                    }

                    return Ok(img_bytes);
                }

                let status = response.status().as_u16();

                if status >= 500 && attempt < max_retries {
                    let jitter = 0.5 + 0.5 * (attempt as f64 / max_retries.max(1) as f64);
                    let sleep_time = retry_delay_secs * (2.0_f64).powi(attempt as i32) * jitter;
                    tokio::time::sleep(std::time::Duration::from_secs_f64(sleep_time)).await;
                    continue;
                }

                return Err(Error::other(format!(
                    "Failed to reach {base_url} API while trying to render                      your graph. Status code: {status}.

                     To resolve this issue:
                     1. Check your internet connection and try again
                     2. Try with higher retry settings:                      `draw_mermaid_png(..., max_retries=5, retry_delay=2.0)`"
                )));
            }
            Err(e) => {
                if attempt < max_retries {
                    let jitter = 0.5 + 0.5 * (attempt as f64 / max_retries.max(1) as f64);
                    let sleep_time = retry_delay_secs * (2.0_f64).powi(attempt as i32) * jitter;
                    tokio::time::sleep(std::time::Duration::from_secs_f64(sleep_time)).await;
                } else {
                    return Err(Error::other(format!(
                        "Failed to reach {base_url} API while trying to render                          your graph after {max_retries} retries: {e}

                         To resolve this issue:
                         1. Check your internet connection and try again
                         2. Try with higher retry settings:                          `draw_mermaid_png(..., max_retries=5, retry_delay=2.0)`"
                    )));
                }
            }
        }
    }

    Err(Error::other(format!(
        "Failed to reach {base_url} API while trying to render          your graph after {max_retries} retries."
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_safe_id_basic() {
        assert_eq!(to_safe_id("foo"), "foo");
        assert_eq!(to_safe_id("foo-bar"), "foo-bar");
        assert_eq!(to_safe_id("foo_1"), "foo_1");
    }

    #[test]
    fn test_to_safe_id_special_chars() {
        assert_eq!(to_safe_id("#foo*&!"), "\\23foo\\2a\\26\\21");
    }
}
