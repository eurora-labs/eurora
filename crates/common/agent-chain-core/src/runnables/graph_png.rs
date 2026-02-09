//! Helper to draw a state graph into a PNG file.
//!
//! This module provides `PngDrawer`, mirroring
//! `langchain_core.runnables.graph_png.PngDrawer`.
//!
//! Actual PNG rendering requires the `graphviz` feature (not yet wired up);
//! the label-management API works without any optional dependency.

use std::collections::HashMap;
use std::path::Path;

use super::graph::{Graph, LabelsDict};

/// Helper to draw a state graph into a PNG file.
///
/// Mirrors Python's `PngDrawer` from `langchain_core.runnables.graph_png`.
#[derive(Debug, Clone)]
pub struct PngDrawer {
    /// Font name used for node and edge labels.
    pub fontname: String,
    /// Label overrides for nodes and edges.
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

impl PngDrawer {
    /// Create a new `PngDrawer`.
    ///
    /// * `fontname` – font for labels (defaults to `"arial"`).
    /// * `labels`   – optional label overrides.
    pub fn new(fontname: Option<&str>, labels: Option<LabelsDict>) -> Self {
        Self {
            fontname: fontname.unwrap_or("arial").to_string(),
            labels: labels.unwrap_or_default(),
        }
    }

    /// Return the display label for a node.
    ///
    /// If the node has a custom label in [`Self::labels`], that label is used;
    /// otherwise the original `label` string is kept. The result is wrapped in
    /// HTML bold tags: `<<B>…</B>>`.
    pub fn get_node_label(&self, label: &str) -> String {
        let resolved = self
            .labels
            .nodes
            .get(label)
            .map(|s| s.as_str())
            .unwrap_or(label);
        format!("<<B>{resolved}</B>>")
    }

    /// Return the display label for an edge.
    ///
    /// If the edge has a custom label in [`Self::labels`], that label is used;
    /// otherwise the original `label` string is kept. The result is wrapped in
    /// HTML underline tags: `<<U>…</U>>`.
    pub fn get_edge_label(&self, label: &str) -> String {
        let resolved = self
            .labels
            .edges
            .get(label)
            .map(|s| s.as_str())
            .unwrap_or(label);
        format!("<<U>{resolved}</U>>")
    }

    /// Build the graphviz attribute map for adding a single node.
    ///
    /// Returns a `HashMap` of attribute key-value pairs that mirror the
    /// Python `add_node` call (yellow fill, font size 15, solid+filled style).
    pub fn node_attrs(&self, node: &str) -> HashMap<String, String> {
        let mut attrs = HashMap::new();
        attrs.insert("label".to_string(), self.get_node_label(node));
        attrs.insert("style".to_string(), "filled".to_string());
        attrs.insert("fillcolor".to_string(), "yellow".to_string());
        attrs.insert("fontsize".to_string(), "15".to_string());
        attrs.insert("fontname".to_string(), self.fontname.clone());
        attrs
    }

    /// Build the graphviz attribute map for adding a single edge.
    ///
    /// Returns a `HashMap` of attribute key-value pairs that mirror the
    /// Python `add_edge` call.
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

    /// Collect attribute maps for all nodes in the graph.
    ///
    /// Mirrors `add_nodes` in the Python implementation.
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

    /// Collect attribute maps for all edges in the graph.
    ///
    /// Mirrors `add_edges` in the Python implementation.
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

    /// Identify subgraph groupings from colon-separated node IDs.
    ///
    /// Mirrors `add_subgraph` in the Python implementation. Returns a list of
    /// `(cluster_name, member_node_ids)` pairs for each subgraph detected.
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

    /// Determine style updates for first and last nodes.
    ///
    /// Mirrors `update_styles` in the Python implementation.
    /// Returns `(first_node_id, last_node_id)` so the caller can set
    /// `fillcolor` to `"lightblue"` and `"orange"` respectively.
    pub fn styled_node_ids(&self, graph: &Graph) -> (Option<String>, Option<String>) {
        let first = graph.first_node().map(|n| n.id.clone());
        let last = graph.last_node().map(|n| n.id.clone());
        (first, last)
    }

    /// Draw the graph to PNG bytes or save to a file.
    ///
    /// This is the main entry point, mirroring `PngDrawer.draw()`.
    ///
    /// Because actual rendering requires a graphviz C library binding that is
    /// not currently wired up as a dependency, this method always returns an
    /// error indicating the missing dependency — exactly like the Python
    /// implementation raises `ImportError` when `pygraphviz` is absent.
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

/// Errors that can occur when drawing a PNG.
#[derive(Debug, thiserror::Error)]
pub enum PngDrawError {
    /// The required graphviz library is not available.
    #[error("{0}")]
    MissingDependency(String),

    /// An I/O error occurred while writing the PNG file.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

// ---------------------------------------------------------------------------
// Internal helper – poor-man's itertools::group_by that works on sorted data.
// ---------------------------------------------------------------------------

mod itertools_substitute {
    /// Group a sorted slice of `Vec<String>` by popping the first element as key.
    ///
    /// Each input vec must have at least one element.
    /// Returns `(key, remaining_tails)` pairs.
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
