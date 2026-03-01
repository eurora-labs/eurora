use std::collections::HashMap;
use std::path::Path;

use bon::bon;

use super::graph::{Graph, LabelsDict};

#[derive(Debug, Clone)]
pub struct PngDrawer {
    pub fontname: String,
    pub labels: LabelsDict,
}

impl Default for PngDrawer {
    fn default() -> Self {
        Self {
            fontname: "arial".to_string(),
            labels: LabelsDict::default(),
        }
    }
}

#[bon]
impl PngDrawer {
    #[builder]
    pub fn new(
        #[builder(default = "arial".to_string())] fontname: String,
        #[builder(default)] labels: LabelsDict,
    ) -> Self {
        Self { fontname, labels }
    }

    pub fn get_node_label(&self, label: &str) -> String {
        let resolved = self
            .labels
            .nodes
            .get(label)
            .map(|s| s.as_str())
            .unwrap_or(label);
        format!("<<B>{resolved}</B>>")
    }

    pub fn get_edge_label(&self, label: &str) -> String {
        let resolved = self
            .labels
            .edges
            .get(label)
            .map(|s| s.as_str())
            .unwrap_or(label);
        format!("<<U>{resolved}</U>>")
    }

    pub fn node_attrs(&self, node: &str) -> HashMap<String, String> {
        let mut attrs = HashMap::new();
        attrs.insert("label".to_string(), self.get_node_label(node));
        attrs.insert("style".to_string(), "filled".to_string());
        attrs.insert("fillcolor".to_string(), "yellow".to_string());
        attrs.insert("fontsize".to_string(), "15".to_string());
        attrs.insert("fontname".to_string(), self.fontname.clone());
        attrs
    }

    pub fn edge_attrs(&self, label: Option<&str>, conditional: bool) -> HashMap<String, String> {
        let mut attrs = HashMap::new();
        let edge_label = match label {
            Some(l) => self.get_edge_label(l),
            None => String::new(),
        };
        attrs.insert("label".to_string(), edge_label);
        attrs.insert("fontsize".to_string(), "12".to_string());
        attrs.insert("fontname".to_string(), self.fontname.clone());
        attrs.insert(
            "style".to_string(),
            if conditional { "dotted" } else { "solid" }.to_string(),
        );
        attrs
    }

    pub fn add_nodes(&self, graph: &Graph) -> Vec<(String, HashMap<String, String>)> {
        let mut nodes: Vec<_> = graph.nodes.keys().cloned().collect();
        nodes.sort();
        nodes
            .into_iter()
            .map(|id| {
                let attrs = self.node_attrs(&id);
                (id, attrs)
            })
            .collect()
    }

    pub fn add_edges(&self, graph: &Graph) -> Vec<(String, String, HashMap<String, String>)> {
        graph
            .edges
            .iter()
            .map(|edge| {
                let label = edge.data.as_deref();
                let attrs = self.edge_attrs(label, edge.conditional);
                (edge.source.clone(), edge.target.clone(), attrs)
            })
            .collect()
    }

    #[allow(clippy::only_used_in_recursion)]
    pub fn collect_subgraphs(
        &self,
        nodes: &[Vec<String>],
        parent_prefix: Option<&[String]>,
    ) -> Vec<(String, Vec<String>)> {
        use itertools_substitute::group_by_key;

        let parent_prefix = parent_prefix.unwrap_or(&[]);

        let mut sorted: Vec<Vec<String>> = nodes.to_vec();
        sorted.sort();

        let mut result = Vec::new();

        for (prefix, group) in group_by_key(&sorted) {
            let mut current_prefix: Vec<String> = parent_prefix.to_vec();
            current_prefix.push(prefix.clone());

            let grouped_nodes: Vec<Vec<String>> = group;
            if grouped_nodes.len() > 1 {
                let cluster_name = format!("cluster_{}", current_prefix.join(":"));
                let member_ids: Vec<String> = grouped_nodes
                    .iter()
                    .map(|node| {
                        let mut parts = current_prefix.clone();
                        parts.extend(node.iter().cloned());
                        parts.join(":")
                    })
                    .collect();
                result.push((cluster_name, member_ids));

                let sub = self.collect_subgraphs(&grouped_nodes, Some(&current_prefix));
                result.extend(sub);
            }
        }

        result
    }

    pub fn styled_node_ids(&self, graph: &Graph) -> (Option<String>, Option<String>) {
        let first = graph.first_node().map(|n| n.id.clone());
        let last = graph.last_node().map(|n| n.id.clone());
        (first, last)
    }

    pub fn draw(
        &self,
        _graph: &Graph,
        _output_path: Option<&Path>,
    ) -> Result<Option<Vec<u8>>, PngDrawError> {
        Err(PngDrawError::MissingDependency(
            "PNG rendering requires a graphviz binding (not yet available). \
             Use draw_mermaid() for text-based graph rendering."
                .to_string(),
        ))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PngDrawError {
    #[error("{0}")]
    MissingDependency(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

mod itertools_substitute {
    pub fn group_by_key(sorted: &[Vec<String>]) -> Vec<(String, Vec<Vec<String>>)> {
        let mut result: Vec<(String, Vec<Vec<String>>)> = Vec::new();

        for item in sorted {
            if item.is_empty() {
                continue;
            }
            let key = item[0].clone();
            let tail: Vec<String> = item[1..].to_vec();

            if let Some(last) = result.last_mut()
                && last.0 == key
            {
                last.1.push(tail);
                continue;
            }
            result.push((key, vec![tail]));
        }

        result
    }
}
