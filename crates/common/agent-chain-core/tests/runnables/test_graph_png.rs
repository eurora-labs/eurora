use std::collections::HashMap;

use agent_chain_core::runnables::graph::{Graph, LabelsDict};
use agent_chain_core::runnables::graph_png::{PngDrawError, PngDrawer};

#[test]
fn test_png_drawer_initialization() {
    let drawer = PngDrawer::default();
    assert_eq!(drawer.fontname, "arial");
    assert_eq!(drawer.labels, LabelsDict::default());
}

#[test]
fn test_png_drawer_initialization_custom() {
    let custom_labels = LabelsDict {
        nodes: HashMap::from([("node1".into(), "CustomNode1".into())]),
        edges: HashMap::from([("edge1".into(), "CustomEdge1".into())]),
    };
    let drawer = PngDrawer::builder()
        .maybe_fontname(Some("helvetica").map(|s: &str| s.to_string()))
        .maybe_labels(Some(custom_labels))
        .build();

    assert_eq!(drawer.fontname, "helvetica");
    assert_eq!(drawer.labels.nodes["node1"], "CustomNode1");
    assert_eq!(drawer.labels.edges["edge1"], "CustomEdge1");
}

#[test]
fn test_png_drawer_get_node_label_default() {
    let drawer = PngDrawer::default();
    let label = drawer.get_node_label("test_node");
    assert!(label.contains("<B>test_node</B>"));
}

#[test]
fn test_png_drawer_get_node_label_custom() {
    let custom_labels = LabelsDict {
        nodes: HashMap::from([("test_node".into(), "Custom Label".into())]),
        edges: HashMap::new(),
    };
    let drawer = PngDrawer::builder()
        .maybe_fontname(None.map(|s: &str| s.to_string()))
        .maybe_labels(Some(custom_labels))
        .build();
    let label = drawer.get_node_label("test_node");
    assert!(label.contains("<B>Custom Label</B>"));
}

#[test]
fn test_png_drawer_get_edge_label_default() {
    let drawer = PngDrawer::default();
    let label = drawer.get_edge_label("test_edge");
    assert!(label.contains("<U>test_edge</U>"));
}

#[test]
fn test_png_drawer_get_edge_label_custom() {
    let custom_labels = LabelsDict {
        nodes: HashMap::new(),
        edges: HashMap::from([("test_edge".into(), "Custom Edge".into())]),
    };
    let drawer = PngDrawer::builder()
        .maybe_fontname(None.map(|s: &str| s.to_string()))
        .maybe_labels(Some(custom_labels))
        .build();
    let label = drawer.get_edge_label("test_edge");
    assert!(label.contains("<U>Custom Edge</U>"));
}

#[test]
fn test_graph_draw_png_returns_missing_dependency_error() {
    let mut graph = Graph::new();
    let node1 = graph.add_node_named("node1", Some("node1"));
    let node2 = graph.add_node_named("node2", Some("node2"));
    graph.add_edge(&node1, &node2, None, false);

    let result = graph.draw_png(None, None, None);
    assert!(result.is_err());
    match result.unwrap_err() {
        PngDrawError::MissingDependency(msg) => {
            assert!(msg.to_lowercase().contains("graphviz"));
        }
        other => panic!("expected MissingDependency, got: {other}"),
    }
}

#[test]
fn test_png_drawer_add_node_structure() {
    let drawer = PngDrawer::default();
    let attrs = drawer.node_attrs("test");
    assert_eq!(attrs["fillcolor"], "yellow");
    assert_eq!(attrs["style"], "filled");
    assert_eq!(attrs["fontsize"], "15");
    assert_eq!(attrs["fontname"], "arial");
}

#[test]
fn test_png_drawer_add_edge_structure() {
    let drawer = PngDrawer::default();
    let attrs = drawer.edge_attrs(Some("label"), false);
    assert_eq!(attrs["style"], "solid");
    assert_eq!(attrs["fontsize"], "12");
}

#[test]
fn test_png_drawer_draw_method() {
    let drawer = PngDrawer::default();
    let graph = Graph::new();
    let result = drawer.draw(&graph, None);
    assert!(result.is_err());
}

#[test]
fn test_png_drawer_with_empty_labels() {
    let labels = LabelsDict::default();
    let drawer = PngDrawer::builder()
        .maybe_fontname(None.map(|s: &str| s.to_string()))
        .maybe_labels(Some(labels))
        .build();

    assert_eq!(drawer.get_node_label("test"), "<<B>test</B>>");
    assert_eq!(drawer.get_edge_label("test"), "<<U>test</U>>");
}

#[test]
fn test_png_drawer_labels_dict_structure() {
    let labels = LabelsDict {
        nodes: HashMap::from([("n1".into(), "Node1".into()), ("n2".into(), "Node2".into())]),
        edges: HashMap::from([("e1".into(), "Edge1".into()), ("e2".into(), "Edge2".into())]),
    };

    assert!(labels.nodes.contains_key("n1"));
    assert!(labels.edges.contains_key("e1"));
    assert_eq!(labels.nodes["n1"], "Node1");
    assert_eq!(labels.edges["e1"], "Edge1");
}

#[test]
fn test_graph_draw_png_with_labels() {
    let mut graph = Graph::new();
    let node1 = graph.add_node_named("node1", Some("node1"));
    let node2 = graph.add_node_named("node2", Some("node2"));
    graph.add_edge(&node1, &node2, None, false);

    let custom_labels = LabelsDict {
        nodes: HashMap::from([
            ("node1".into(), "Start Node".into()),
            ("node2".into(), "End Node".into()),
        ]),
        edges: HashMap::new(),
    };

    let result = graph.draw_png(None, None, Some(custom_labels));
    assert!(matches!(result, Err(PngDrawError::MissingDependency(_))));
}

#[test]
fn test_graph_draw_png_with_fontname() {
    let mut graph = Graph::new();
    let node1 = graph.add_node_named("node1", Some("node1"));
    let node2 = graph.add_node_named("node2", Some("node2"));
    graph.add_edge(&node1, &node2, None, false);

    let result = graph.draw_png(None, Some("courier"), None);
    assert!(matches!(result, Err(PngDrawError::MissingDependency(_))));
}

#[test]
fn test_png_drawer_add_nodes_method() {
    let drawer = PngDrawer::default();
    let mut graph = Graph::new();
    let n1 = graph.add_node_named("A", Some("a"));
    let n2 = graph.add_node_named("B", Some("b"));
    graph.add_edge(&n1, &n2, None, false);

    let nodes = drawer.add_nodes(&graph);
    assert_eq!(nodes.len(), 2);
}

#[test]
fn test_png_drawer_add_edges_method() {
    let drawer = PngDrawer::default();
    let mut graph = Graph::new();
    let n1 = graph.add_node_named("A", Some("a"));
    let n2 = graph.add_node_named("B", Some("b"));
    graph.add_edge(&n1, &n2, None, false);

    let edges = drawer.add_edges(&graph);
    assert_eq!(edges.len(), 1);
}

#[test]
fn test_png_drawer_update_styles_method() {
    let drawer = PngDrawer::default();
    let mut graph = Graph::new();
    let n1 = graph.add_node_named("A", Some("a"));
    let n2 = graph.add_node_named("B", Some("b"));
    graph.add_edge(&n1, &n2, None, false);

    let (first, last) = drawer.styled_node_ids(&graph);
    assert_eq!(first.as_deref(), Some("a"));
    assert_eq!(last.as_deref(), Some("b"));
}

#[test]
fn test_png_drawer_add_subgraph_method() {
    let drawer = PngDrawer::default();
    let nodes: Vec<Vec<String>> = vec![
        vec!["parent".into(), "child1".into()],
        vec!["parent".into(), "child2".into()],
        vec!["other".into()],
    ];
    let subgraphs = drawer.collect_subgraphs(&nodes, None);
    assert!(!subgraphs.is_empty());
    assert!(subgraphs[0].0.starts_with("cluster_"));
}

#[test]
fn test_labels_dict_can_be_empty() {
    let labels = LabelsDict::default();
    assert!(labels.nodes.is_empty());
    assert!(labels.edges.is_empty());
}

#[test]
fn test_labels_dict_nodes_only() {
    let labels = LabelsDict {
        nodes: HashMap::from([
            ("node1".into(), "Label1".into()),
            ("node2".into(), "Label2".into()),
        ]),
        edges: HashMap::new(),
    };
    assert_eq!(labels.nodes.len(), 2);
    assert!(labels.edges.is_empty());
}

#[test]
fn test_labels_dict_edges_only() {
    let labels = LabelsDict {
        nodes: HashMap::new(),
        edges: HashMap::from([
            ("edge1".into(), "Label1".into()),
            ("edge2".into(), "Label2".into()),
        ]),
    };
    assert!(labels.nodes.is_empty());
    assert_eq!(labels.edges.len(), 2);
}

#[test]
fn test_png_drawer_multiple_custom_labels() {
    let custom_labels = LabelsDict {
        nodes: HashMap::from([
            ("n1".into(), "Node One".into()),
            ("n2".into(), "Node Two".into()),
            ("n3".into(), "Node Three".into()),
        ]),
        edges: HashMap::from([
            ("e1".into(), "Edge One".into()),
            ("e2".into(), "Edge Two".into()),
        ]),
    };
    let drawer = PngDrawer::builder()
        .maybe_fontname(None.map(|s: &str| s.to_string()))
        .maybe_labels(Some(custom_labels))
        .build();

    assert_eq!(drawer.get_node_label("n1"), "<<B>Node One</B>>");
    assert_eq!(drawer.get_node_label("n2"), "<<B>Node Two</B>>");
    assert_eq!(drawer.get_edge_label("e1"), "<<U>Edge One</U>>");
}

#[test]
fn test_graph_draw_png_returns_error_when_no_path() {
    let mut graph = Graph::new();
    let node1 = graph.add_node_named("node1", Some("node1"));
    let node2 = graph.add_node_named("node2", Some("node2"));
    graph.add_edge(&node1, &node2, None, false);

    let result = graph.draw_png(None, None, None);
    assert!(result.is_err());
}

#[test]
fn test_png_drawer_font_names() {
    let fonts = ["arial", "helvetica", "courier", "times"];
    for font in &fonts {
        let drawer = PngDrawer::builder()
            .maybe_fontname(Some(font.to_string()))
            .maybe_labels(None)
            .build();
        assert_eq!(drawer.fontname, *font);
    }
}

#[test]
fn test_png_drawer_special_node_names() {
    let drawer = PngDrawer::default();

    let start_label = drawer.get_node_label("__start__");
    assert!(start_label.contains("<B>__start__</B>"));

    let end_label = drawer.get_node_label("__end__");
    assert!(end_label.contains("<B>__end__</B>"));
}

#[test]
fn test_png_drawer_html_formatting() {
    let drawer = PngDrawer::default();

    let node_label = drawer.get_node_label("test");
    assert!(node_label.starts_with("<<"));
    assert!(node_label.ends_with(">>"));
    assert!(node_label.contains("<B>"));

    let edge_label = drawer.get_edge_label("test");
    assert!(edge_label.starts_with("<<"));
    assert!(edge_label.ends_with(">>"));
    assert!(edge_label.contains("<U>"));
}

#[test]
fn test_labels_dict_type_definition() {
    let labels = LabelsDict::default();
    assert!(labels.nodes.is_empty());
    assert!(labels.edges.is_empty());
}

#[test]
fn test_png_drawer_labels_with_special_chars() {
    let custom_labels = LabelsDict {
        nodes: HashMap::from([
            ("n1".into(), "Node & Test".into()),
            ("n2".into(), "Node < > Test".into()),
        ]),
        edges: HashMap::from([("e1".into(), "Edge \"quoted\"".into())]),
    };
    let drawer = PngDrawer::builder()
        .maybe_fontname(None.map(|s: &str| s.to_string()))
        .maybe_labels(Some(custom_labels))
        .build();

    let label1 = drawer.get_node_label("n1");
    assert!(label1.contains("Node & Test"));

    let label2 = drawer.get_edge_label("e1");
    assert!(label2.contains("Edge"));
}

#[test]
fn test_graph_first_last_node_styling() {
    let mut graph = Graph::new();
    let first = graph.add_node_named("first", Some("first"));
    let middle = graph.add_node_named("middle", Some("middle"));
    let last = graph.add_node_named("last", Some("last"));

    graph.add_edge(&first, &middle, None, false);
    graph.add_edge(&middle, &last, None, false);

    let drawer = PngDrawer::default();
    let (first_id, last_id) = drawer.styled_node_ids(&graph);
    assert_eq!(first_id, Some("first".to_string()));
    assert_eq!(last_id, Some("last".to_string()));
}

#[test]
fn test_png_drawer_conditional_edges() {
    let drawer = PngDrawer::default();

    let solid_attrs = drawer.edge_attrs(None, false);
    assert_eq!(solid_attrs["style"], "solid");

    let dotted_attrs = drawer.edge_attrs(None, true);
    assert_eq!(dotted_attrs["style"], "dotted");
}

#[test]
fn test_graph_draw_png_complex_structure() {
    let mut graph = Graph::new();
    let nodes: Vec<_> = (0..5)
        .map(|i| {
            let name = format!("node{i}");
            graph.add_node_named(&name, Some(&name))
        })
        .collect();

    graph.add_edge(&nodes[0], &nodes[1], None, false);
    graph.add_edge(&nodes[0], &nodes[2], None, false);
    graph.add_edge(&nodes[1], &nodes[3], None, false);
    graph.add_edge(&nodes[2], &nodes[3], None, false);
    graph.add_edge(&nodes[3], &nodes[4], None, false);

    let result = graph.draw_png(None, None, None);
    assert!(matches!(result, Err(PngDrawError::MissingDependency(_))));
}

#[test]
fn test_png_drawer_with_subgraphs() {
    let mut graph = Graph::new();
    let parent = graph.add_node_named("parent", Some("parent"));
    let child1 = graph.add_node_named("parent:child1", Some("parent:child1"));
    let child2 = graph.add_node_named("parent:child2", Some("parent:child2"));

    graph.add_edge(&parent, &child1, None, false);
    graph.add_edge(&parent, &child2, None, false);

    let result = graph.draw_png(None, None, None);
    assert!(matches!(result, Err(PngDrawError::MissingDependency(_))));
}

#[test]
fn test_png_drawer_empty_graph() {
    let drawer = PngDrawer::default();
    let graph = Graph::new();

    let result = drawer.draw(&graph, None);
    assert!(result.is_err());
}

#[test]
fn test_labels_dict_partial_labels() {
    let labels = LabelsDict {
        nodes: HashMap::from([("node1".into(), "Custom1".into())]),
        edges: HashMap::new(),
    };
    let drawer = PngDrawer::builder()
        .maybe_fontname(None.map(|s: &str| s.to_string()))
        .maybe_labels(Some(labels))
        .build();

    assert!(drawer.get_node_label("node1").contains("Custom1"));
    assert!(drawer.get_node_label("node2").contains("node2"));
}

#[test]
fn test_png_drawer_fontname_used() {
    let fonts = ["arial", "helvetica", "times", "courier", "verdana"];
    for font in &fonts {
        let drawer = PngDrawer::builder()
            .maybe_fontname(Some(font.to_string()))
            .maybe_labels(None)
            .build();
        assert_eq!(drawer.fontname, *font);
    }
}

#[test]
fn test_graph_draw_png_with_conditional_edges() {
    let mut graph = Graph::new();
    let node1 = graph.add_node_named("node1", Some("node1"));
    let node2 = graph.add_node_named("node2", Some("node2"));
    let node3 = graph.add_node_named("node3", Some("node3"));

    graph.add_edge(&node1, &node2, None, false);
    graph.add_edge(&node1, &node3, None, true);

    let result = graph.draw_png(None, None, None);
    assert!(matches!(result, Err(PngDrawError::MissingDependency(_))));
}

#[test]
fn test_png_drawer_edge_with_data() {
    let mut graph = Graph::new();
    let node1 = graph.add_node_named("node1", Some("node1"));
    let node2 = graph.add_node_named("node2", Some("node2"));

    graph.add_edge(&node1, &node2, Some("edge_label".into()), false);

    let result = graph.draw_png(None, None, None);
    assert!(matches!(result, Err(PngDrawError::MissingDependency(_))));
}

#[test]
fn test_labels_dict_preserves_all_entries() {
    let node_labels: HashMap<String, String> = (0..10)
        .map(|i| (format!("node{i}"), format!("Label{i}")))
        .collect();
    let edge_labels: HashMap<String, String> = (0..10)
        .map(|i| (format!("edge{i}"), format!("EdgeLabel{i}")))
        .collect();

    let labels = LabelsDict {
        nodes: node_labels,
        edges: edge_labels,
    };

    assert_eq!(labels.nodes.len(), 10);
    assert_eq!(labels.edges.len(), 10);
}

#[test]
fn test_png_drawer_default_font() {
    let drawer = PngDrawer::default();
    assert_eq!(drawer.fontname, "arial");
}

#[test]
fn test_png_drawer_labels_immutable_default() {
    let drawer1 = PngDrawer::default();
    let drawer2 = PngDrawer::default();

    assert_eq!(drawer1.labels, drawer2.labels);
}

#[test]
fn test_graph_draw_png_returns_error_when_path_specified() {
    let mut graph = Graph::new();
    let node1 = graph.add_node_named("node1", Some("node1"));
    let node2 = graph.add_node_named("node2", Some("node2"));
    graph.add_edge(&node1, &node2, None, false);

    let result = graph.draw_png(
        Some(std::path::Path::new("/tmp/test_graph.png")),
        None,
        None,
    );
    assert!(matches!(result, Err(PngDrawError::MissingDependency(_))));
}

#[test]
fn test_png_drawer_handles_none_data() {
    let mut graph = Graph::new();
    let node1 = graph.add_node_named("node1", Some("node1"));
    let node2 = graph.add_node_named("node2", Some("node2"));
    graph.add_edge(&node1, &node2, None, false);

    let drawer = PngDrawer::default();
    let result = drawer.draw(&graph, None);
    assert!(result.is_err());
}

#[test]
fn test_graph_draw_png_overload_signatures() {
    let mut graph = Graph::new();
    let node1 = graph.add_node_named("node1", Some("node1"));
    let node2 = graph.add_node_named("node2", Some("node2"));
    graph.add_edge(&node1, &node2, None, false);

    let result1 = graph.draw_png(None, None, None);
    assert!(result1.is_err());

    let result2 = graph.draw_png(Some(std::path::Path::new("/tmp/test.png")), None, None);
    assert!(result2.is_err());
}

#[test]
fn test_png_drawer_labels_dict_typed_correctly() {
    let labels = LabelsDict {
        nodes: HashMap::from([("n1".into(), "Label1".into())]),
        edges: HashMap::from([("e1".into(), "Label2".into())]),
    };

    assert!(!labels.nodes.is_empty());
    assert!(!labels.edges.is_empty());
}

#[test]
fn test_graph_draw_png_with_metadata() {
    let mut graph = Graph::new();
    let meta1 = HashMap::from([("version".to_string(), serde_json::json!("1.0"))]);
    let meta2 = HashMap::from([("type".to_string(), serde_json::json!("processor"))]);

    let node1 = graph.add_node_named_with_metadata("node1", Some("node1"), meta1);
    let node2 = graph.add_node_named_with_metadata("node2", Some("node2"), meta2);
    graph.add_edge(&node1, &node2, None, false);

    let result = graph.draw_png(None, None, None);
    assert!(matches!(result, Err(PngDrawError::MissingDependency(_))));
}

#[test]
fn test_png_drawer_preserves_graph_structure() {
    let mut graph = Graph::new();
    let nodes: Vec<_> = (0..4)
        .map(|i| {
            let name = format!("node{i}");
            graph.add_node_named(&name, Some(&name))
        })
        .collect();

    for i in 0..nodes.len() - 1 {
        graph.add_edge(&nodes[i], &nodes[i + 1], None, false);
    }

    let drawer = PngDrawer::default();
    let node_list = drawer.add_nodes(&graph);
    let edge_list = drawer.add_edges(&graph);

    assert_eq!(node_list.len(), 4);
    assert_eq!(edge_list.len(), 3);
}

#[test]
fn test_png_drawer_edge_attrs_with_label() {
    let drawer = PngDrawer::default();
    let attrs = drawer.edge_attrs(Some("go"), false);
    assert_eq!(attrs["label"], "<<U>go</U>>");
}

#[test]
fn test_png_drawer_edge_attrs_without_label() {
    let drawer = PngDrawer::default();
    let attrs = drawer.edge_attrs(None, false);
    assert_eq!(attrs["label"], "");
}

#[test]
fn test_png_drawer_node_attrs_label() {
    let custom_labels = LabelsDict {
        nodes: HashMap::from([("x".into(), "CustomX".into())]),
        edges: HashMap::new(),
    };
    let drawer = PngDrawer::builder()
        .maybe_fontname(None.map(|s: &str| s.to_string()))
        .maybe_labels(Some(custom_labels))
        .build();
    let attrs = drawer.node_attrs("x");
    assert_eq!(attrs["label"], "<<B>CustomX</B>>");
}

#[test]
fn test_collect_subgraphs_no_shared_prefix() {
    let drawer = PngDrawer::default();
    let nodes: Vec<Vec<String>> = vec![vec!["a".into()], vec!["b".into()], vec!["c".into()]];
    let subgraphs = drawer.collect_subgraphs(&nodes, None);
    assert!(subgraphs.is_empty());
}

#[test]
fn test_collect_subgraphs_shared_prefix() {
    let drawer = PngDrawer::default();
    let nodes: Vec<Vec<String>> = vec![
        vec!["parent".into(), "child1".into()],
        vec!["parent".into(), "child2".into()],
    ];
    let subgraphs = drawer.collect_subgraphs(&nodes, None);
    assert_eq!(subgraphs.len(), 1);
    assert_eq!(subgraphs[0].0, "cluster_parent");
    assert_eq!(subgraphs[0].1.len(), 2);
}

#[test]
fn test_png_drawer_add_edges_conditional() {
    let drawer = PngDrawer::default();
    let mut graph = Graph::new();
    let n1 = graph.add_node_named("A", Some("a"));
    let n2 = graph.add_node_named("B", Some("b"));
    let n3 = graph.add_node_named("C", Some("c"));

    graph.add_edge(&n1, &n2, None, false);
    graph.add_edge(&n1, &n3, None, true);

    let edges = drawer.add_edges(&graph);
    assert_eq!(edges.len(), 2);

    let conditional_edge = edges.iter().find(|(_, t, _)| t == "c").expect("edge to c");
    assert_eq!(conditional_edge.2["style"], "dotted");

    let solid_edge = edges.iter().find(|(_, t, _)| t == "b").expect("edge to b");
    assert_eq!(solid_edge.2["style"], "solid");
}
