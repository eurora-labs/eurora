//! State graph module for LangGraph workflows.
//!
//! This module provides the StateGraph builder which allows you to create
//! graphs where nodes communicate by reading and writing to a shared state.

use std::collections::HashMap;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::Arc;

use futures::stream::{self, Stream};

use crate::constants::{END, START};
use crate::stream::{StreamChunk, StreamMode};

/// A node action that can be either sync or async.
pub type NodeAction<S> = Arc<dyn Fn(S) -> Pin<Box<dyn Future<Output = S> + Send>> + Send + Sync>;

/// A conditional edge function that returns the next node name.
pub type ConditionalEdge<S> =
    Arc<dyn Fn(&S) -> Pin<Box<dyn Future<Output = String> + Send>> + Send + Sync>;

/// Specification for a node in the graph.
pub struct NodeSpec<S> {
    /// The action to execute for this node.
    pub action: NodeAction<S>,
    /// Optional metadata for the node.
    pub metadata: Option<HashMap<String, String>>,
}

/// Specification for a conditional branch.
pub struct BranchSpec<S> {
    /// The condition function that determines the next node.
    pub condition: ConditionalEdge<S>,
    /// Mapping from condition result to node name.
    pub path_map: Option<HashMap<String, String>>,
}

/// A graph whose nodes communicate by reading and writing to a shared state.
///
/// The signature of each node is `State -> State` (or partial state update).
///
/// # Example
///
/// ```ignore
/// use agent_graph::graph::{StateGraph, START, END};
///
/// struct State {
///     text: String,
/// }
///
/// let mut graph = StateGraph::<State>::new();
///
/// graph.add_node("node_a", |mut state| async move {
///     state.text.push_str("a");
///     state
/// });
///
/// graph.add_node("node_b", |mut state| async move {
///     state.text.push_str("b");
///     state
/// });
///
/// graph.add_edge(START, "node_a");
/// graph.add_edge("node_a", "node_b");
/// graph.add_edge("node_b", END);
///
/// let compiled = graph.compile();
/// let result = compiled.invoke(State { text: String::new() }).await;
/// assert_eq!(result.text, "ab");
/// ```
pub struct StateGraph<S>
where
    S: Clone + Send + 'static,
{
    /// Nodes in the graph.
    nodes: HashMap<String, NodeSpec<S>>,
    /// Edges in the graph (from -> to).
    edges: Vec<(String, String)>,
    /// Conditional branches (from -> branch spec).
    branches: HashMap<String, BranchSpec<S>>,
    /// Whether the graph has been compiled.
    compiled: bool,
    /// Phantom data for the state type.
    _phantom: PhantomData<S>,
}

impl<S> Default for StateGraph<S>
where
    S: Clone + Send + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<S> StateGraph<S>
where
    S: Clone + Send + 'static,
{
    /// Create a new StateGraph.
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: Vec::new(),
            branches: HashMap::new(),
            compiled: false,
            _phantom: PhantomData,
        }
    }

    /// Add a new node to the StateGraph.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the node.
    /// * `action` - The async function to execute for this node.
    ///
    /// # Returns
    ///
    /// `&mut Self` for method chaining.
    ///
    /// # Panics
    ///
    /// Panics if a node with the same name already exists, or if the name
    /// is a reserved value (START or END).
    pub fn add_node<F, Fut>(&mut self, name: impl Into<String>, action: F) -> &mut Self
    where
        F: Fn(S) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = S> + Send + 'static,
    {
        let name = name.into();

        if name == START || name == END {
            panic!("Node name '{}' is reserved", name);
        }

        if self.nodes.contains_key(&name) {
            panic!("Node '{}' already exists", name);
        }

        let action: NodeAction<S> = Arc::new(move |state| {
            let fut = action(state);
            Box::pin(fut)
        });

        self.nodes.insert(
            name,
            NodeSpec {
                action,
                metadata: None,
            },
        );

        self
    }

    /// Add a directed edge from the start node to the end node.
    ///
    /// # Arguments
    ///
    /// * `start` - The name of the start node (or START constant).
    /// * `end` - The name of the end node (or END constant).
    ///
    /// # Returns
    ///
    /// `&mut Self` for method chaining.
    ///
    /// # Panics
    ///
    /// Panics if END is used as a start node, or if START is used as an end node.
    pub fn add_edge(&mut self, start: impl Into<String>, end: impl Into<String>) -> &mut Self {
        let start = start.into();
        let end = end.into();

        if start == END {
            panic!("END cannot be a start node");
        }

        if end == START {
            panic!("START cannot be an end node");
        }

        self.edges.push((start, end));
        self
    }

    /// Add a conditional edge from the source node to one of several destinations.
    ///
    /// # Arguments
    ///
    /// * `source` - The source node name.
    /// * `path` - An async function that returns the next node name based on the state.
    /// * `path_map` - Optional mapping from path function result to actual node names.
    ///
    /// # Returns
    ///
    /// `&mut Self` for method chaining.
    pub fn add_conditional_edges<F, Fut>(
        &mut self,
        source: impl Into<String>,
        path: F,
        path_map: Option<HashMap<String, String>>,
    ) -> &mut Self
    where
        F: Fn(&S) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = String> + Send + 'static,
    {
        let source = source.into();

        let condition: ConditionalEdge<S> = Arc::new(move |state| {
            let state_ref = state.clone();
            let fut = path(&state_ref);
            Box::pin(fut)
        });

        self.branches.insert(
            source,
            BranchSpec {
                condition,
                path_map,
            },
        );

        self
    }

    /// Set the entry point of the graph.
    ///
    /// Equivalent to `add_edge(START, key)`.
    pub fn set_entry_point(&mut self, key: impl Into<String>) -> &mut Self {
        self.add_edge(START, key)
    }

    /// Set a finish point of the graph.
    ///
    /// Equivalent to `add_edge(key, END)`.
    pub fn set_finish_point(&mut self, key: impl Into<String>) -> &mut Self {
        self.add_edge(key, END)
    }

    /// Set a conditional entry point for the graph.
    ///
    /// Equivalent to `add_conditional_edges(START, path, path_map)`.
    pub fn set_conditional_entry_point<F, Fut>(
        &mut self,
        path: F,
        path_map: Option<HashMap<String, String>>,
    ) -> &mut Self
    where
        F: Fn(&S) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = String> + Send + 'static,
    {
        self.add_conditional_edges(START, path, path_map)
    }

    /// Validate the graph structure.
    fn validate(&self) -> Result<(), String> {
        let has_start_edge = self.edges.iter().any(|(from, _)| from == START);
        let has_start_branch = self.branches.contains_key(START);

        if !has_start_edge && !has_start_branch {
            return Err(
                "Graph must have an entrypoint: add at least one edge from START to another node"
                    .to_string(),
            );
        }

        for (from, _) in &self.edges {
            if from != START && !self.nodes.contains_key(from) {
                return Err(format!("Edge source '{}' not found in nodes", from));
            }
        }

        for (_, to) in &self.edges {
            if to != END && !self.nodes.contains_key(to) {
                return Err(format!("Edge target '{}' not found in nodes", to));
            }
        }

        Ok(())
    }

    /// Compile the StateGraph into a CompiledGraph that can be invoked.
    ///
    /// # Returns
    ///
    /// A `CompiledGraph` that can be used to invoke or stream the workflow.
    ///
    /// # Panics
    ///
    /// Panics if the graph validation fails.
    pub fn compile(mut self) -> CompiledGraph<S> {
        self.validate().expect("Graph validation failed");
        self.compiled = true;

        CompiledGraph {
            nodes: self.nodes,
            edges: self.edges,
            branches: self.branches,
        }
    }
}

/// A compiled state graph that can be invoked or streamed.
pub struct CompiledGraph<S>
where
    S: Clone + Send + 'static,
{
    nodes: HashMap<String, NodeSpec<S>>,
    edges: Vec<(String, String)>,
    branches: HashMap<String, BranchSpec<S>>,
}

impl<S> CompiledGraph<S>
where
    S: Clone + Send + 'static,
{
    /// Find the next node(s) to execute after the given node.
    async fn get_next_nodes(&self, current: &str, state: &S) -> Vec<String> {
        if let Some(branch) = self.branches.get(current) {
            let result = (branch.condition)(state).await;
            let next = if let Some(ref path_map) = branch.path_map {
                path_map.get(&result).cloned().unwrap_or(result)
            } else {
                result
            };
            return vec![next];
        }

        self.edges
            .iter()
            .filter(|(from, _)| from == current)
            .map(|(_, to)| to.clone())
            .collect()
    }

    /// Invoke the graph with the given input state.
    ///
    /// # Arguments
    ///
    /// * `input` - The initial state.
    ///
    /// # Returns
    ///
    /// The final state after all nodes have been executed.
    pub async fn invoke(&self, input: S) -> S {
        let mut state = input;
        let mut current_nodes = self.get_next_nodes(START, &state).await;

        while !current_nodes.is_empty() {
            let current = current_nodes.remove(0);

            if current == END {
                continue;
            }

            if let Some(node_spec) = self.nodes.get(&current) {
                state = (node_spec.action)(state.clone()).await;
            }

            let next = self.get_next_nodes(&current, &state).await;
            current_nodes.extend(next);
        }

        state
    }

    /// Stream the graph execution.
    ///
    /// # Arguments
    ///
    /// * `input` - The initial state.
    /// * `mode` - The stream mode.
    ///
    /// # Returns
    ///
    /// A stream of `StreamChunk` values.
    pub fn stream(
        &self,
        input: S,
        mode: StreamMode,
    ) -> Pin<Box<dyn Stream<Item = StreamChunk<S>> + Send + '_>>
    where
        S: 'static,
    {
        let nodes = self.nodes.clone();
        let edges = self.edges.clone();
        let branches = self.branches.clone();

        Box::pin(stream::unfold(
            (input, vec![START.to_string()], nodes, edges, branches, mode),
            |(mut state, mut current_nodes, nodes, edges, branches, mode)| async move {
                loop {
                    if current_nodes.is_empty() {
                        return None;
                    }

                    let current = current_nodes.remove(0);

                    if current == END {
                        if mode == StreamMode::Values && current_nodes.is_empty() {
                            return Some((
                                StreamChunk::new(END, state.clone()),
                                (state, current_nodes, nodes, edges, branches, mode),
                            ));
                        }
                        continue;
                    }

                    if current == START {
                        if let Some(branch) = branches.get(START) {
                            let result = (branch.condition)(&state).await;
                            let next = if let Some(ref path_map) = branch.path_map {
                                path_map.get(&result).cloned().unwrap_or(result)
                            } else {
                                result
                            };
                            current_nodes.push(next);
                        } else {
                            let next: Vec<_> = edges
                                .iter()
                                .filter(|(from, _)| from == START)
                                .map(|(_, to)| to.clone())
                                .collect();
                            current_nodes.extend(next);
                        }
                        continue;
                    }

                    if let Some(node_spec) = nodes.get(&current) {
                        state = (node_spec.action)(state.clone()).await;

                        if let Some(branch) = branches.get(&current) {
                            let result = (branch.condition)(&state).await;
                            let next = if let Some(ref path_map) = branch.path_map {
                                path_map.get(&result).cloned().unwrap_or(result)
                            } else {
                                result
                            };
                            current_nodes.push(next);
                        } else {
                            let next: Vec<_> = edges
                                .iter()
                                .filter(|(from, _)| from == &current)
                                .map(|(_, to)| to.clone())
                                .collect();
                            current_nodes.extend(next);
                        }

                        match mode {
                            StreamMode::Updates | StreamMode::Values => {
                                return Some((
                                    StreamChunk::new(&current, state.clone()),
                                    (state, current_nodes, nodes, edges, branches, mode),
                                ));
                            }
                            _ => continue,
                        }
                    }
                }
            },
        ))
    }

    /// Get the graph structure for visualization.
    pub fn get_graph(&self) -> GraphStructure {
        GraphStructure {
            nodes: self.nodes.keys().cloned().collect(),
            edges: self.edges.clone(),
            branches: self.branches.keys().cloned().collect(),
        }
    }
}

/// Structure representing the graph for visualization.
#[derive(Debug, Clone)]
pub struct GraphStructure {
    /// Node names.
    pub nodes: Vec<String>,
    /// Edges as (from, to) pairs.
    pub edges: Vec<(String, String)>,
    /// Nodes with conditional branches.
    pub branches: Vec<String>,
}

impl<S> Clone for NodeSpec<S> {
    fn clone(&self) -> Self {
        Self {
            action: self.action.clone(),
            metadata: self.metadata.clone(),
        }
    }
}

impl<S> Clone for BranchSpec<S> {
    fn clone(&self) -> Self {
        Self {
            condition: self.condition.clone(),
            path_map: self.path_map.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, PartialEq)]
    struct TestState {
        value: i32,
    }

    #[tokio::test]
    async fn test_simple_graph() {
        let mut graph = StateGraph::<TestState>::new();

        graph.add_node("add_one", |mut state| async move {
            state.value += 1;
            state
        });

        graph.add_node("double", |mut state| async move {
            state.value *= 2;
            state
        });

        graph.add_edge(START, "add_one");
        graph.add_edge("add_one", "double");
        graph.add_edge("double", END);

        let compiled = graph.compile();
        let result = compiled.invoke(TestState { value: 5 }).await;

        assert_eq!(result.value, 12); // (5 + 1) * 2 = 12
    }

    #[tokio::test]
    async fn test_conditional_edges() {
        let mut graph = StateGraph::<TestState>::new();

        graph.add_node("check", |state| async move { state });

        graph.add_node("positive", |mut state| async move {
            state.value = 100;
            state
        });

        graph.add_node("negative", |mut state| async move {
            state.value = -100;
            state
        });

        graph.add_edge(START, "check");
        graph.add_conditional_edges(
            "check",
            |state: &TestState| {
                let value = state.value;
                async move {
                    if value >= 0 {
                        "positive".to_string()
                    } else {
                        "negative".to_string()
                    }
                }
            },
            None,
        );
        graph.add_edge("positive", END);
        graph.add_edge("negative", END);

        let compiled = graph.compile();

        let result = compiled.invoke(TestState { value: 5 }).await;
        assert_eq!(result.value, 100);

        let result = compiled.invoke(TestState { value: -5 }).await;
        assert_eq!(result.value, -100);
    }
}
