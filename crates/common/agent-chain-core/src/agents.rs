//! Schema definitions for representing agent actions, observations, and return values.
//!
//! Agents use language models to choose a sequence of actions to take.
//!
//! A basic agent works in the following manner:
//!
//! 1. Given a prompt an agent uses an LLM to request an action to take
//!    (e.g., a tool to run).
//! 2. The agent executes the action (e.g., runs the tool), and receives an observation.
//! 3. The agent returns the observation to the LLM, which can then be used to generate
//!    the next action.
//! 4. When the agent reaches a stopping condition, it returns a final return value.
//!
//! Mirrors `langchain_core.agents`.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::load::Serializable;
use crate::messages::{AIMessage, BaseMessage, FunctionMessage, HumanMessage};

/// Represents a request to execute an action by an agent.
///
/// The action consists of the name of the tool to execute and the input to pass
/// to the tool. The log is used to pass along extra information about the action.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentAction {
    /// The name of the Tool to execute.
    pub tool: String,
    /// The input to pass in to the Tool.
    pub tool_input: ToolInput,
    /// Additional information to log about the action.
    pub log: String,
}

impl AgentAction {
    /// Create a new AgentAction.
    pub fn new(
        tool: impl Into<String>,
        tool_input: impl Into<ToolInput>,
        log: impl Into<String>,
    ) -> Self {
        Self {
            tool: tool.into(),
            tool_input: tool_input.into(),
            log: log.into(),
        }
    }

    /// Return the messages that correspond to this action.
    pub fn messages(&self) -> Vec<BaseMessage> {
        vec![BaseMessage::AI(
            AIMessage::builder().content(&self.log).build(),
        )]
    }
}

impl Serializable for AgentAction {
    fn is_lc_serializable() -> bool {
        true
    }

    fn get_lc_namespace() -> Vec<String> {
        vec![
            "langchain".to_string(),
            "schema".to_string(),
            "agent".to_string(),
        ]
    }
}

/// Tool input that can be either a string or a dictionary.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum ToolInput {
    /// Simple string input.
    Text(String),
    /// Structured dictionary input.
    Dict(HashMap<String, Value>),
}

impl From<&str> for ToolInput {
    fn from(s: &str) -> Self {
        ToolInput::Text(s.to_string())
    }
}

impl From<String> for ToolInput {
    fn from(s: String) -> Self {
        ToolInput::Text(s)
    }
}

impl From<HashMap<String, Value>> for ToolInput {
    fn from(d: HashMap<String, Value>) -> Self {
        ToolInput::Dict(d)
    }
}

/// Representation of an action to be executed by an agent, with a message log.
///
/// This is similar to [`AgentAction`], but includes a message log consisting of
/// chat messages. This is useful when working with ChatModels, and is used to
/// reconstruct conversation history from the agent's perspective.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentActionMessageLog {
    /// The name of the Tool to execute.
    pub tool: String,
    /// The input to pass in to the Tool.
    pub tool_input: ToolInput,
    /// Additional information to log about the action.
    pub log: String,
    /// The message log from the LLM prediction before parsing out the action.
    pub message_log: Vec<BaseMessage>,
}

impl AgentActionMessageLog {
    /// Create a new AgentActionMessageLog.
    pub fn new(
        tool: impl Into<String>,
        tool_input: impl Into<ToolInput>,
        log: impl Into<String>,
        message_log: Vec<BaseMessage>,
    ) -> Self {
        Self {
            tool: tool.into(),
            tool_input: tool_input.into(),
            log: log.into(),
            message_log,
        }
    }

    /// Return the messages that correspond to this action.
    pub fn messages(&self) -> &[BaseMessage] {
        &self.message_log
    }
}

/// Result of running an [`AgentAction`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentStep {
    /// The AgentAction that was executed.
    pub action: AgentAction,
    /// The result of the AgentAction.
    pub observation: Value,
}

impl AgentStep {
    /// Create a new AgentStep.
    pub fn new(action: AgentAction, observation: Value) -> Self {
        Self {
            action,
            observation,
        }
    }

    /// Messages that correspond to this observation.
    pub fn messages(&self) -> Vec<BaseMessage> {
        convert_agent_observation_to_messages(&self.action, &self.observation)
    }
}

/// Final return value of an ActionAgent.
///
/// Agents return an AgentFinish when they have reached a stopping condition.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentFinish {
    /// Dictionary of return values.
    pub return_values: HashMap<String, Value>,
    /// Additional information to log about the return value.
    pub log: String,
}

impl AgentFinish {
    /// Create a new AgentFinish.
    pub fn new(return_values: HashMap<String, Value>, log: impl Into<String>) -> Self {
        Self {
            return_values,
            log: log.into(),
        }
    }

    /// Messages that correspond to this observation.
    pub fn messages(&self) -> Vec<BaseMessage> {
        vec![BaseMessage::AI(
            AIMessage::builder().content(&self.log).build(),
        )]
    }
}

impl Serializable for AgentFinish {
    fn is_lc_serializable() -> bool {
        true
    }

    fn get_lc_namespace() -> Vec<String> {
        vec![
            "langchain".to_string(),
            "schema".to_string(),
            "agent".to_string(),
        ]
    }
}

/// Convert an agent observation to messages.
fn convert_agent_observation_to_messages(
    _agent_action: &AgentAction,
    observation: &Value,
) -> Vec<BaseMessage> {
    let content = match observation {
        Value::String(s) => s.clone(),
        other => serde_json::to_string(other).unwrap_or_else(|_| other.to_string()),
    };
    vec![BaseMessage::Human(
        HumanMessage::builder().content(content).build(),
    )]
}

/// Convert an AgentActionMessageLog observation to messages.
pub fn convert_agent_action_message_log_observation_to_messages(
    agent_action: &AgentActionMessageLog,
    observation: &Value,
) -> Vec<BaseMessage> {
    let content = match observation {
        Value::String(s) => s.clone(),
        other => serde_json::to_string(other).unwrap_or_else(|_| other.to_string()),
    };
    vec![BaseMessage::Function(
        FunctionMessage::builder()
            .content(content)
            .name(&agent_action.tool)
            .build(),
    )]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_action_new() {
        let action = AgentAction::new("search", "query", "Searching for query");
        assert_eq!(action.tool, "search");
        assert_eq!(action.log, "Searching for query");
        match &action.tool_input {
            ToolInput::Text(s) => assert_eq!(s, "query"),
            _ => panic!("Expected text input"),
        }
    }

    #[test]
    fn test_agent_action_messages() {
        let action = AgentAction::new("search", "query", "I should search");
        let messages = action.messages();
        assert_eq!(messages.len(), 1);
    }

    #[test]
    fn test_agent_action_dict_input() {
        let mut input = HashMap::new();
        input.insert("key".to_string(), Value::String("value".to_string()));
        let action = AgentAction::new("tool", ToolInput::Dict(input), "log");
        match &action.tool_input {
            ToolInput::Dict(d) => assert_eq!(d.get("key").unwrap(), "value"),
            _ => panic!("Expected dict input"),
        }
    }

    #[test]
    fn test_agent_action_message_log() {
        let msg = BaseMessage::AI(AIMessage::builder().content("I should search").build());
        let action =
            AgentActionMessageLog::new("search", "query", "I should search", vec![msg.clone()]);
        assert_eq!(action.messages(), &[msg]);
    }

    #[test]
    fn test_agent_finish() {
        let mut return_values = HashMap::new();
        return_values.insert("output".to_string(), Value::String("42".to_string()));
        let finish = AgentFinish::new(return_values, "Final Answer: 42");
        assert_eq!(finish.log, "Final Answer: 42");
        assert_eq!(finish.messages().len(), 1);
    }

    #[test]
    fn test_agent_step() {
        let action = AgentAction::new("search", "query", "Searching");
        let step = AgentStep::new(action, Value::String("result".to_string()));
        let messages = step.messages();
        assert_eq!(messages.len(), 1);
    }

    #[test]
    fn test_agent_action_serializable() {
        assert!(AgentAction::is_lc_serializable());
        assert_eq!(
            AgentAction::get_lc_namespace(),
            vec!["langchain", "schema", "agent"]
        );
    }

    #[test]
    fn test_agent_finish_serializable() {
        assert!(AgentFinish::is_lc_serializable());
        assert_eq!(
            AgentFinish::get_lc_namespace(),
            vec!["langchain", "schema", "agent"]
        );
    }

    #[test]
    fn test_agent_action_serialization() {
        let action = AgentAction::new("search", "query", "log");
        let json = serde_json::to_string(&action).unwrap();
        let deserialized: AgentAction = serde_json::from_str(&json).unwrap();
        assert_eq!(action, deserialized);
    }
}
