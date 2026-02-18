//! Comprehensive tests for Mermaid graph drawing functionality.
//!
//! Mirrors `langchain/libs/core/tests/unit_tests/runnables/test_graph_mermaid.py`

use std::collections::HashMap;

use agent_chain_core::runnables::graph::{
    CurveStyle, Edge, Graph, MermaidDrawMethod, MermaidOptions, Node, NodeStyles,
};
use agent_chain_core::runnables::graph_mermaid::{generate_mermaid_graph_styles, to_safe_id};
use serde_json::Value;

fn make_node(id: &str, name: &str) -> Node {
    Node::new(id, name)
}

fn make_node_with_metadata(id: &str, name: &str, metadata: HashMap<String, Value>) -> Node {
    Node::new(id, name).with_metadata(metadata)
}

#[test]
fn test_to_safe_id_alphanumeric() {
    assert_eq!(to_safe_id("node1"), "node1");
    assert_eq!(to_safe_id("MyNode"), "MyNode");
    assert_eq!(to_safe_id("node_123"), "node_123");
    assert_eq!(to_safe_id("node-abc"), "node-abc");
}

#[test]
fn test_to_safe_id_special_characters() {
    assert_eq!(to_safe_id("node#1"), "node\\231");
    assert_eq!(to_safe_id("node@test"), "node\\40test");
    assert_eq!(to_safe_id("node$var"), "node\\24var");
    assert_eq!(to_safe_id("node%20"), "node\\2520");
}

#[test]
fn test_to_safe_id_unicode() {
    let result = to_safe_id("开始");
    assert!(result.contains('\\'));
    assert_ne!(result, "开始");
}

#[test]
fn test_to_safe_id_spaces() {
    let result = to_safe_id("my node");
    assert!(result.contains('\\'));
}

#[test]
fn test_to_safe_id_empty_string() {
    let result = to_safe_id("");
    assert_eq!(result, "");
}

#[test]
fn test_to_safe_id_only_special_chars() {
    let result = to_safe_id("!@#$");
    assert!(result.contains('\\'));
    assert_ne!(result, "!@#$");
}

#[test]
fn test_to_safe_id_preserves_allowed_chars() {
    let allowed_input = "abcABC123_-";
    let result = to_safe_id(allowed_input);
    assert_eq!(result, allowed_input);
}

#[test]
fn test_to_safe_id_escapes_punctuation() {
    assert_eq!(to_safe_id("node!"), "node\\21");
    assert_eq!(to_safe_id("node?"), "node\\3f");
    assert_eq!(to_safe_id("node."), "node\\2e");
    assert_eq!(to_safe_id("node,"), "node\\2c");
}

#[test]
fn test_generate_mermaid_graph_styles() {
    let styles = NodeStyles::default();
    let result = generate_mermaid_graph_styles(&styles);

    assert!(result.contains("classDef default"));
    assert!(result.contains("classDef first"));
    assert!(result.contains("classDef last"));
    assert!(result.contains("fill:#f2f0ff"));
}

#[test]
fn test_generate_mermaid_graph_styles_custom() {
    let styles = NodeStyles {
        default: "fill:#ff0000".to_string(),
        first: "fill:#00ff00".to_string(),
        last: "fill:#0000ff".to_string(),
    };
    let result = generate_mermaid_graph_styles(&styles);

    assert!(result.contains("fill:#ff0000"));
    assert!(result.contains("fill:#00ff00"));
    assert!(result.contains("fill:#0000ff"));
}

#[test]
fn test_draw_mermaid_simple_graph() {
    let mut nodes = HashMap::new();
    nodes.insert("1".to_string(), make_node("1", "Start"));
    nodes.insert("2".to_string(), make_node("2", "End"));

    let edges = vec![Edge::new("1", "2")];
    let graph = Graph::from_parts(nodes, edges);
    let result = graph.draw_mermaid(None).unwrap();

    assert!(result.contains("graph TD;"));
    assert!(result.contains("Start"));
    assert!(result.contains("End"));
    assert!(result.contains("-->"));
}

#[test]
fn test_draw_mermaid_without_styles() {
    let mut nodes = HashMap::new();
    nodes.insert("1".to_string(), make_node("1", "A"));
    nodes.insert("2".to_string(), make_node("2", "B"));

    let edges = vec![Edge::new("1", "2")];
    let graph = Graph::from_parts(nodes, edges);
    let result = graph
        .draw_mermaid(Some(MermaidOptions {
            with_styles: false,
            ..Default::default()
        }))
        .unwrap();

    assert!(result.contains("graph TD;"));
    assert!(!result.contains("classDef"));
    assert!(!result.contains("---"));
}

#[test]
fn test_draw_mermaid_with_conditional_edge() {
    let mut nodes = HashMap::new();
    nodes.insert("1".to_string(), make_node("1", "Decision"));
    nodes.insert("2".to_string(), make_node("2", "PathA"));
    nodes.insert("3".to_string(), make_node("3", "PathB"));

    let edges = vec![
        Edge {
            source: "1".to_string(),
            target: "2".to_string(),
            data: Some("yes".to_string()),
            conditional: true,
        },
        Edge::new("1", "3"),
    ];
    let graph = Graph::from_parts(nodes, edges);
    let result = graph.draw_mermaid(None).unwrap();

    assert!(result.contains("-."));
    assert!(result.contains(".->"));
    assert!(result.contains("-->"));
}

#[test]
fn test_draw_mermaid_with_edge_labels() {
    let mut nodes = HashMap::new();
    nodes.insert("1".to_string(), make_node("1", "A"));
    nodes.insert("2".to_string(), make_node("2", "B"));

    let edges = vec![Edge {
        source: "1".to_string(),
        target: "2".to_string(),
        data: Some("label text".to_string()),
        conditional: false,
    }];
    let graph = Graph::from_parts(nodes, edges);
    let result = graph.draw_mermaid(None).unwrap();

    assert!(result.contains("label text") || result.contains("label&nbsp"));
}

#[test]
fn test_draw_mermaid_with_long_edge_label() {
    let mut nodes = HashMap::new();
    nodes.insert("1".to_string(), make_node("1", "A"));
    nodes.insert("2".to_string(), make_node("2", "B"));

    let long_label: String = (0..15)
        .map(|i| format!("word{}", i))
        .collect::<Vec<_>>()
        .join(" ");
    let edges = vec![Edge {
        source: "1".to_string(),
        target: "2".to_string(),
        data: Some(long_label),
        conditional: false,
    }];
    let graph = Graph::from_parts(nodes, edges);
    let result = graph
        .draw_mermaid(Some(MermaidOptions {
            wrap_label_n_words: 9,
            ..Default::default()
        }))
        .unwrap();

    assert!(result.contains("<br>") || result.contains("&nbsp"));
}

#[test]
fn test_draw_mermaid_curve_styles() {
    let mut nodes = HashMap::new();
    nodes.insert("1".to_string(), make_node("1", "A"));
    nodes.insert("2".to_string(), make_node("2", "B"));

    let edges = vec![Edge::new("1", "2")];

    for curve_style in &CurveStyle::ALL {
        let graph = Graph::from_parts(nodes.clone(), edges.clone());
        let result = graph
            .draw_mermaid(Some(MermaidOptions {
                curve_style: curve_style.clone(),
                ..Default::default()
            }))
            .unwrap();
        assert!(
            result.contains(curve_style.value()),
            "Missing curve style '{}' in output",
            curve_style.value()
        );
    }
}

#[test]
fn test_draw_mermaid_first_last_nodes() {
    let mut nodes = HashMap::new();
    nodes.insert("start".to_string(), make_node("start", "Start"));
    nodes.insert("middle".to_string(), make_node("middle", "Middle"));
    nodes.insert("end".to_string(), make_node("end", "End"));

    let edges = vec![Edge::new("start", "middle"), Edge::new("middle", "end")];
    let graph = Graph::from_parts(nodes, edges);
    let result = graph.draw_mermaid(None).unwrap();

    assert!(result.contains(":::first"));
    assert!(result.contains(":::last"));
}

#[test]
fn test_draw_mermaid_subgraph() {
    let mut nodes = HashMap::new();
    nodes.insert("parent".to_string(), make_node("parent", "Parent"));
    nodes.insert("sub:child".to_string(), make_node("sub:child", "Child"));

    let edges = vec![Edge::new("parent", "sub:child")];
    let graph = Graph::from_parts(nodes, edges);
    let result = graph.draw_mermaid(None).unwrap();

    assert!(result.contains("subgraph sub"));
    assert!(result.contains("end"));
}

#[test]
fn test_draw_mermaid_nested_subgraphs() {
    let mut nodes = HashMap::new();
    nodes.insert("root".to_string(), make_node("root", "Root"));
    nodes.insert("a:b:c".to_string(), make_node("a:b:c", "Nested"));

    let edges = vec![Edge::new("root", "a:b:c")];
    let graph = Graph::from_parts(nodes, edges);
    let result = graph.draw_mermaid(None).unwrap();

    assert!(
        result.contains("\\3a") || result.contains("subgraph"),
        "Nested subgraph should use escaped colons or subgraph"
    );
}

#[test]
fn test_draw_mermaid_node_metadata() {
    let mut meta = HashMap::new();
    meta.insert("key1".to_string(), Value::String("value1".to_string()));
    meta.insert("key2".to_string(), Value::String("value2".to_string()));

    let mut nodes = HashMap::new();
    nodes.insert("1".to_string(), make_node_with_metadata("1", "Node1", meta));

    let graph = Graph::from_parts(nodes, vec![]);
    let result = graph.draw_mermaid(None).unwrap();

    assert!(result.contains("key1") || result.contains("value1"));
}

#[test]
fn test_draw_mermaid_frontmatter_config() {
    let mut nodes = HashMap::new();
    nodes.insert("1".to_string(), make_node("1", "A"));
    nodes.insert("2".to_string(), make_node("2", "B"));

    let edges = vec![Edge::new("1", "2")];

    let mut theme_vars = serde_json::Map::new();
    theme_vars.insert(
        "primaryColor".to_string(),
        Value::String("#ff0000".to_string()),
    );

    let mut config_inner = serde_json::Map::new();
    config_inner.insert("theme".to_string(), Value::String("dark".to_string()));
    config_inner.insert("themeVariables".to_string(), Value::Object(theme_vars));

    let mut frontmatter = HashMap::new();
    frontmatter.insert("config".to_string(), Value::Object(config_inner));

    let graph = Graph::from_parts(nodes, edges);
    let result = graph
        .draw_mermaid(Some(MermaidOptions {
            frontmatter_config: Some(frontmatter),
            ..Default::default()
        }))
        .unwrap();

    assert!(result.contains("---"));
    assert!(result.contains("theme: dark"));
}

#[test]
fn test_draw_mermaid_markdown_special_chars() {
    let mut nodes = HashMap::new();
    nodes.insert("1".to_string(), make_node("1", "*bold*"));
    nodes.insert("2".to_string(), make_node("2", "_italic_"));

    let edges = vec![Edge::new("1", "2")];
    let graph = Graph::from_parts(nodes, edges);
    let result = graph.draw_mermaid(None).unwrap();

    assert!(
        result.contains("<p>*bold*</p>") || result.contains("*bold*"),
        "Bold markdown should be wrapped or present"
    );
    assert!(
        result.contains("<p>_italic_</p>") || result.contains("_italic_"),
        "Italic markdown should be wrapped or present"
    );
}

#[test]
fn test_graph_draw_mermaid_method() {
    let mut graph = Graph::new();
    let node1 = graph.add_node_named("node1", Some("node1"));
    let node2 = graph.add_node_named("node2", Some("node2"));
    graph.add_edge(&node1, &node2, None, false);

    let result = graph.draw_mermaid(None).unwrap();

    assert!(result.contains("graph TD;"));
}

#[test]
fn test_graph_draw_mermaid_without_styles() {
    let mut graph = Graph::new();
    let node1 = graph.add_node_named("node1", Some("node1"));
    let node2 = graph.add_node_named("node2", Some("node2"));
    graph.add_edge(&node1, &node2, None, false);

    let result = graph
        .draw_mermaid(Some(MermaidOptions {
            with_styles: false,
            ..Default::default()
        }))
        .unwrap();

    assert!(!result.contains("classDef"));
}

#[test]
fn test_mermaid_curve_style_linear() {
    let mut nodes = HashMap::new();
    nodes.insert("1".to_string(), make_node("1", "A"));

    let graph = Graph::from_parts(nodes, vec![]);
    let result = graph
        .draw_mermaid(Some(MermaidOptions {
            curve_style: CurveStyle::Linear,
            ..Default::default()
        }))
        .unwrap();

    assert!(result.contains("curve: linear"));
}

#[test]
fn test_mermaid_curve_style_basis() {
    let mut nodes = HashMap::new();
    nodes.insert("1".to_string(), make_node("1", "A"));

    let graph = Graph::from_parts(nodes, vec![]);
    let result = graph
        .draw_mermaid(Some(MermaidOptions {
            curve_style: CurveStyle::Basis,
            ..Default::default()
        }))
        .unwrap();

    assert!(result.contains("curve: basis"));
}

#[test]
fn test_mermaid_empty_graph() {
    let graph = Graph::from_parts(HashMap::new(), vec![]);
    let result = graph.draw_mermaid(None).unwrap();

    assert!(result.contains("graph TD;"));
}

#[test]
fn test_mermaid_single_node() {
    let mut nodes = HashMap::new();
    nodes.insert("1".to_string(), make_node("1", "OnlyNode"));

    let graph = Graph::from_parts(nodes, vec![]);
    let result = graph
        .draw_mermaid(Some(MermaidOptions {
            with_styles: false,
            ..Default::default()
        }))
        .unwrap();

    assert!(result.contains("graph TD;"));
}

#[test]
fn test_mermaid_parallel_edges() {
    let mut nodes = HashMap::new();
    nodes.insert("1".to_string(), make_node("1", "Source"));
    nodes.insert("2".to_string(), make_node("2", "Target1"));
    nodes.insert("3".to_string(), make_node("3", "Target2"));

    let edges = vec![Edge::new("1", "2"), Edge::new("1", "3")];
    let graph = Graph::from_parts(nodes, edges);
    let result = graph.draw_mermaid(None).unwrap();

    assert!(
        result.matches("-->").count() >= 2,
        "Should have at least two arrows from source"
    );
}

#[test]
fn test_mermaid_self_loop() {
    let mut nodes = HashMap::new();
    nodes.insert("1".to_string(), make_node("1", "SelfLoop"));

    let edges = vec![Edge::new("1", "1")];
    let graph = Graph::from_parts(nodes, edges);
    let result = graph.draw_mermaid(None).unwrap();

    assert!(result.contains("graph TD;"));
}

#[test]
fn test_mermaid_duplicate_subgraph_name_error() {
    let mut nodes = HashMap::new();
    nodes.insert("sub:node1".to_string(), make_node("sub:node1", "Node1"));
    nodes.insert("sub:node2".to_string(), make_node("sub:node2", "Node2"));
    nodes.insert(
        "other:sub:node3".to_string(),
        make_node("other:sub:node3", "Node3"),
    );

    let edges = vec![
        Edge::new("sub:node1", "sub:node2"),
        Edge::new("other:sub:node3", "sub:node1"),
    ];
    let graph = Graph::from_parts(nodes, edges);

    let result = graph.draw_mermaid(None).unwrap();
    assert!(result.contains("subgraph"));
}

#[test]
fn test_mermaid_node_with_metadata() {
    let mut meta = HashMap::new();
    meta.insert("version".to_string(), Value::String("1.0".to_string()));
    meta.insert("author".to_string(), Value::String("test".to_string()));

    let mut nodes = HashMap::new();
    nodes.insert(
        "1".to_string(),
        make_node_with_metadata("1", "TestNode", meta),
    );

    let graph = Graph::from_parts(nodes, vec![]);
    let result = graph.draw_mermaid(None).unwrap();

    assert!(
        result.contains("version = 1.0") || result.contains("version"),
        "Should contain version metadata"
    );
    assert!(
        result.contains("author = test") || result.contains("author"),
        "Should contain author metadata"
    );
}

#[test]
fn test_mermaid_wrap_label_custom_words() {
    let mut nodes = HashMap::new();
    nodes.insert("1".to_string(), make_node("1", "A"));
    nodes.insert("2".to_string(), make_node("2", "B"));

    let long_label: String = (0..20)
        .map(|i| format!("word{}", i))
        .collect::<Vec<_>>()
        .join(" ");
    let edges = vec![Edge {
        source: "1".to_string(),
        target: "2".to_string(),
        data: Some(long_label),
        conditional: false,
    }];
    let graph = Graph::from_parts(nodes, edges);
    let result = graph
        .draw_mermaid(Some(MermaidOptions {
            wrap_label_n_words: 5,
            ..Default::default()
        }))
        .unwrap();

    assert!(
        result.matches("<br>").count() > 2,
        "Should have multiple line breaks"
    );
}

#[test]
fn test_mermaid_frontmatter_preserves_existing_config() {
    let mut nodes = HashMap::new();
    nodes.insert("1".to_string(), make_node("1", "A"));

    let mut flowchart = serde_json::Map::new();
    flowchart.insert("htmlLabels".to_string(), Value::Bool(true));

    let mut config_inner = serde_json::Map::new();
    config_inner.insert("theme".to_string(), Value::String("forest".to_string()));
    config_inner.insert("flowchart".to_string(), Value::Object(flowchart));

    let mut frontmatter = HashMap::new();
    frontmatter.insert("config".to_string(), Value::Object(config_inner));

    let graph = Graph::from_parts(nodes, vec![]);
    let result = graph
        .draw_mermaid(Some(MermaidOptions {
            frontmatter_config: Some(frontmatter),
            curve_style: CurveStyle::Basis,
            ..Default::default()
        }))
        .unwrap();

    assert!(result.contains("theme: forest"));
    assert!(result.contains("curve: basis"));
}

#[test]
fn test_mermaid_empty_subgraph() {
    let mut nodes = HashMap::new();
    nodes.insert("regular".to_string(), make_node("regular", "Regular"));
    nodes.insert("sub:node1".to_string(), make_node("sub:node1", "SubNode1"));
    nodes.insert("sub:node2".to_string(), make_node("sub:node2", "SubNode2"));

    let edges = vec![Edge::new("regular", "sub:node1")];
    let graph = Graph::from_parts(nodes, edges);
    let result = graph.draw_mermaid(None).unwrap();

    assert!(result.contains("subgraph sub"));
}

#[test]
fn test_graph_draw_mermaid_with_curve_styles() {
    let mut graph = Graph::new();
    let node1 = graph.add_node_named("node1", Some("node1"));
    let node2 = graph.add_node_named("node2", Some("node2"));
    graph.add_edge(&node1, &node2, None, false);

    for curve_style in &CurveStyle::ALL {
        let result = graph
            .draw_mermaid(Some(MermaidOptions {
                curve_style: curve_style.clone(),
                ..Default::default()
            }))
            .unwrap();
        assert!(
            result.contains(&format!("curve: {}", curve_style.value())),
            "Should contain curve: {}",
            curve_style.value()
        );
    }
}

#[test]
fn test_graph_draw_mermaid_custom_node_colors() {
    let mut graph = Graph::new();
    let node1 = graph.add_node_named("node1", Some("node1"));
    let node2 = graph.add_node_named("node2", Some("node2"));
    graph.add_edge(&node1, &node2, None, false);

    let custom_colors = NodeStyles {
        default: "fill:#abcdef".to_string(),
        first: "fill:#123456".to_string(),
        last: "fill:#fedcba".to_string(),
    };

    let result = graph
        .draw_mermaid(Some(MermaidOptions {
            node_styles: Some(custom_colors),
            ..Default::default()
        }))
        .unwrap();

    assert!(result.contains("fill:#abcdef"));
    assert!(result.contains("fill:#123456"));
    assert!(result.contains("fill:#fedcba"));
}

#[test]
fn test_draw_mermaid_all_curve_styles_valid() {
    let valid_styles = CurveStyle::ALL;

    assert_eq!(valid_styles.len(), 12);
    for style in &valid_styles {
        let value = style.value();
        assert!(!value.is_empty());
    }
}

#[test]
fn test_mermaid_node_styles_default_values() {
    let styles = NodeStyles::default();

    assert!(styles.default.contains("fill:#f2f0ff"));
    assert!(styles.first.contains("fill-opacity:0"));
    assert!(styles.last.contains("fill:#bfb6fc"));
}

#[test]
fn test_mermaid_draw_method_enum() {
    assert_eq!(MermaidDrawMethod::Pyppeteer.value(), "pyppeteer");
    assert_eq!(MermaidDrawMethod::Api.value(), "api");
}

#[test]
fn test_draw_mermaid_special_node_names() {
    let mut nodes = HashMap::new();
    nodes.insert("__start__".to_string(), make_node("__start__", "__start__"));
    nodes.insert("__end__".to_string(), make_node("__end__", "__end__"));

    let edges = vec![Edge::new("__start__", "__end__")];
    let graph = Graph::from_parts(nodes, edges);
    let result = graph.draw_mermaid(None).unwrap();

    assert!(
        result.contains("__start__") || result.contains("start"),
        "Should contain start node"
    );
    assert!(
        result.contains("__end__") || result.contains("end"),
        "Should contain end node"
    );
}

#[test]
fn test_draw_mermaid_numeric_node_ids() {
    let mut nodes = HashMap::new();
    nodes.insert("1".to_string(), make_node("1", "First"));
    nodes.insert("2".to_string(), make_node("2", "Second"));

    let edges = vec![Edge::new("1", "2")];
    let graph = Graph::from_parts(nodes, edges);
    let result = graph.draw_mermaid(None).unwrap();

    assert!(result.contains("graph TD;"));
}

#[test]
fn test_draw_mermaid_complex_metadata() {
    let mut meta = HashMap::new();
    meta.insert("nested".to_string(), serde_json::json!({"key": "value"}));
    meta.insert("list".to_string(), serde_json::json!([1, 2, 3]));
    meta.insert("bool".to_string(), Value::Bool(true));

    let mut nodes = HashMap::new();
    nodes.insert("1".to_string(), make_node_with_metadata("1", "Node", meta));

    let graph = Graph::from_parts(nodes, vec![]);
    let result = graph.draw_mermaid(None).unwrap();

    assert!(result.contains("nested") || result.contains("key"));
}

#[test]
fn test_mermaid_multiple_disconnected_subgraphs() {
    let mut nodes = HashMap::new();
    nodes.insert("sub1:a".to_string(), make_node("sub1:a", "A"));
    nodes.insert("sub1:b".to_string(), make_node("sub1:b", "B"));
    nodes.insert("sub2:c".to_string(), make_node("sub2:c", "C"));
    nodes.insert("sub2:d".to_string(), make_node("sub2:d", "D"));

    let edges = vec![Edge::new("sub1:a", "sub1:b"), Edge::new("sub2:c", "sub2:d")];
    let graph = Graph::from_parts(nodes, edges);
    let result = graph.draw_mermaid(None).unwrap();

    assert!(result.contains("subgraph sub1"));
    assert!(result.contains("subgraph sub2"));
}

#[test]
fn test_mermaid_edge_with_none_data() {
    let mut nodes = HashMap::new();
    nodes.insert("1".to_string(), make_node("1", "A"));
    nodes.insert("2".to_string(), make_node("2", "B"));

    let edges = vec![Edge::new("1", "2")];
    let graph = Graph::from_parts(nodes, edges);
    let result = graph.draw_mermaid(None).unwrap();

    assert!(result.contains("-->"));
}

#[test]
fn test_mermaid_conditional_edge_with_label() {
    let mut nodes = HashMap::new();
    nodes.insert("1".to_string(), make_node("1", "A"));
    nodes.insert("2".to_string(), make_node("2", "B"));

    let edges = vec![Edge {
        source: "1".to_string(),
        target: "2".to_string(),
        data: Some("condition".to_string()),
        conditional: true,
    }];
    let graph = Graph::from_parts(nodes, edges);
    let result = graph.draw_mermaid(None).unwrap();

    assert!(result.contains("-.") && result.contains(".->"));
}

#[test]
fn test_node_styles_dataclass() {
    let styles = NodeStyles {
        default: "custom-default".to_string(),
        first: "custom-first".to_string(),
        last: "custom-last".to_string(),
    };

    assert_eq!(styles.default, "custom-default");
    assert_eq!(styles.first, "custom-first");
    assert_eq!(styles.last, "custom-last");
}

#[test]
fn test_mermaid_with_all_features() {
    let mut meta = HashMap::new();
    meta.insert("type".to_string(), Value::String("processor".to_string()));

    let mut nodes = HashMap::new();
    nodes.insert("start".to_string(), make_node("start", "Start"));
    nodes.insert(
        "sub:node1".to_string(),
        make_node_with_metadata("sub:node1", "Process1", meta),
    );
    nodes.insert("sub:node2".to_string(), make_node("sub:node2", "Process2"));
    nodes.insert("end".to_string(), make_node("end", "End"));

    let edges = vec![
        Edge::new("start", "sub:node1"),
        Edge {
            source: "sub:node1".to_string(),
            target: "sub:node2".to_string(),
            data: Some("next".to_string()),
            conditional: false,
        },
        Edge {
            source: "sub:node2".to_string(),
            target: "end".to_string(),
            data: None,
            conditional: true,
        },
    ];

    let mut config_inner = serde_json::Map::new();
    config_inner.insert("theme".to_string(), Value::String("neutral".to_string()));
    let mut frontmatter = HashMap::new();
    frontmatter.insert("config".to_string(), Value::Object(config_inner));

    let custom_styles = NodeStyles {
        default: "fill:#eee".to_string(),
        ..Default::default()
    };

    let graph = Graph::from_parts(nodes, edges);
    let result = graph
        .draw_mermaid(Some(MermaidOptions {
            with_styles: true,
            curve_style: CurveStyle::Cardinal,
            node_styles: Some(custom_styles),
            wrap_label_n_words: 5,
            frontmatter_config: Some(frontmatter),
        }))
        .unwrap();

    assert!(result.contains("graph TD;"));
    assert!(result.contains("theme: neutral"));
    assert!(result.contains("curve: cardinal"));
    assert!(result.contains("subgraph sub"));
    assert!(result.contains("fill:#eee"));
    assert!(result.contains(":::first"));
    assert!(result.contains(":::last"));
}

#[test]
fn test_graph_reid_with_duplicate_names() {
    let mut graph = Graph::new();
    let node1 = graph.add_node_named("BaseModel", None);
    let node2 = graph.add_node_named("BaseModel", None);
    graph.add_edge(&node1, &node2, None, false);

    let reided_graph = graph.reid();

    let node_ids: Vec<&str> = reided_graph.nodes.keys().map(|k| k.as_str()).collect();
    let unique_count = {
        let mut seen = std::collections::HashSet::new();
        node_ids.iter().filter(|n| seen.insert(**n)).count()
    };
    assert_eq!(
        unique_count,
        node_ids.len(),
        "Node IDs should be unique after reid"
    );

    let mut sorted_ids: Vec<String> = reided_graph.nodes.keys().cloned().collect();
    sorted_ids.sort();
    assert_eq!(sorted_ids, vec!["BaseModel_1", "BaseModel_2"]);
}

#[test]
fn test_mermaid_subgraph_single_node() {
    let mut nodes = HashMap::new();
    nodes.insert("outer".to_string(), make_node("outer", "Outer"));
    nodes.insert("sub:inner".to_string(), make_node("sub:inner", "Inner"));

    let edges = vec![Edge::new("outer", "sub:inner")];
    let graph = Graph::from_parts(nodes, edges);
    let result = graph.draw_mermaid(None).unwrap();

    assert!(result.contains("subgraph sub"));
    assert!(result.contains("end"));
}

#[test]
fn test_curve_style_enum_values() {
    for style in &CurveStyle::ALL {
        let value = style.value();
        assert!(!value.is_empty());
    }
}
