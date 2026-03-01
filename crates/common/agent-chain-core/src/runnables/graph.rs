use std::collections::HashMap;

use bon::bon;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LabelsDict {
    pub nodes: HashMap<String, String>,
    pub edges: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Edge {
    pub source: String,
    pub target: String,
    pub data: Option<String>,
    pub conditional: bool,
}

#[bon]
impl Edge {
    #[builder]
    pub fn new(
        source: impl Into<String>,
        target: impl Into<String>,
        data: Option<String>,
        #[builder(default)] conditional: bool,
    ) -> Self {
        Self {
            source: source.into(),
            target: target.into(),
            data,
            conditional,
        }
    }

    pub fn conditional(source: impl Into<String>, target: impl Into<String>) -> Self {
        Self::builder()
            .source(source)
            .target(target)
            .conditional(true)
            .build()
    }

    pub fn copy(&self, source: Option<&str>, target: Option<&str>) -> Self {
        Self {
            source: source.unwrap_or(&self.source).to_string(),
            target: target.unwrap_or(&self.target).to_string(),
            data: self.data.clone(),
            conditional: self.conditional,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeData {
    Schema { name: String },
    Runnable { name: String },
}

pub fn node_data_str(id: &str, data: Option<&NodeData>) -> String {
    let data = match data {
        Some(d) if is_uuid(id) => d,
        _ => return id.to_string(),
    };
    let data_str = match data {
        NodeData::Schema { name } => name.clone(),
        NodeData::Runnable { name } => name.clone(),
    };
    match data_str.strip_prefix("Runnable") {
        Some(stripped) => stripped.to_string(),
        None => data_str,
    }
}

pub fn node_data_json(node: &Node) -> Value {
    let mut json = serde_json::Map::new();

    match &node.data {
        None => {}
        Some(NodeData::Runnable { name }) => {
            json.insert("type".to_string(), Value::String("runnable".to_string()));
            let mut data_obj = serde_json::Map::new();
            data_obj.insert(
                "id".to_string(),
                Value::Array(
                    name.split("::")
                        .map(|s| Value::String(s.to_string()))
                        .collect(),
                ),
            );
            data_obj.insert(
                "name".to_string(),
                Value::String(node_data_str(&node.id, node.data.as_ref())),
            );
            json.insert("data".to_string(), Value::Object(data_obj));
        }
        Some(NodeData::Schema { .. }) => {
            json.insert("type".to_string(), Value::String("schema".to_string()));
            json.insert(
                "data".to_string(),
                Value::String(node_data_str(&node.id, node.data.as_ref())),
            );
        }
    }

    if let Some(ref metadata) = node.metadata
        && let Ok(meta_val) = serde_json::to_value(metadata)
    {
        json.insert("metadata".to_string(), meta_val);
    }

    Value::Object(json)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Node {
    pub id: String,
    pub name: String,
    pub data: Option<NodeData>,
    pub metadata: Option<HashMap<String, Value>>,
}

#[bon]
impl Node {
    #[builder]
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        data: Option<NodeData>,
        metadata: Option<HashMap<String, Value>>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            data,
            metadata,
        }
    }

    pub fn copy(&self, id: Option<&str>, name: Option<&str>) -> Self {
        Self {
            id: id.unwrap_or(&self.id).to_string(),
            name: name.unwrap_or(&self.name).to_string(),
            data: self.data.clone(),
            metadata: self.metadata.clone(),
        }
    }
}

pub fn is_uuid(value: &str) -> bool {
    Uuid::parse_str(value).is_ok()
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Graph {
    pub nodes: HashMap<String, Node>,
    pub edges: Vec<Edge>,
}

impl Graph {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: Vec::new(),
        }
    }

    pub fn from_parts(nodes: HashMap<String, Node>, edges: Vec<Edge>) -> Self {
        Self { nodes, edges }
    }

    pub fn next_id(&self) -> String {
        Uuid::new_v4().to_string()
    }

    pub fn add_node(
        &mut self,
        data: Option<NodeData>,
        id: Option<&str>,
        metadata: Option<HashMap<String, Value>>,
    ) -> Node {
        let id = id.map(|s| s.to_string()).unwrap_or_else(|| self.next_id());
        let name = node_data_str(&id, data.as_ref());
        let node = Node {
            id: id.clone(),
            name,
            data,
            metadata,
        };
        self.nodes.insert(id, node.clone());
        node
    }

    pub fn add_node_named(&mut self, name: impl Into<String>, id: Option<&str>) -> Node {
        let name = name.into();
        let id = id.map(|s| s.to_string()).unwrap_or_else(|| self.next_id());
        let node = Node {
            id: id.clone(),
            name,
            data: None,
            metadata: None,
        };
        self.nodes.insert(id, node.clone());
        node
    }

    pub fn add_node_named_with_metadata(
        &mut self,
        name: impl Into<String>,
        id: Option<&str>,
        metadata: HashMap<String, Value>,
    ) -> Node {
        let name = name.into();
        let id = id.map(|s| s.to_string()).unwrap_or_else(|| self.next_id());
        let node = Node {
            id: id.clone(),
            name,
            data: None,
            metadata: Some(metadata),
        };
        self.nodes.insert(id, node.clone());
        node
    }

    pub fn remove_node(&mut self, node: &Node) {
        self.nodes.remove(&node.id);
        self.edges
            .retain(|e| e.source != node.id && e.target != node.id);
    }

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

    pub fn first_node(&self) -> Option<&Node> {
        first_node_impl(self, &[])
    }

    pub fn last_node(&self) -> Option<&Node> {
        last_node_impl(self, &[])
    }

    pub fn trim_first_node(&mut self) {
        let first = match self.first_node() {
            Some(n) => n.clone(),
            None => return,
        };

        let has_replacement = first_node_impl(self, &[&first.id]).is_some();
        let outgoing_count = self.edges.iter().filter(|e| e.source == first.id).count();

        if has_replacement && outgoing_count == 1 {
            self.remove_node(&first);
        }
    }

    pub fn trim_last_node(&mut self) {
        let last = match self.last_node() {
            Some(n) => n.clone(),
            None => return,
        };

        let has_replacement = last_node_impl(self, &[&last.id]).is_some();
        let incoming_count = self.edges.iter().filter(|e| e.target == last.id).count();

        if has_replacement && incoming_count == 1 {
            self.remove_node(&last);
        }
    }

    pub fn to_json(&self) -> Value {
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
                let mut obj = match node_data_json(node) {
                    Value::Object(m) => m,
                    _ => serde_json::Map::new(),
                };
                obj.insert("id".to_string(), stable_ids[node.id.as_str()].clone());
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

    pub fn reid(&self) -> Graph {
        use std::collections::BTreeMap;

        let mut name_to_ids: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for node in self.nodes.values() {
            name_to_ids
                .entry(node.name.clone())
                .or_default()
                .push(node.id.clone());
        }

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

    pub fn draw_png(
        &self,
        output_file_path: Option<&std::path::Path>,
        fontname: Option<&str>,
        labels: Option<LabelsDict>,
    ) -> Result<Option<Vec<u8>>, super::graph_png::PngDrawError> {
        let default_node_labels: std::collections::HashMap<String, String> = self
            .nodes
            .values()
            .map(|n| (n.id.clone(), n.name.clone()))
            .collect();

        let merged_labels = LabelsDict {
            nodes: {
                let mut merged = default_node_labels;
                if let Some(ref user_labels) = labels {
                    merged.extend(
                        user_labels
                            .nodes
                            .iter()
                            .map(|(k, v)| (k.clone(), v.clone())),
                    );
                }
                merged
            },
            edges: labels.as_ref().map(|l| l.edges.clone()).unwrap_or_default(),
        };

        let drawer = super::graph_png::PngDrawer::new(fontname, Some(merged_labels));
        drawer.draw(self, output_file_path)
    }

    pub fn extend(&mut self, graph: Graph, prefix: &str) -> (Option<Node>, Option<Node>) {
        let prefix = if graph.nodes.values().all(|n| is_uuid(&n.id)) {
            ""
        } else {
            prefix
        };

        let prefixed = |id: &str| -> String {
            if prefix.is_empty() {
                id.to_string()
            } else {
                format!("{}:{}", prefix, id)
            }
        };

        for (key, node) in &graph.nodes {
            let new_id = prefixed(key);
            self.nodes
                .insert(new_id.clone(), node.copy(Some(&new_id), None));
        }

        for edge in &graph.edges {
            self.edges
                .push(edge.copy(Some(&prefixed(&edge.source)), Some(&prefixed(&edge.target))));
        }

        let first = graph.first_node().map(|n| {
            let new_id = prefixed(&n.id);
            n.copy(Some(&new_id), None)
        });
        let last = graph.last_node().map(|n| {
            let new_id = prefixed(&n.id);
            n.copy(Some(&new_id), None)
        });

        (first, last)
    }

    pub fn draw_mermaid(&self, options: Option<MermaidOptions>) -> crate::error::Result<String> {
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

    pub async fn draw_mermaid_png(
        &self,
        options: Option<MermaidOptions>,
        output_file_path: Option<&std::path::Path>,
        draw_method: MermaidDrawMethod,
        background_color: Option<&str>,
        max_retries: usize,
        retry_delay_secs: f64,
        base_url: Option<&str>,
    ) -> crate::error::Result<Vec<u8>> {
        let mermaid_syntax = self.draw_mermaid(options)?;
        super::graph_mermaid::draw_mermaid_png(
            &mermaid_syntax,
            output_file_path,
            draw_method,
            background_color,
            max_retries,
            retry_delay_secs,
            base_url,
        )
        .await
    }

    pub fn draw_ascii(&self) -> Result<String, String> {
        let vertices: std::collections::HashMap<String, String> = self
            .nodes
            .iter()
            .map(|(id, node)| (id.clone(), node.name.clone()))
            .collect();
        super::graph_ascii::draw_ascii(&vertices, &self.edges)
    }

    pub fn print_ascii(&self) {
        match self.draw_ascii() {
            Ok(ascii) => println!("{}", ascii),
            Err(err) => eprintln!("Error drawing ASCII: {}", err),
        }
    }
}

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

#[derive(Debug, Clone)]
pub struct Branch {
    pub condition: String,
    pub ends: Option<HashMap<String, String>>,
}

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
    pub const ALL: [CurveStyle; 12] = [
        CurveStyle::Basis,
        CurveStyle::BumpX,
        CurveStyle::BumpY,
        CurveStyle::Cardinal,
        CurveStyle::CatmullRom,
        CurveStyle::Linear,
        CurveStyle::MonotoneX,
        CurveStyle::MonotoneY,
        CurveStyle::Natural,
        CurveStyle::Step,
        CurveStyle::StepAfter,
        CurveStyle::StepBefore,
    ];

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MermaidDrawMethod {
    Pyppeteer,
    Api,
}

impl MermaidDrawMethod {
    pub fn value(&self) -> &'static str {
        match self {
            MermaidDrawMethod::Pyppeteer => "pyppeteer",
            MermaidDrawMethod::Api => "api",
        }
    }
}

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
