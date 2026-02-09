//! Tests for graph data structures and Mermaid rendering.
//!
//! Mirrors `langchain/libs/core/tests/unit_tests/runnables/test_graph.py`

use std::collections::HashMap;

use agent_chain_core::runnables::graph::{Edge, Graph, MermaidOptions, Node};
use agent_chain_core::runnables::graph_mermaid::to_safe_id;
use serde_json::Value;

// ===========================================================================
// Helpers
// ===========================================================================

fn make_node(id: &str, name: &str) -> Node {
    Node::new(id, name)
}

fn make_node_with_metadata(id: &str, name: &str, metadata: HashMap<String, Value>) -> Node {
    Node::new(id, name).with_metadata(metadata)
}

// ===========================================================================
// Tests for _to_safe_id (mirrors test_graph_mermaid_to_safe_id)
// ===========================================================================

#[test]
fn test_to_safe_id_plain() {
    assert_eq!(to_safe_id("foo"), "foo");
}

#[test]
fn test_to_safe_id_with_hyphen() {
    assert_eq!(to_safe_id("foo-bar"), "foo-bar");
}

#[test]
fn test_to_safe_id_with_underscore_and_digit() {
    assert_eq!(to_safe_id("foo_1"), "foo_1");
}

#[test]
fn test_to_safe_id_special_chars() {
    assert_eq!(to_safe_id("#foo*&!"), "\\23foo\\2a\\26\\21");
}

// ===========================================================================
// Tests for Graph trim (mirrors test_trim)
// ===========================================================================

#[test]
fn test_trim() {
    let mut graph = Graph::new();
    let start = graph.add_node("__start__", Some("__start__"));
    let ask = graph.add_node("ask_question", Some("ask_question"));
    let answer = graph.add_node("answer_question", Some("answer_question"));
    let end = graph.add_node("__end__", Some("__end__"));

    graph.add_edge(&start, &ask, None, false);
    graph.add_edge(&ask, &answer, None, false);
    graph.add_edge(&answer, &ask, None, true);
    graph.add_edge(&answer, &end, None, true);

    assert_eq!(graph.first_node().unwrap().id, "__start__");
    assert_eq!(graph.last_node().unwrap().id, "__end__");

    // Can't trim __start__ or __end__ nodes
    graph.trim_first_node();
    assert_eq!(graph.first_node().unwrap().id, "__start__");

    graph.trim_last_node();
    assert_eq!(graph.last_node().unwrap().id, "__end__");
}

#[test]
fn test_trim_basic() {
    // A simple 3-node graph where start/end can't be trimmed (named nodes)
    let mut graph = Graph::new();
    let start = graph.add_node("__start__", Some("__start__"));
    let middle = graph.add_node("process", Some("process"));
    let end = graph.add_node("__end__", Some("__end__"));

    graph.add_edge(&start, &middle, None, false);
    graph.add_edge(&middle, &end, None, false);

    assert_eq!(graph.nodes.len(), 3);
    assert_eq!(graph.edges.len(), 2);
    assert_eq!(graph.first_node().unwrap().id, "__start__");
    assert_eq!(graph.last_node().unwrap().id, "__end__");
}

#[test]
fn test_trim_json_output() {
    let mut graph = Graph::new();
    let start = graph.add_node("__start__", Some("__start__"));
    let ask = graph.add_node("ask_question", Some("ask_question"));
    let answer = graph.add_node("answer_question", Some("answer_question"));
    let end = graph.add_node("__end__", Some("__end__"));

    graph.add_edge(&start, &ask, None, false);
    graph.add_edge(&ask, &answer, None, false);
    graph.add_edge(&answer, &ask, None, true);
    graph.add_edge(&answer, &end, None, true);

    let json = graph.to_json();

    // Verify structure
    assert!(json.get("nodes").unwrap().is_array());
    assert!(json.get("edges").unwrap().is_array());
    assert_eq!(json["nodes"].as_array().unwrap().len(), 4);
    assert_eq!(json["edges"].as_array().unwrap().len(), 4);

    // Verify conditional edges are marked
    let edges = json["edges"].as_array().unwrap();
    let conditional_edges: Vec<_> = edges
        .iter()
        .filter(|e| {
            e.get("conditional")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
        })
        .collect();
    assert_eq!(conditional_edges.len(), 2);
}

// ===========================================================================
// Tests for trim_multi_edge (mirrors test_trim_multi_edge)
// ===========================================================================

#[test]
fn test_trim_multi_edge() {
    let mut graph = Graph::new();
    let start = graph.add_node("__start__", Some("__start__"));
    let a = graph.add_node("a", Some("a"));
    let last = graph.add_node("__end__", Some("__end__"));

    graph.add_edge(&start, &a, None, false);
    graph.add_edge(&a, &last, None, false);
    graph.add_edge(&start, &last, None, false);

    // trim_first_node should not remove __start__ since it has 2 outgoing edges
    graph.trim_first_node();
    assert_eq!(graph.first_node().unwrap().id, "__start__");

    // trim_last_node should not remove __end__ since it has 2 incoming edges
    graph.trim_last_node();
    assert_eq!(graph.last_node().unwrap().id, "__end__");
}

// ===========================================================================
// Tests for parallel subgraph Mermaid (mirrors test_parallel_subgraph_mermaid)
// ===========================================================================

#[test]
fn test_parallel_subgraph_mermaid() {
    let mut nodes = HashMap::new();
    nodes.insert("__start__".to_string(), make_node("__start__", "__start__"));
    nodes.insert("outer_1".to_string(), make_node("outer_1", "outer_1"));
    nodes.insert(
        "inner_1:inner_1".to_string(),
        make_node("inner_1:inner_1", "inner_1"),
    );
    let mut interrupt_meta = HashMap::new();
    interrupt_meta.insert(
        "__interrupt".to_string(),
        Value::String("before".to_string()),
    );
    nodes.insert(
        "inner_1:inner_2".to_string(),
        make_node_with_metadata("inner_1:inner_2", "inner_2", interrupt_meta),
    );
    nodes.insert(
        "inner_2:inner_1".to_string(),
        make_node("inner_2:inner_1", "inner_1"),
    );
    nodes.insert(
        "inner_2:inner_2".to_string(),
        make_node("inner_2:inner_2", "inner_2"),
    );
    nodes.insert("outer_2".to_string(), make_node("outer_2", "outer_2"));
    nodes.insert("__end__".to_string(), make_node("__end__", "__end__"));

    let edges = vec![
        Edge::new("inner_1:inner_1", "inner_1:inner_2"),
        Edge::new("inner_2:inner_1", "inner_2:inner_2"),
        Edge::new("__start__", "outer_1"),
        Edge::new("inner_1:inner_2", "outer_2"),
        Edge::new("inner_2:inner_2", "outer_2"),
        Edge::new("outer_1", "inner_1:inner_1"),
        Edge::new("outer_1", "inner_2:inner_1"),
        Edge::new("outer_2", "__end__"),
    ];

    let graph = Graph::from_parts(nodes, edges);
    let mermaid = graph.draw_mermaid(None);

    // Verify key structural elements
    assert!(mermaid.contains("graph TD;"));
    assert!(mermaid.contains("subgraph inner_1"));
    assert!(mermaid.contains("subgraph inner_2"));
    assert!(mermaid.contains("__start__"));
    assert!(mermaid.contains("__end__"));
    assert!(mermaid.contains(" --> "));
    assert!(mermaid.contains("end"));
}

// ===========================================================================
// Tests for double nested subgraph (mirrors test_double_nested_subgraph_mermaid)
// ===========================================================================

#[test]
fn test_double_nested_subgraph_mermaid() {
    let mut nodes = HashMap::new();
    nodes.insert("__start__".to_string(), make_node("__start__", "__start__"));
    nodes.insert("parent_1".to_string(), make_node("parent_1", "parent_1"));
    nodes.insert(
        "child:child_1:grandchild_1".to_string(),
        make_node("child:child_1:grandchild_1", "grandchild_1"),
    );
    let mut interrupt_meta = HashMap::new();
    interrupt_meta.insert(
        "__interrupt".to_string(),
        Value::String("before".to_string()),
    );
    nodes.insert(
        "child:child_1:grandchild_2".to_string(),
        make_node_with_metadata("child:child_1:grandchild_2", "grandchild_2", interrupt_meta),
    );
    nodes.insert(
        "child:child_2".to_string(),
        make_node("child:child_2", "child_2"),
    );
    nodes.insert("parent_2".to_string(), make_node("parent_2", "parent_2"));
    nodes.insert("__end__".to_string(), make_node("__end__", "__end__"));

    let edges = vec![
        Edge::new("child:child_1:grandchild_1", "child:child_1:grandchild_2"),
        Edge::new("child:child_1:grandchild_2", "child:child_2"),
        Edge::new("__start__", "parent_1"),
        Edge::new("child:child_2", "parent_2"),
        Edge::new("parent_1", "child:child_1:grandchild_1"),
        Edge::new("parent_2", "__end__"),
    ];

    let graph = Graph::from_parts(nodes, edges);
    let mermaid = graph.draw_mermaid(None);

    assert!(mermaid.contains("graph TD;"));
    assert!(mermaid.contains("subgraph child_1"));
    assert!(
        mermaid.contains("subgraph child"),
        "Should have parent 'child' subgraph"
    );
    assert!(mermaid.contains("end"));
}

// ===========================================================================
// Tests for triple nested subgraph (mirrors test_triple_nested_subgraph_mermaid)
// ===========================================================================

#[test]
fn test_triple_nested_subgraph_mermaid() {
    let mut nodes = HashMap::new();
    nodes.insert("__start__".to_string(), make_node("__start__", "__start__"));
    nodes.insert("parent_1".to_string(), make_node("parent_1", "parent_1"));
    nodes.insert(
        "child:child_1:grandchild_1".to_string(),
        make_node("child:child_1:grandchild_1", "grandchild_1"),
    );
    nodes.insert(
        "child:child_1:grandchild_1:greatgrandchild".to_string(),
        make_node(
            "child:child_1:grandchild_1:greatgrandchild",
            "greatgrandchild",
        ),
    );
    let mut interrupt_meta = HashMap::new();
    interrupt_meta.insert(
        "__interrupt".to_string(),
        Value::String("before".to_string()),
    );
    nodes.insert(
        "child:child_1:grandchild_2".to_string(),
        make_node_with_metadata("child:child_1:grandchild_2", "grandchild_2", interrupt_meta),
    );
    nodes.insert(
        "child:child_2".to_string(),
        make_node("child:child_2", "child_2"),
    );
    nodes.insert("parent_2".to_string(), make_node("parent_2", "parent_2"));
    nodes.insert("__end__".to_string(), make_node("__end__", "__end__"));

    let edges = vec![
        Edge::new(
            "child:child_1:grandchild_1",
            "child:child_1:grandchild_1:greatgrandchild",
        ),
        Edge::new(
            "child:child_1:grandchild_1:greatgrandchild",
            "child:child_1:grandchild_2",
        ),
        Edge::new("child:child_1:grandchild_2", "child:child_2"),
        Edge::new("__start__", "parent_1"),
        Edge::new("child:child_2", "parent_2"),
        Edge::new("parent_1", "child:child_1:grandchild_1"),
        Edge::new("parent_2", "__end__"),
    ];

    let graph = Graph::from_parts(nodes, edges);
    let mermaid = graph.draw_mermaid(None);

    assert!(mermaid.contains("graph TD;"));
    assert!(mermaid.contains("subgraph grandchild_1"));
    assert!(mermaid.contains("subgraph child_1"));
}

// ===========================================================================
// Tests for single node subgraph (mirrors test_single_node_subgraph_mermaid)
// ===========================================================================

#[test]
fn test_single_node_subgraph_mermaid() {
    let mut nodes = HashMap::new();
    nodes.insert("__start__".to_string(), make_node("__start__", "__start__"));
    nodes.insert("sub:meow".to_string(), make_node("sub:meow", "meow"));
    nodes.insert("__end__".to_string(), make_node("__end__", "__end__"));

    let edges = vec![
        Edge::new("__start__", "sub:meow"),
        Edge::new("sub:meow", "__end__"),
    ];

    let graph = Graph::from_parts(nodes, edges);
    let mermaid = graph.draw_mermaid(None);

    assert!(mermaid.contains("graph TD;"));
    assert!(mermaid.contains("subgraph sub"));
    assert!(mermaid.contains("meow"));
    assert!(mermaid.contains("end"));
}

// ===========================================================================
// Tests for frontmatter config (mirrors test_graph_mermaid_frontmatter_config)
// ===========================================================================

#[test]
fn test_graph_mermaid_frontmatter_config() {
    let mut nodes = HashMap::new();
    nodes.insert("__start__".to_string(), make_node("__start__", "__start__"));
    nodes.insert("my_node".to_string(), make_node("my_node", "my_node"));

    let edges = vec![Edge::new("__start__", "my_node")];

    let graph = Graph::from_parts(nodes, edges);

    let mut theme_vars = serde_json::Map::new();
    theme_vars.insert(
        "primaryColor".to_string(),
        Value::String("#e2e2e2".to_string()),
    );

    let mut config_inner = serde_json::Map::new();
    config_inner.insert("theme".to_string(), Value::String("neutral".to_string()));
    config_inner.insert("look".to_string(), Value::String("handDrawn".to_string()));
    config_inner.insert("themeVariables".to_string(), Value::Object(theme_vars));

    let mut frontmatter = HashMap::new();
    frontmatter.insert("config".to_string(), Value::Object(config_inner));

    let mermaid = graph.draw_mermaid(Some(MermaidOptions {
        frontmatter_config: Some(frontmatter),
        ..Default::default()
    }));

    // Verify frontmatter is present
    assert!(mermaid.starts_with("---\n"));
    assert!(mermaid.contains("theme: neutral"));
    assert!(mermaid.contains("handDrawn"));
    assert!(mermaid.contains("primaryColor"));
    assert!(mermaid.contains("graph TD;"));
}

// ===========================================================================
// Tests for special characters (mirrors test_graph_mermaid_special_chars)
// ===========================================================================

#[test]
fn test_graph_mermaid_special_chars() {
    let mut nodes = HashMap::new();
    nodes.insert("__start__".to_string(), make_node("__start__", "__start__"));
    // Chinese characters
    nodes.insert("开始".to_string(), make_node("开始", "开始"));
    nodes.insert("结束".to_string(), make_node("结束", "结束"));
    nodes.insert("__end__".to_string(), make_node("__end__", "__end__"));

    let edges = vec![
        Edge::new("__start__", "开始"),
        Edge::new("开始", "结束"),
        Edge::new("结束", "__end__"),
    ];

    let graph = Graph::from_parts(nodes, edges);
    let mermaid = graph.draw_mermaid(None);

    assert!(mermaid.contains("graph TD;"));
    // Chinese characters should be escaped to safe ids
    assert!(mermaid.contains("\\"));
    // The node labels should still contain the Chinese characters
    assert!(mermaid.contains("开始"));
    assert!(mermaid.contains("结束"));
}

// ===========================================================================
// Tests for draw_mermaid without styles (mirrors with_styles=False usage)
// ===========================================================================

#[test]
fn test_draw_mermaid_without_styles() {
    let mut nodes = HashMap::new();
    nodes.insert("__start__".to_string(), make_node("__start__", "__start__"));
    nodes.insert("my_node".to_string(), make_node("my_node", "my_node"));
    nodes.insert("__end__".to_string(), make_node("__end__", "__end__"));

    let edges = vec![
        Edge::new("__start__", "my_node"),
        Edge::new("my_node", "__end__"),
    ];

    let graph = Graph::from_parts(nodes, edges);
    let mermaid = graph.draw_mermaid(Some(MermaidOptions {
        with_styles: false,
        ..Default::default()
    }));

    // Without styles: no frontmatter, no classDef
    assert!(mermaid.starts_with("graph TD;\n"));
    assert!(!mermaid.contains("---"));
    assert!(!mermaid.contains("classDef"));
}

// ===========================================================================
// Tests for Graph API basics
// ===========================================================================

#[test]
fn test_graph_add_node() {
    let mut graph = Graph::new();
    let node = graph.add_node("test_node", Some("my_id"));
    assert_eq!(node.id, "my_id");
    assert_eq!(node.name, "test_node");
    assert!(graph.nodes.contains_key("my_id"));
}

#[test]
fn test_graph_add_node_auto_id() {
    let mut graph = Graph::new();
    let node = graph.add_node("test_node", None);
    assert!(!node.id.is_empty());
    assert_eq!(node.name, "test_node");
    assert_eq!(graph.nodes.len(), 1);
}

#[test]
fn test_graph_add_edge() {
    let mut graph = Graph::new();
    let source = graph.add_node("source", Some("s"));
    let target = graph.add_node("target", Some("t"));
    let edge = graph.add_edge(&source, &target, None, false);
    assert_eq!(edge.source, "s");
    assert_eq!(edge.target, "t");
    assert!(!edge.conditional);
    assert_eq!(graph.edges.len(), 1);
}

#[test]
fn test_graph_remove_node() {
    let mut graph = Graph::new();
    let a = graph.add_node("a", Some("a"));
    let b = graph.add_node("b", Some("b"));
    let c = graph.add_node("c", Some("c"));
    graph.add_edge(&a, &b, None, false);
    graph.add_edge(&b, &c, None, false);

    graph.remove_node(&b);

    assert_eq!(graph.nodes.len(), 2);
    assert!(graph.edges.is_empty());
}

#[test]
fn test_graph_first_last_node() {
    let mut graph = Graph::new();
    let a = graph.add_node("a", Some("a"));
    let b = graph.add_node("b", Some("b"));
    let c = graph.add_node("c", Some("c"));
    graph.add_edge(&a, &b, None, false);
    graph.add_edge(&b, &c, None, false);

    assert_eq!(graph.first_node().unwrap().id, "a");
    assert_eq!(graph.last_node().unwrap().id, "c");
}

#[test]
fn test_graph_no_first_node_with_multiple_roots() {
    let mut graph = Graph::new();
    let a = graph.add_node("a", Some("a"));
    let b = graph.add_node("b", Some("b"));
    let c = graph.add_node("c", Some("c"));
    graph.add_edge(&a, &c, None, false);
    graph.add_edge(&b, &c, None, false);

    // Both a and b are roots — no single first node
    assert!(graph.first_node().is_none());
    assert_eq!(graph.last_node().unwrap().id, "c");
}

#[test]
fn test_graph_reid() {
    let mut graph = Graph::new();
    // Use UUID-like ids
    let a = graph.add_node("alpha", None);
    let b = graph.add_node("beta", None);
    graph.add_edge(&a, &b, None, false);

    let reided = graph.reid();
    assert!(reided.nodes.contains_key("alpha"));
    assert!(reided.nodes.contains_key("beta"));
    assert_eq!(reided.edges.len(), 1);
    assert_eq!(reided.edges[0].source, "alpha");
    assert_eq!(reided.edges[0].target, "beta");
}

#[test]
fn test_graph_conditional_edge() {
    let mut graph = Graph::new();
    let a = graph.add_node("a", Some("a"));
    let b = graph.add_node("b", Some("b"));
    graph.add_edge(&a, &b, None, true);

    let mermaid = graph.draw_mermaid(None);
    assert!(
        mermaid.contains("-.->"),
        "Conditional edge should use dashed arrow"
    );
}

#[test]
fn test_graph_edge_with_data() {
    let mut graph = Graph::new();
    let a = graph.add_node("a", Some("a"));
    let b = graph.add_node("b", Some("b"));
    graph.add_edge(&a, &b, Some("my label".to_string()), false);

    let mermaid = graph.draw_mermaid(None);
    assert!(
        mermaid.contains("my label"),
        "Edge data should appear as label"
    );
}

#[test]
fn test_node_with_metadata_renders_in_mermaid() {
    let mut nodes = HashMap::new();
    let mut meta = HashMap::new();
    meta.insert(
        "__interrupt".to_string(),
        Value::String("before".to_string()),
    );
    nodes.insert(
        "my_node".to_string(),
        make_node_with_metadata("my_node", "my_node", meta),
    );
    nodes.insert("other".to_string(), make_node("other", "other"));

    let edges = vec![Edge::new("other", "my_node")];

    let graph = Graph::from_parts(nodes, edges);
    let mermaid = graph.draw_mermaid(None);

    assert!(mermaid.contains("__interrupt"));
    assert!(mermaid.contains("before"));
}
