use std::collections::HashMap;

use bon::bon;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::load::Serializable;
use crate::messages::{AIMessage, BaseMessage, FunctionMessage, HumanMessage};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentAction {
    pub tool: String,
    pub tool_input: ToolInput,
    pub log: String,
}

#[bon]
impl AgentAction {
    #[builder]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum ToolInput {
    Text(String),
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentActionMessageLog {
    pub tool: String,
    pub tool_input: ToolInput,
    pub log: String,
    pub message_log: Vec<BaseMessage>,
}

#[bon]
impl AgentActionMessageLog {
    #[builder]
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

    pub fn messages(&self) -> &[BaseMessage] {
        &self.message_log
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentStep {
    pub action: AgentAction,
    pub observation: Value,
}

#[bon]
impl AgentStep {
    #[builder]
    pub fn new(action: AgentAction, observation: Value) -> Self {
        Self {
            action,
            observation,
        }
    }

    pub fn messages(&self) -> Vec<BaseMessage> {
        convert_agent_observation_to_messages(&self.action, &self.observation)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentFinish {
    pub return_values: HashMap<String, Value>,
    pub log: String,
}

#[bon]
impl AgentFinish {
    #[builder]
    pub fn new(return_values: HashMap<String, Value>, log: impl Into<String>) -> Self {
        Self {
            return_values,
            log: log.into(),
        }
    }

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
        let action = AgentAction::builder()
            .tool("search")
            .tool_input("query")
            .log("Searching for query")
            .build();
        assert_eq!(action.tool, "search");
        assert_eq!(action.log, "Searching for query");
        match &action.tool_input {
            ToolInput::Text(s) => assert_eq!(s, "query"),
            _ => panic!("Expected text input"),
        }
    }

    #[test]
    fn test_agent_action_messages() {
        let action = AgentAction::builder()
            .tool("search")
            .tool_input("query")
            .log("I should search")
            .build();
        let messages = action.messages();
        assert_eq!(messages.len(), 1);
    }

    #[test]
    fn test_agent_action_dict_input() {
        let mut input = HashMap::new();
        input.insert("key".to_string(), Value::String("value".to_string()));
        let action = AgentAction::builder()
            .tool("tool")
            .tool_input(ToolInput::Dict(input))
            .log("log")
            .build();
        match &action.tool_input {
            ToolInput::Dict(d) => assert_eq!(d.get("key").unwrap(), "value"),
            _ => panic!("Expected dict input"),
        }
    }

    #[test]
    fn test_agent_action_message_log() {
        let msg = BaseMessage::AI(AIMessage::builder().content("I should search").build());
        let action = AgentActionMessageLog::builder()
            .tool("search")
            .tool_input("query")
            .log("I should search")
            .message_log(vec![msg.clone()])
            .build();
        assert_eq!(action.messages(), &[msg]);
    }

    #[test]
    fn test_agent_finish() {
        let mut return_values = HashMap::new();
        return_values.insert("output".to_string(), Value::String("42".to_string()));
        let finish = AgentFinish::builder()
            .return_values(return_values)
            .log("Final Answer: 42")
            .build();
        assert_eq!(finish.log, "Final Answer: 42");
        assert_eq!(finish.messages().len(), 1);
    }

    #[test]
    fn test_agent_step() {
        let action = AgentAction::builder()
            .tool("search")
            .tool_input("query")
            .log("Searching")
            .build();
        let step = AgentStep::builder()
            .action(action)
            .observation(Value::String("result".to_string()))
            .build();
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
        let action = AgentAction::builder()
            .tool("search")
            .tool_input("query")
            .log("log")
            .build();
        let json = serde_json::to_string(&action).unwrap();
        let deserialized: AgentAction = serde_json::from_str(&json).unwrap();
        assert_eq!(action, deserialized);
    }
}
