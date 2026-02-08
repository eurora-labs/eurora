//! Graph data structure for representing runnable compositions.
//!
//! This module provides the core `Graph`, `Node`, and `Edge` types,
//! mirroring `langchain_core.runnables.graph`.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// Edge in a graph.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Edge {
    /// The source node id.
    pub source: String,
    /// The target node id.
    pub target: String,
    /// Optional data associated with the edge.
    pub data: Option<String>,
    /// Whether the edge is conditional.
    pub conditional: bool,
}

impl Edge {
    /// Create a new edge.
    pub fn new(source: impl Into<String>, target: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            target: target.into(),
            data: None,
            conditional: false,
        }
    }

    /// Create a new conditional edge.
    pub fn conditional(source: impl Into<String>, target: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            target: target.into(),
            data: None,
            conditional: true,
        }
    }

    /// Return a copy of the edge with optional new source and target.
    pub fn copy(&self, source: Option<&str>, target: Option<&str>) -> Self {
        Self {
            source: source.unwrap_or(&self.source).to_string(),
            target: target.unwrap_or(&self.target).to_string(),
            data: self.data.clone(),
            conditional: self.conditional,
        }
    }
}

/// Node in a graph.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Node {
    /// The unique identifier of the node.
    pub id: String,
    /// The name of the node.
    pub name: String,
    /// Optional metadata for the node.
    pub metadata: Option<HashMap<String, Value>>,
}

impl Node {
    /// Create a new node.
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        let id = id.into();
        let name = name.into();
        Self {
            id,
            name,
            metadata: None,
        }
    }

    /// Create a new node with metadata.
    pub fn with_metadata(mut self, metadata: HashMap<String, Value>) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Return a copy of the node with optional new id and name.
    pub fn copy(&self, id: Option<&str>, name: Option<&str>) -> Self {
        Self {
            id: id.unwrap_or(&self.id).to_string(),
            name: name.unwrap_or(&self.name).to_string(),
            metadata: self.metadata.clone(),
        }
    }
}

/// Check if a string is a valid UUID.
pub fn is_uuid(value: &str) -> bool {
    Uuid::parse_str(value).is_ok()
}

/// Graph of nodes and edges.
///
/// This mirrors Python's `Graph` dataclass from `langchain_core.runnables.graph`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Graph {
    /// Dictionary of nodes in the graph, keyed by node id.
    pub nodes: HashMap<String, Node>,
    /// List of edges in the graph.
    pub edges: Vec<Edge>,
}

impl Graph {
    /// Create a new empty graph.
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: Vec::new(),
        }
    }

    /// Create a graph from existing nodes and edges.
    pub fn from_parts(nodes: HashMap<String, Node>, edges: Vec<Edge>) -> Self {
        Self { nodes, edges }
    }

    /// Return a new unique node identifier.
    pub fn next_id(&self) -> String {
        Uuid::new_v4().to_string()
    }

    /// Add a node to the graph and return a reference to it.
    ///
    /// If `id` is not provided, a new UUID is generated.
    pub fn add_node(&mut self, name: impl Into<String>, id: Option<&str>) -> Node {
        let name = name.into();
        let id = id.map(|s| s.to_string()).unwrap_or_else(|| self.next_id());
        let node = Node::new(&id, &name);
        self.nodes.insert(id, node.clone());
        node
    }

    /// Add a node with metadata.
    pub fn add_node_with_metadata(
        &mut self,
        name: impl Into<String>,
        id: Option<&str>,
        metadata: HashMap<String, Value>,
    ) -> Node {
        let name = name.into();
        let id = id.map(|s| s.to_string()).unwrap_or_else(|| self.next_id());
        let node = Node::new(&id, &name).with_metadata(metadata);
        self.nodes.insert(id, node.clone());
        node
    }

    /// Remove a node and all edges connected to it.
    pub fn remove_node(&mut self, node: &Node) {
        self.nodes.remove(&node.id);
        self.edges
            .retain(|e| e.source != node.id && e.target != node.id);
    }

    /// Add an edge to the graph.
    pub fn add_edge(
        &mut self,
        source: &Node,
        target: &Node,
        data: Option<String>,
        conditional: bool,
    ) -> Edge {
        let edge = Edge {
            source: source.id.clone(),
            target: target.id.clone(),
            data,
            conditional,
        };
        self.edges.push(edge.clone());
        edge
    }

    /// Find the single node that is not a target of any edge.
    pub fn first_node(&self) -> Option<&Node> {
        first_node_impl(self, &[])
    }

    /// Find the single node that is not a source of any edge.
    pub fn last_node(&self) -> Option<&Node> {
        last_node_impl(self, &[])
    }

    /// Remove the first node if it has a single outgoing edge and
    /// removing it would still leave a valid first node.
    pub fn trim_first_node(&mut self) {
        let first = match self.first_node() {
            Some(n) => n.clone(),
            None => return,
        };

        // Check that removing this node would leave a valid first node
        let has_replacement = first_node_impl(self, &[&first.id]).is_some();
        let outgoing_count = self.edges.iter().filter(|e| e.source == first.id).count();

        if has_replacement && outgoing_count == 1 {
            self.remove_node(&first);
        }
    }

    /// Remove the last node if it has a single incoming edge and
    /// removing it would still leave a valid last node.
    pub fn trim_last_node(&mut self) {
        let last = match self.last_node() {
            Some(n) => n.clone(),
            None => return,
        };

        // Check that removing this node would leave a valid last node
        let has_replacement = last_node_impl(self, &[&last.id]).is_some();
        let incoming_count = self.edges.iter().filter(|e| e.target == last.id).count();

        if has_replacement && incoming_count == 1 {
            self.remove_node(&last);
        }
    }

    /// Convert the graph to a JSON-serializable format.
    pub fn to_json(&self) -> Value {
        // Create stable integer IDs for UUID-based nodes
        let stable_ids: HashMap<&str, Value> = self
            .nodes
            .values()
            .enumerate()
            .map(|(i, node)| {
                let stable_id = if is_uuid(&node.id) {
                    Value::Number(serde_json::Number::from(i))
                } else {
                    Value::String(node.id.clone())
                };
                (node.id.as_str(), stable_id)
            })
            .collect();

        let nodes: Vec<Value> = self
            .nodes
            .values()
            .map(|node| {
                let mut obj = serde_json::Map::new();
                obj.insert("id".to_string(), stable_ids[node.id.as_str()].clone());
                obj.insert("name".to_string(), Value::String(node.name.clone()));
                if let Some(ref metadata) = node.metadata {
                    obj.insert(
                        "metadata".to_string(),
                        serde_json::to_value(metadata).unwrap_or(Value::Null),
                    );
                }
                Value::Object(obj)
            })
            .collect();

        let edges: Vec<Value> = self
            .edges
            .iter()
            .map(|edge| {
                let mut obj = serde_json::Map::new();
                obj.insert(
                    "source".to_string(),
                    stable_ids[edge.source.as_str()].clone(),
                );
                obj.insert(
                    "target".to_string(),
                    stable_ids[edge.target.as_str()].clone(),
                );
                if let Some(ref data) = edge.data {
                    obj.insert("data".to_string(), Value::String(data.clone()));
                }
                if edge.conditional {
                    obj.insert("conditional".to_string(), Value::Bool(true));
                }
                Value::Object(obj)
            })
            .collect();

        serde_json::json!({
            "nodes": nodes,
            "edges": edges,
        })
    }

    /// Re-identify all nodes using their readable names where possible.
    pub fn reid(&self) -> Graph {
        use std::collections::BTreeMap;

        // Group node names to detect duplicates
        let mut name_to_ids: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for node in self.nodes.values() {
            name_to_ids
                .entry(node.name.clone())
                .or_default()
                .push(node.id.clone());
        }

        // Create unique labels
        let mut unique_labels: HashMap<String, String> = HashMap::new();
        for (name, ids) in &name_to_ids {
            if ids.len() == 1 {
                unique_labels.insert(ids[0].clone(), name.clone());
            } else {
                for (i, id) in ids.iter().enumerate() {
                    unique_labels.insert(id.clone(), format!("{}_{}", name, i + 1));
                }
            }
        }

        let get_id = |node_id: &str| -> String {
            let label = &unique_labels[node_id];
            if is_uuid(node_id) {
                label.clone()
            } else {
                node_id.to_string()
            }
        };

        let new_nodes: HashMap<String, Node> = self
            .nodes
            .iter()
            .map(|(id, node)| {
                let new_id = get_id(id);
                (new_id.clone(), node.copy(Some(&new_id), None))
            })
            .collect();

        let new_edges: Vec<Edge> = self
            .edges
            .iter()
            .map(|edge| edge.copy(Some(&get_id(&edge.source)), Some(&get_id(&edge.target))))
            .collect();

        Graph {
            nodes: new_nodes,
            edges: new_edges,
        }
    }

    /// Draw the graph as a Mermaid syntax string.
    pub fn draw_mermaid(&self, options: Option<MermaidOptions>) -> String {
        let opts = options.unwrap_or_default();
        let graph = self.reid();
        let first_node = graph.first_node().map(|n| n.id.clone());
        let last_node = graph.last_node().map(|n| n.id.clone());

        super::graph_mermaid::draw_mermaid(
            &graph.nodes,
            &graph.edges,
            first_node.as_deref(),
            last_node.as_deref(),
            opts.with_styles,
            &opts.curve_style,
            &opts.node_styles.unwrap_or_default(),
            opts.wrap_label_n_words,
            opts.frontmatter_config.as_ref(),
        )
    }
}

/// Options for Mermaid rendering.
pub struct MermaidOptions {
    pub with_styles: bool,
    pub curve_style: CurveStyle,
    pub node_styles: Option<NodeStyles>,
    pub wrap_label_n_words: usize,
    pub frontmatter_config: Option<HashMap<String, Value>>,
}

impl Default for MermaidOptions {
    fn default() -> Self {
        Self {
            with_styles: true,
            curve_style: CurveStyle::Linear,
            node_styles: None,
            wrap_label_n_words: 9,
            frontmatter_config: None,
        }
    }
}

/// Enum for different curve styles supported by Mermaid.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CurveStyle {
    Basis,
    BumpX,
    BumpY,
    Cardinal,
    CatmullRom,
    Linear,
    MonotoneX,
    MonotoneY,
    Natural,
    Step,
    StepAfter,
    StepBefore,
}

impl CurveStyle {
    /// Get the Mermaid value string for this curve style.
    pub fn value(&self) -> &'static str {
        match self {
            CurveStyle::Basis => "basis",
            CurveStyle::BumpX => "bumpX",
            CurveStyle::BumpY => "bumpY",
            CurveStyle::Cardinal => "cardinal",
            CurveStyle::CatmullRom => "catmullRom",
            CurveStyle::Linear => "linear",
            CurveStyle::MonotoneX => "monotoneX",
            CurveStyle::MonotoneY => "monotoneY",
            CurveStyle::Natural => "natural",
            CurveStyle::Step => "step",
            CurveStyle::StepAfter => "stepAfter",
            CurveStyle::StepBefore => "stepBefore",
        }
    }
}

/// Hexadecimal color codes for different node types.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeStyles {
    pub default: String,
    pub first: String,
    pub last: String,
}

impl Default for NodeStyles {
    fn default() -> Self {
        Self {
            default: "fill:#f2f0ff,line-height:1.2".to_string(),
            first: "fill-opacity:0".to_string(),
            last: "fill:#bfb6fc".to_string(),
        }
    }
}

/// Enum for different draw methods supported by Mermaid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MermaidDrawMethod {
    Pyppeteer,
    Api,
}

// ============================================================================
// Internal helper functions
// ============================================================================

fn first_node_impl<'a>(graph: &'a Graph, exclude: &[&str]) -> Option<&'a Node> {
    let targets: std::collections::HashSet<&str> = graph
        .edges
        .iter()
        .filter(|e| !exclude.contains(&e.source.as_str()))
        .map(|e| e.target.as_str())
        .collect();

    let found: Vec<&Node> = graph
        .nodes
        .values()
        .filter(|n| !exclude.contains(&n.id.as_str()) && !targets.contains(n.id.as_str()))
        .collect();

    if found.len() == 1 {
        Some(found[0])
    } else {
        None
    }
}

fn last_node_impl<'a>(graph: &'a Graph, exclude: &[&str]) -> Option<&'a Node> {
    let sources: std::collections::HashSet<&str> = graph
        .edges
        .iter()
        .filter(|e| !exclude.contains(&e.target.as_str()))
        .map(|e| e.source.as_str())
        .collect();

    let found: Vec<&Node> = graph
        .nodes
        .values()
        .filter(|n| !exclude.contains(&n.id.as_str()) && !sources.contains(n.id.as_str()))
        .collect();

    if found.len() == 1 {
        Some(found[0])
    } else {
        None
    }
}
