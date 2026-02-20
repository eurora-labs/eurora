use std::collections::HashMap;

use agent_chain_core::runnables::graph::{Edge, Graph, MermaidOptions, Node};
use agent_chain_core::runnables::graph_mermaid::to_safe_id;
use serde_json::Value;

fn make_node(id: &str, name: &str) -> Node {
    Node::new(id, name)
}

fn make_node_with_metadata(id: &str, name: &str, metadata: HashMap<String, Value>) -> Node {
    Node::new(id, name).with_metadata(metadata)
}

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

#[test]
fn test_trim() {
    let mut graph = Graph::new();
    let start = graph.add_node_named("__start__", Some("__start__"));
    let ask = graph.add_node_named("ask_question", Some("ask_question"));
    let answer = graph.add_node_named("answer_question", Some("answer_question"));
    let end = graph.add_node_named("__end__", Some("__end__"));

    graph.add_edge(&start, &ask, None, false);
    graph.add_edge(&ask, &answer, None, false);
    graph.add_edge(&answer, &ask, None, true);
    graph.add_edge(&answer, &end, None, true);

    assert_eq!(graph.first_node().unwrap().id, "__start__");
    assert_eq!(graph.last_node().unwrap().id, "__end__");

    graph.trim_first_node();
    assert_eq!(graph.first_node().unwrap().id, "__start__");

    graph.trim_last_node();
    assert_eq!(graph.last_node().unwrap().id, "__end__");
}

#[test]
fn test_trim_basic() {
    let mut graph = Graph::new();
    let start = graph.add_node_named("__start__", Some("__start__"));
    let middle = graph.add_node_named("process", Some("process"));
    let end = graph.add_node_named("__end__", Some("__end__"));

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
    let start = graph.add_node_named("__start__", Some("__start__"));
    let ask = graph.add_node_named("ask_question", Some("ask_question"));
    let answer = graph.add_node_named("answer_question", Some("answer_question"));
    let end = graph.add_node_named("__end__", Some("__end__"));

    graph.add_edge(&start, &ask, None, false);
    graph.add_edge(&ask, &answer, None, false);
    graph.add_edge(&answer, &ask, None, true);
    graph.add_edge(&answer, &end, None, true);

    let json = graph.to_json();

    assert!(json.get("nodes").unwrap().is_array());
    assert!(json.get("edges").unwrap().is_array());
    assert_eq!(json["nodes"].as_array().unwrap().len(), 4);
    assert_eq!(json["edges"].as_array().unwrap().len(), 4);

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

#[test]
fn test_trim_multi_edge() {
    let mut graph = Graph::new();
    let start = graph.add_node_named("__start__", Some("__start__"));
    let a = graph.add_node_named("a", Some("a"));
    let last = graph.add_node_named("__end__", Some("__end__"));

    graph.add_edge(&start, &a, None, false);
    graph.add_edge(&a, &last, None, false);
    graph.add_edge(&start, &last, None, false);

    graph.trim_first_node();
    assert_eq!(graph.first_node().unwrap().id, "__start__");

    graph.trim_last_node();
    assert_eq!(graph.last_node().unwrap().id, "__end__");
}

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
    let mermaid = graph.draw_mermaid(None).unwrap();

    assert!(mermaid.contains("graph TD;"));
    assert!(mermaid.contains("subgraph inner_1"));
    assert!(mermaid.contains("subgraph inner_2"));
    assert!(mermaid.contains("__start__"));
    assert!(mermaid.contains("__end__"));
    assert!(mermaid.contains(" --> "));
    assert!(mermaid.contains("end"));
}

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
    let mermaid = graph.draw_mermaid(None).unwrap();

    assert!(mermaid.contains("graph TD;"));
    assert!(mermaid.contains("subgraph child_1"));
    assert!(
        mermaid.contains("subgraph child"),
        "Should have parent 'child' subgraph"
    );
    assert!(mermaid.contains("end"));
}

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
    let mermaid = graph.draw_mermaid(None).unwrap();

    assert!(mermaid.contains("graph TD;"));
    assert!(mermaid.contains("subgraph grandchild_1"));
    assert!(mermaid.contains("subgraph child_1"));
}

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
    let mermaid = graph.draw_mermaid(None).unwrap();

    assert!(mermaid.contains("graph TD;"));
    assert!(mermaid.contains("subgraph sub"));
    assert!(mermaid.contains("meow"));
    assert!(mermaid.contains("end"));
}

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

    let mermaid = graph
        .draw_mermaid(Some(MermaidOptions {
            frontmatter_config: Some(frontmatter),
            ..Default::default()
        }))
        .unwrap();

    assert!(mermaid.starts_with("---\n"));
    assert!(mermaid.contains("theme: neutral"));
    assert!(mermaid.contains("handDrawn"));
    assert!(mermaid.contains("primaryColor"));
    assert!(mermaid.contains("graph TD;"));
}

#[test]
fn test_graph_mermaid_special_chars() {
    let mut nodes = HashMap::new();
    nodes.insert("__start__".to_string(), make_node("__start__", "__start__"));
    nodes.insert("开始".to_string(), make_node("开始", "开始"));
    nodes.insert("结束".to_string(), make_node("结束", "结束"));
    nodes.insert("__end__".to_string(), make_node("__end__", "__end__"));

    let edges = vec![
        Edge::new("__start__", "开始"),
        Edge::new("开始", "结束"),
        Edge::new("结束", "__end__"),
    ];

    let graph = Graph::from_parts(nodes, edges);
    let mermaid = graph.draw_mermaid(None).unwrap();

    assert!(mermaid.contains("graph TD;"));
    assert!(mermaid.contains("\\"));
    assert!(mermaid.contains("开始"));
    assert!(mermaid.contains("结束"));
}

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
    let mermaid = graph
        .draw_mermaid(Some(MermaidOptions {
            with_styles: false,
            ..Default::default()
        }))
        .unwrap();

    assert!(mermaid.starts_with("graph TD;\n"));
    assert!(!mermaid.contains("---"));
    assert!(!mermaid.contains("classDef"));
}

#[test]
fn test_graph_add_node() {
    let mut graph = Graph::new();
    let node = graph.add_node_named("test_node", Some("my_id"));
    assert_eq!(node.id, "my_id");
    assert_eq!(node.name, "test_node");
    assert!(graph.nodes.contains_key("my_id"));
}

#[test]
fn test_graph_add_node_auto_id() {
    let mut graph = Graph::new();
    let node = graph.add_node_named("test_node", None);
    assert!(!node.id.is_empty());
    assert_eq!(node.name, "test_node");
    assert_eq!(graph.nodes.len(), 1);
}

#[test]
fn test_graph_add_edge() {
    let mut graph = Graph::new();
    let source = graph.add_node_named("source", Some("s"));
    let target = graph.add_node_named("target", Some("t"));
    let edge = graph.add_edge(&source, &target, None, false);
    assert_eq!(edge.source, "s");
    assert_eq!(edge.target, "t");
    assert!(!edge.conditional);
    assert_eq!(graph.edges.len(), 1);
}

#[test]
fn test_graph_remove_node() {
    let mut graph = Graph::new();
    let a = graph.add_node_named("a", Some("a"));
    let b = graph.add_node_named("b", Some("b"));
    let c = graph.add_node_named("c", Some("c"));
    graph.add_edge(&a, &b, None, false);
    graph.add_edge(&b, &c, None, false);

    graph.remove_node(&b);

    assert_eq!(graph.nodes.len(), 2);
    assert!(graph.edges.is_empty());
}

#[test]
fn test_graph_first_last_node() {
    let mut graph = Graph::new();
    let a = graph.add_node_named("a", Some("a"));
    let b = graph.add_node_named("b", Some("b"));
    let c = graph.add_node_named("c", Some("c"));
    graph.add_edge(&a, &b, None, false);
    graph.add_edge(&b, &c, None, false);

    assert_eq!(graph.first_node().unwrap().id, "a");
    assert_eq!(graph.last_node().unwrap().id, "c");
}

#[test]
fn test_graph_no_first_node_with_multiple_roots() {
    let mut graph = Graph::new();
    let a = graph.add_node_named("a", Some("a"));
    let b = graph.add_node_named("b", Some("b"));
    let c = graph.add_node_named("c", Some("c"));
    graph.add_edge(&a, &c, None, false);
    graph.add_edge(&b, &c, None, false);

    assert!(graph.first_node().is_none());
    assert_eq!(graph.last_node().unwrap().id, "c");
}

#[test]
fn test_graph_reid() {
    let mut graph = Graph::new();
    let a = graph.add_node_named("alpha", None);
    let b = graph.add_node_named("beta", None);
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
    let a = graph.add_node_named("a", Some("a"));
    let b = graph.add_node_named("b", Some("b"));
    graph.add_edge(&a, &b, None, true);

    let mermaid = graph.draw_mermaid(None).unwrap();
    assert!(
        mermaid.contains("-.->"),
        "Conditional edge should use dashed arrow"
    );
}

#[test]
fn test_graph_edge_with_data() {
    let mut graph = Graph::new();
    let a = graph.add_node_named("a", Some("a"));
    let b = graph.add_node_named("b", Some("b"));
    graph.add_edge(&a, &b, Some("my label".to_string()), false);

    let mermaid = graph.draw_mermaid(None).unwrap();
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
    let mermaid = graph.draw_mermaid(None).unwrap();

    assert!(mermaid.contains("__interrupt"));
    assert!(mermaid.contains("before"));
}

#[test]
fn test_graph_extend_basic() {
    let mut graph = Graph::new();
    let a = graph.add_node_named("a", Some("a"));
    let b = graph.add_node_named("b", Some("b"));
    graph.add_edge(&a, &b, None, false);

    let mut other = Graph::new();
    let c = other.add_node_named("c", Some("c"));
    let d = other.add_node_named("d", Some("d"));
    other.add_edge(&c, &d, None, false);

    let (first, last) = graph.extend(other, "");
    assert_eq!(first.unwrap().id, "c");
    assert_eq!(last.unwrap().id, "d");
    assert_eq!(graph.nodes.len(), 4);
    assert_eq!(graph.edges.len(), 2);
}

#[test]
fn test_graph_extend_with_prefix() {
    let mut graph = Graph::new();
    let a = graph.add_node_named("a", Some("a"));
    let _ = a;

    let mut other = Graph::new();
    let b = other.add_node_named("b", Some("b"));
    let c = other.add_node_named("c", Some("c"));
    other.add_edge(&b, &c, None, false);

    let (first, last) = graph.extend(other, "sub");
    assert_eq!(first.as_ref().unwrap().id, "sub:b");
    assert_eq!(last.as_ref().unwrap().id, "sub:c");
    assert!(graph.nodes.contains_key("sub:b"));
    assert!(graph.nodes.contains_key("sub:c"));
    assert_eq!(graph.edges.last().unwrap().source, "sub:b");
    assert_eq!(graph.edges.last().unwrap().target, "sub:c");
}

#[test]
fn test_graph_extend_uuid_nodes_ignore_prefix() {
    let mut graph = Graph::new();

    let mut other = Graph::new();
    let b = other.add_node_named("b", None);
    let c = other.add_node_named("c", None);
    other.add_edge(&b, &c, None, false);

    let (first, last) = graph.extend(other, "should_be_ignored");
    let first = first.unwrap();
    let last = last.unwrap();
    assert!(!first.id.contains("should_be_ignored"));
    assert!(!last.id.contains("should_be_ignored"));
}

#[test]
fn test_graph_extend_empty_graph() {
    let mut graph = Graph::new();
    let a = graph.add_node_named("a", Some("a"));
    let _ = a;

    let other = Graph::new();
    let (first, last) = graph.extend(other, "");
    assert!(first.is_none());
    assert!(last.is_none());
    assert_eq!(graph.nodes.len(), 1);
}

#[test]
fn test_graph_extend_returns_correct_first_last() {
    let mut graph = Graph::new();

    let mut other = Graph::new();
    let x = other.add_node_named("x", Some("x"));
    let y = other.add_node_named("y", Some("y"));
    let z = other.add_node_named("z", Some("z"));
    other.add_edge(&x, &y, None, false);
    other.add_edge(&y, &z, None, false);

    let (first, last) = graph.extend(other, "");
    assert_eq!(first.unwrap().id, "x");
    assert_eq!(last.unwrap().id, "z");
}

use agent_chain_core::runnables::base::{Runnable, runnable_lambda};

#[test]
fn test_get_graph_base_runnable() {
    let r = runnable_lambda(|x: String| Ok(x.len()));
    let graph = r.get_graph(None).unwrap();

    assert_eq!(graph.nodes.len(), 3);
    assert_eq!(graph.edges.len(), 2);

    assert!(graph.first_node().is_some());
    assert!(graph.last_node().is_some());
}

#[test]
fn test_get_graph_base_runnable_names() {
    let r = runnable_lambda(|x: String| Ok(x.len()));
    let graph = r.get_graph(None).unwrap();
    let reided = graph.reid();

    let names: Vec<&str> = reided.nodes.values().map(|n| n.name.as_str()).collect();
    assert!(
        names.iter().any(|n| n.contains("Input")),
        "Should have Input node, got: {:?}",
        names
    );
    assert!(
        names.iter().any(|n| n.contains("Output")),
        "Should have Output node, got: {:?}",
        names
    );
}

#[test]
fn test_get_graph_sequence() {
    use agent_chain_core::runnables::base::pipe;

    let a = runnable_lambda(|x: String| Ok(x.len()));
    let b = runnable_lambda(|x: usize| Ok(x.to_string()));
    let seq = pipe(a, b);
    let graph = seq.get_graph(None).unwrap();

    assert!(
        graph.nodes.len() >= 3,
        "Expected >= 3 nodes, got {}",
        graph.nodes.len()
    );
    assert!(
        graph.edges.len() >= 2,
        "Expected >= 2 edges, got {}",
        graph.edges.len()
    );

    assert!(graph.first_node().is_some());
    assert!(graph.last_node().is_some());
}

#[test]
fn test_get_graph_sequence_draws_mermaid() {
    use agent_chain_core::runnables::base::pipe;

    let a = runnable_lambda(|x: String| Ok(x.len()));
    let b = runnable_lambda(|x: usize| Ok(x.to_string()));
    let seq = pipe(a, b);
    let graph = seq.get_graph(None).unwrap();

    let mermaid = graph.draw_mermaid(None).unwrap();
    assert!(mermaid.contains("graph TD;"));
    assert!(mermaid.contains(" --> "));
}

#[test]
fn test_get_graph_parallel() {
    use agent_chain_core::runnables::base::RunnableParallel;

    let par = RunnableParallel::<String>::new()
        .add(
            "a",
            runnable_lambda(|x: String| Ok(serde_json::Value::String(x.clone()))),
        )
        .add(
            "b",
            runnable_lambda(|x: String| {
                Ok(serde_json::Value::Number(serde_json::Number::from(x.len())))
            }),
        );

    let graph = par.get_graph(None).unwrap();

    assert!(
        graph.nodes.len() >= 4,
        "Expected >= 4 nodes, got {}",
        graph.nodes.len()
    );

    assert!(
        graph.edges.len() >= 4,
        "Expected >= 4 edges, got {}",
        graph.edges.len()
    );

    assert!(graph.first_node().is_some());
    assert!(graph.last_node().is_some());
}

#[test]
fn test_get_graph_parallel_draws_mermaid() {
    use agent_chain_core::runnables::base::RunnableParallel;

    let par = RunnableParallel::<String>::new()
        .add(
            "a",
            runnable_lambda(|x: String| Ok(serde_json::Value::String(x.clone()))),
        )
        .add(
            "b",
            runnable_lambda(|x: String| {
                Ok(serde_json::Value::Number(serde_json::Number::from(x.len())))
            }),
        );

    let graph = par.get_graph(None).unwrap();
    let mermaid = graph.draw_mermaid(None).unwrap();

    assert!(mermaid.contains("graph TD;"));
}

#[test]
fn test_get_graph_binding_delegates() {
    let r = runnable_lambda(|x: String| Ok(x.len()));
    let binding = r.bind(HashMap::new());
    let graph = binding.get_graph(None).unwrap();

    assert_eq!(graph.nodes.len(), 3);
    assert_eq!(graph.edges.len(), 2);
}

use agent_chain_core::runnables::graph::{NodeData, node_data_json, node_data_str};

#[test]
fn test_node_data_str_with_uuid_and_schema() {
    let data = NodeData::Schema {
        name: "MyInput".to_string(),
    };
    let uuid_id = "550e8400-e29b-41d4-a716-446655440000";
    assert_eq!(node_data_str(uuid_id, Some(&data)), "MyInput");
}

#[test]
fn test_node_data_str_with_uuid_and_runnable() {
    let data = NodeData::Runnable {
        name: "RunnableLambda".to_string(),
    };
    let uuid_id = "550e8400-e29b-41d4-a716-446655440000";
    assert_eq!(node_data_str(uuid_id, Some(&data)), "Lambda");
}

#[test]
fn test_node_data_str_with_uuid_and_no_prefix() {
    let data = NodeData::Runnable {
        name: "ChatOpenAI".to_string(),
    };
    let uuid_id = "550e8400-e29b-41d4-a716-446655440000";
    assert_eq!(node_data_str(uuid_id, Some(&data)), "ChatOpenAI");
}

#[test]
fn test_node_data_str_with_non_uuid_returns_id() {
    let data = NodeData::Schema {
        name: "MyInput".to_string(),
    };
    assert_eq!(node_data_str("my_node", Some(&data)), "my_node");
}

#[test]
fn test_node_data_str_with_none_data_returns_id() {
    let uuid_id = "550e8400-e29b-41d4-a716-446655440000";
    assert_eq!(node_data_str(uuid_id, None), uuid_id);
}

#[test]
fn test_node_data_json_runnable() {
    let node = Node {
        id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
        name: "Lambda".to_string(),
        data: Some(NodeData::Runnable {
            name: "RunnableLambda".to_string(),
        }),
        metadata: None,
    };
    let json = node_data_json(&node);
    assert_eq!(json["type"], "runnable");
    assert_eq!(json["data"]["name"], "Lambda");
}

#[test]
fn test_node_data_json_schema() {
    let node = Node {
        id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
        name: "MyInput".to_string(),
        data: Some(NodeData::Schema {
            name: "MyInput".to_string(),
        }),
        metadata: None,
    };
    let json = node_data_json(&node);
    assert_eq!(json["type"], "schema");
    assert_eq!(json["data"], "MyInput");
}

#[test]
fn test_node_data_json_none() {
    let node = Node {
        id: "my_id".to_string(),
        name: "my_id".to_string(),
        data: None,
        metadata: None,
    };
    let json = node_data_json(&node);
    assert!(
        json.as_object().unwrap().is_empty() || !json.as_object().unwrap().contains_key("type")
    );
}

#[test]
fn test_node_data_json_with_metadata() {
    let mut meta = HashMap::new();
    meta.insert("key".to_string(), Value::String("value".to_string()));
    let node = Node {
        id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
        name: "Lambda".to_string(),
        data: Some(NodeData::Runnable {
            name: "RunnableLambda".to_string(),
        }),
        metadata: Some(meta),
    };
    let json = node_data_json(&node);
    assert_eq!(json["type"], "runnable");
    assert_eq!(json["metadata"]["key"], "value");
}

#[test]
fn test_add_node_with_schema_data() {
    let mut graph = Graph::new();
    let node = graph.add_node(
        Some(NodeData::Schema {
            name: "MyInput".to_string(),
        }),
        None,
        None,
    );
    assert_eq!(node.name, "MyInput");
    assert!(node.data.is_some());
}

#[test]
fn test_add_node_with_runnable_data() {
    let mut graph = Graph::new();
    let node = graph.add_node(
        Some(NodeData::Runnable {
            name: "RunnableLambda".to_string(),
        }),
        None,
        None,
    );
    assert_eq!(node.name, "Lambda");
}

#[test]
fn test_add_node_with_no_data() {
    let mut graph = Graph::new();
    let node = graph.add_node(None, Some("my_id"), None);
    assert_eq!(node.name, "my_id");
    assert!(node.data.is_none());
}

#[test]
fn test_to_json_includes_node_data_type() {
    let mut graph = Graph::new();
    let input = graph.add_node(
        Some(NodeData::Schema {
            name: "MyInput".to_string(),
        }),
        None,
        None,
    );
    let runnable = graph.add_node(
        Some(NodeData::Runnable {
            name: "RunnableLambda".to_string(),
        }),
        None,
        None,
    );
    graph.add_edge(&input, &runnable, None, false);

    let json = graph.to_json();
    let nodes = json["nodes"].as_array().unwrap();

    let schema_node = nodes
        .iter()
        .find(|n| n.get("type").and_then(|t| t.as_str()) == Some("schema"));
    assert!(schema_node.is_some(), "Should have a schema node");

    let runnable_node = nodes
        .iter()
        .find(|n| n.get("type").and_then(|t| t.as_str()) == Some("runnable"));
    assert!(runnable_node.is_some(), "Should have a runnable node");
}

#[test]
fn test_lambda_no_deps_default_graph() {
    let r = runnable_lambda(|x: String| Ok(x.len()));
    let graph = r.get_graph(None).unwrap();
    assert_eq!(graph.nodes.len(), 3);
    assert_eq!(graph.edges.len(), 2);
}

#[test]
fn test_lambda_with_deps_graph() {
    use agent_chain_core::runnables::base::RunnableGraphProvider;
    use std::sync::Arc;

    let dep = runnable_lambda(|x: String| Ok(x.to_uppercase()));
    let r = runnable_lambda(|x: String| Ok(x.len())).with_dep(Arc::new(RunnableGraphProvider(dep)));

    let graph = r.get_graph(None).unwrap();
    assert!(
        graph.nodes.len() >= 3,
        "Expected >= 3 nodes, got {}",
        graph.nodes.len()
    );
    assert!(
        graph.edges.len() >= 2,
        "Expected >= 2 edges, got {}",
        graph.edges.len()
    );

    assert!(graph.first_node().is_some());
    assert!(graph.last_node().is_some());
}

#[test]
fn test_lambda_with_multiple_deps_graph() {
    use agent_chain_core::runnables::base::RunnableGraphProvider;
    use std::sync::Arc;

    let dep1 = runnable_lambda(|x: String| Ok(x.to_uppercase()));
    let dep2 = runnable_lambda(|x: String| Ok(x.to_lowercase()));
    let r = runnable_lambda(|x: String| Ok(x.len())).with_deps(vec![
        Arc::new(RunnableGraphProvider(dep1)),
        Arc::new(RunnableGraphProvider(dep2)),
    ]);

    let graph = r.get_graph(None).unwrap();
    assert!(
        graph.nodes.len() >= 4,
        "Expected >= 4 nodes, got {}",
        graph.nodes.len()
    );
    assert!(
        graph.edges.len() >= 4,
        "Expected >= 4 edges, got {}",
        graph.edges.len()
    );
}

#[test]
fn test_lambda_with_deps_draws_mermaid() {
    use agent_chain_core::runnables::base::RunnableGraphProvider;
    use std::sync::Arc;

    let dep = runnable_lambda(|x: String| Ok(x.to_uppercase()));
    let r = runnable_lambda(|x: String| Ok(x.len())).with_dep(Arc::new(RunnableGraphProvider(dep)));

    let graph = r.get_graph(None).unwrap();
    let mermaid = graph.draw_mermaid(None).unwrap();
    assert!(mermaid.contains("graph TD;"));
    assert!(mermaid.contains(" --> "));
}
