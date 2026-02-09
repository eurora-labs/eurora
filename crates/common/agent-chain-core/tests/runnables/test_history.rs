//! Tests for RunnableWithMessageHistory and related types.
//!
//! Mirrors `langchain/libs/core/tests/unit_tests/runnables/test_history.py`

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use agent_chain_core::chat_history::{BaseChatMessageHistory, InMemoryChatMessageHistory};
use agent_chain_core::error::Result;
use agent_chain_core::messages::{AIMessage, BaseMessage, HumanMessage, SystemMessage};
use agent_chain_core::runnables::config::RunnableConfig;
use agent_chain_core::runnables::history::{
    GetSessionHistoryFn, HistoryInput, HistoryOutput, HistoryRunnable, RunnableWithMessageHistory,
};
use agent_chain_core::runnables::utils::ConfigurableFieldSpec;
use serde_json::Value;

// ===========================================================================
// Helpers
// ===========================================================================

/// Build a `RunnableConfig` with the given configurable key-value pairs.
fn config_with(pairs: &[(&str, &str)]) -> RunnableConfig {
    let mut cfg = RunnableConfig::default();
    for (k, v) in pairs {
        cfg.configurable
            .insert(k.to_string(), Value::String(v.to_string()));
    }
    cfg
}

/// Convenience: create a `GetSessionHistoryFn` backed by a shared store.
///
/// The store maps a single `session_id` string to a history instance.
fn make_session_factory(
    store: Arc<Mutex<HashMap<String, Arc<Mutex<InMemoryChatMessageHistory>>>>>,
) -> GetSessionHistoryFn {
    Arc::new(move |params: &HashMap<String, String>| {
        let session_id = params.get("session_id").cloned().unwrap_or_default();
        let mut guard = store.lock().unwrap();
        let entry = guard
            .entry(session_id)
            .or_insert_with(|| Arc::new(Mutex::new(InMemoryChatMessageHistory::new())));
        entry.clone()
    })
}

fn human(content: &str) -> BaseMessage {
    BaseMessage::Human(HumanMessage::builder().content(content).build())
}

fn ai(content: &str) -> BaseMessage {
    BaseMessage::AI(AIMessage::builder().content(content).build())
}

fn system(content: &str) -> BaseMessage {
    BaseMessage::System(SystemMessage::builder().content(content).build())
}

// ---------------------------------------------------------------------------
// Simple runnable: takes a list of messages, returns string output
// ---------------------------------------------------------------------------

/// A runnable that concatenates human-message contents, prefixed with "you said: ".
///
/// Mirrors the Python lambda used across many of the tests.
#[derive(Debug)]
struct ConcatHumanMessages;

impl HistoryRunnable for ConcatHumanMessages {
    fn invoke_history(
        &self,
        input: HistoryInput,
        _config: Option<&RunnableConfig>,
    ) -> Result<HistoryOutput> {
        let messages = match input {
            HistoryInput::Messages(msgs) => msgs,
            _ => {
                return Err(agent_chain_core::error::Error::Other(
                    "expected Messages input".into(),
                ));
            }
        };
        let human_contents: Vec<String> = messages
            .iter()
            .filter_map(|m| match m {
                BaseMessage::Human(h) => Some(h.content.as_text()),
                _ => None,
            })
            .collect();
        Ok(HistoryOutput::Text(format!(
            "you said: {}",
            human_contents.join("\n")
        )))
    }
}

// ---------------------------------------------------------------------------
// Dict-input runnable: takes {"messages": [...]} dict
// ---------------------------------------------------------------------------

/// A runnable whose input is a dict with a "messages" key.
#[derive(Debug)]
struct DictMessagesRunnable;

impl HistoryRunnable for DictMessagesRunnable {
    fn invoke_history(
        &self,
        input: HistoryInput,
        _config: Option<&RunnableConfig>,
    ) -> Result<HistoryOutput> {
        let map = match input {
            HistoryInput::Dict(m) => m,
            _ => {
                return Err(agent_chain_core::error::Error::Other(
                    "expected Dict input".into(),
                ));
            }
        };
        let msgs_val = map.get("messages").cloned().unwrap_or(Value::Array(vec![]));
        let messages: Vec<BaseMessage> = match msgs_val {
            Value::Array(arr) => arr
                .into_iter()
                .filter_map(|v| serde_json::from_value(v).ok())
                .collect(),
            _ => vec![],
        };
        let human_contents: Vec<String> = messages
            .iter()
            .filter_map(|m| match m {
                BaseMessage::Human(h) => Some(h.content.as_text()),
                _ => None,
            })
            .collect();
        Ok(HistoryOutput::Text(format!(
            "you said: {}",
            human_contents.join("\n")
        )))
    }
}

// ---------------------------------------------------------------------------
// Dict-input runnable with separate history key
// ---------------------------------------------------------------------------

/// A runnable whose input is `{"input": "...", "history": [...]}`.
#[derive(Debug)]
struct DictWithHistoryKeyRunnable;

impl HistoryRunnable for DictWithHistoryKeyRunnable {
    fn invoke_history(
        &self,
        input: HistoryInput,
        _config: Option<&RunnableConfig>,
    ) -> Result<HistoryOutput> {
        let map = match input {
            HistoryInput::Dict(m) => m,
            _ => {
                return Err(agent_chain_core::error::Error::Other(
                    "expected Dict input".into(),
                ));
            }
        };

        // Parse history messages
        let history_val = map.get("history").cloned().unwrap_or(Value::Array(vec![]));
        let history_msgs: Vec<BaseMessage> = match history_val {
            Value::Array(arr) => arr
                .into_iter()
                .filter_map(|v| serde_json::from_value(v).ok())
                .collect(),
            _ => vec![],
        };

        let input_str = map.get("input").and_then(|v| v.as_str()).unwrap_or("");

        let mut parts: Vec<String> = history_msgs
            .iter()
            .filter_map(|m| match m {
                BaseMessage::Human(h) => Some(h.content.as_text()),
                _ => None,
            })
            .collect();
        parts.push(input_str.to_string());

        Ok(HistoryOutput::Text(format!(
            "you said: {}",
            parts.join("\n")
        )))
    }
}

// ---------------------------------------------------------------------------
// Dict-input runnable that returns an AIMessage
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct DictReturnsAIMessage;

impl HistoryRunnable for DictReturnsAIMessage {
    fn invoke_history(
        &self,
        input: HistoryInput,
        _config: Option<&RunnableConfig>,
    ) -> Result<HistoryOutput> {
        let map = match input {
            HistoryInput::Dict(m) => m,
            _ => {
                return Err(agent_chain_core::error::Error::Other(
                    "expected Dict input".into(),
                ));
            }
        };

        let history_val = map.get("history").cloned().unwrap_or(Value::Array(vec![]));
        let history_msgs: Vec<BaseMessage> = match history_val {
            Value::Array(arr) => arr
                .into_iter()
                .filter_map(|v| serde_json::from_value(v).ok())
                .collect(),
            _ => vec![],
        };

        let input_str = map.get("input").and_then(|v| v.as_str()).unwrap_or("");

        let mut parts: Vec<String> = history_msgs
            .iter()
            .filter_map(|m| match m {
                BaseMessage::Human(h) => Some(h.content.as_text()),
                _ => None,
            })
            .collect();
        parts.push(input_str.to_string());

        let content = format!("you said: {}", parts.join("\n"));
        Ok(HistoryOutput::Message(ai(&content)))
    }
}

// ---------------------------------------------------------------------------
// Dict-input runnable that returns a list of AIMessages
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct DictReturnsMessageList;

impl HistoryRunnable for DictReturnsMessageList {
    fn invoke_history(
        &self,
        input: HistoryInput,
        _config: Option<&RunnableConfig>,
    ) -> Result<HistoryOutput> {
        let map = match input {
            HistoryInput::Dict(m) => m,
            _ => {
                return Err(agent_chain_core::error::Error::Other(
                    "expected Dict input".into(),
                ));
            }
        };

        let history_val = map.get("history").cloned().unwrap_or(Value::Array(vec![]));
        let history_msgs: Vec<BaseMessage> = match history_val {
            Value::Array(arr) => arr
                .into_iter()
                .filter_map(|v| serde_json::from_value(v).ok())
                .collect(),
            _ => vec![],
        };

        let input_str = map.get("input").and_then(|v| v.as_str()).unwrap_or("");

        let mut parts: Vec<String> = history_msgs
            .iter()
            .filter_map(|m| match m {
                BaseMessage::Human(h) => Some(h.content.as_text()),
                _ => None,
            })
            .collect();
        parts.push(input_str.to_string());

        let content = format!("you said: {}", parts.join("\n"));
        Ok(HistoryOutput::Messages(vec![ai(&content)]))
    }
}

// ---------------------------------------------------------------------------
// Dict-input runnable that returns a dict output
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct DictReturnsDictOutput;

impl HistoryRunnable for DictReturnsDictOutput {
    fn invoke_history(
        &self,
        input: HistoryInput,
        _config: Option<&RunnableConfig>,
    ) -> Result<HistoryOutput> {
        let map = match input {
            HistoryInput::Dict(m) => m,
            _ => {
                return Err(agent_chain_core::error::Error::Other(
                    "expected Dict input".into(),
                ));
            }
        };

        let history_val = map.get("history").cloned().unwrap_or(Value::Array(vec![]));
        let history_msgs: Vec<BaseMessage> = match history_val {
            Value::Array(arr) => arr
                .into_iter()
                .filter_map(|v| serde_json::from_value(v).ok())
                .collect(),
            _ => vec![],
        };

        let input_str = map.get("input").and_then(|v| v.as_str()).unwrap_or("");

        let mut parts: Vec<String> = history_msgs
            .iter()
            .filter_map(|m| match m {
                BaseMessage::Human(h) => Some(h.content.as_text()),
                _ => None,
            })
            .collect();
        parts.push(input_str.to_string());

        let content = format!("you said: {}", parts.join("\n"));
        let mut dict = HashMap::new();
        dict.insert(
            "output".to_string(),
            HistoryOutput::Messages(vec![ai(&content)]),
        );
        Ok(HistoryOutput::Dict(dict))
    }
}

// ---------------------------------------------------------------------------
// Length model: returns message count as string (mirrors LengthChatModel)
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct LengthRunnable;

impl HistoryRunnable for LengthRunnable {
    fn invoke_history(
        &self,
        input: HistoryInput,
        _config: Option<&RunnableConfig>,
    ) -> Result<HistoryOutput> {
        let messages = match input {
            HistoryInput::Messages(msgs) => msgs,
            _ => {
                return Err(agent_chain_core::error::Error::Other(
                    "expected Messages input".into(),
                ));
            }
        };
        let count = messages.len();
        Ok(HistoryOutput::Message(ai(&count.to_string())))
    }
}

// ---------------------------------------------------------------------------
// Dict-input runnable for custom config specs test
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct DictCustomConfigRunnable;

impl HistoryRunnable for DictCustomConfigRunnable {
    fn invoke_history(
        &self,
        input: HistoryInput,
        _config: Option<&RunnableConfig>,
    ) -> Result<HistoryOutput> {
        let map = match input {
            HistoryInput::Dict(m) => m,
            _ => {
                return Err(agent_chain_core::error::Error::Other(
                    "expected Dict input".into(),
                ));
            }
        };
        let msgs_val = map.get("messages").cloned().unwrap_or(Value::Array(vec![]));
        let messages: Vec<BaseMessage> = match msgs_val {
            Value::Array(arr) => arr
                .into_iter()
                .filter_map(|v| serde_json::from_value(v).ok())
                .collect(),
            _ => vec![],
        };
        let human_contents: Vec<String> = messages
            .iter()
            .filter_map(|m| match m {
                BaseMessage::Human(h) => Some(h.content.as_text()),
                _ => None,
            })
            .collect();
        Ok(HistoryOutput::Messages(vec![ai(&format!(
            "you said: {}",
            human_contents.join("\n")
        ))]))
    }
}

// ===========================================================================
// Tests
// ===========================================================================

/// Mirrors `test_interfaces` in Python.
#[test]
fn test_interfaces() {
    let mut history = InMemoryChatMessageHistory::new();
    history.add_messages(&[system("system"), human("human 1"), ai("ai")]);
    assert_eq!(
        history.to_string(),
        "System: system\nHuman: human 1\nAI: ai"
    );
}

/// Mirrors `test_input_messages` in Python.
///
/// The runnable takes a list of messages and returns a string.
/// History should accumulate across invocations for the same session.
#[test]
fn test_input_messages() {
    let store: Arc<Mutex<HashMap<String, Arc<Mutex<InMemoryChatMessageHistory>>>>> =
        Arc::new(Mutex::new(HashMap::new()));
    let factory = make_session_factory(store.clone());

    let with_history = RunnableWithMessageHistory::new(
        Box::new(ConcatHumanMessages),
        factory,
        None, // input_messages_key
        None, // output_messages_key
        None, // history_messages_key
        None, // history_factory_config (defaults to session_id)
    );

    let cfg = config_with(&[("session_id", "1")]);

    // First invocation
    let output = with_history
        .invoke(
            HistoryInput::Messages(vec![human("hello")]),
            Some(cfg.clone()),
        )
        .unwrap();
    assert_eq!(output_text(&output), "you said: hello");

    // Second invocation — history should include previous messages
    let output = with_history
        .invoke(
            HistoryInput::Messages(vec![human("good bye")]),
            Some(cfg.clone()),
        )
        .unwrap();
    assert_eq!(output_text(&output), "you said: hello\ngood bye");

    // Verify stored history
    let guard = store.lock().unwrap();
    let hist = guard.get("1").unwrap().lock().unwrap();
    let msgs = hist.messages();
    assert_eq!(msgs.len(), 4);
    assert_eq!(msgs[0].content(), "hello");
    assert_eq!(msgs[1].content(), "you said: hello");
    assert_eq!(msgs[2].content(), "good bye");
    assert_eq!(msgs[3].content(), "you said: hello\ngood bye");
}

/// Mirrors `test_input_dict` in Python.
#[test]
fn test_input_dict() {
    let store: Arc<Mutex<HashMap<String, Arc<Mutex<InMemoryChatMessageHistory>>>>> =
        Arc::new(Mutex::new(HashMap::new()));
    let factory = make_session_factory(store.clone());

    let with_history = RunnableWithMessageHistory::new(
        Box::new(DictMessagesRunnable),
        factory,
        Some("messages".into()),
        None,
        None,
        None,
    );

    let cfg = config_with(&[("session_id", "2")]);

    let input1 = HistoryInput::Dict(HashMap::from([(
        "messages".into(),
        serde_json::to_value(&[human("hello")]).unwrap(),
    )]));
    let output = with_history.invoke(input1, Some(cfg.clone())).unwrap();
    assert_eq!(output_text(&output), "you said: hello");

    let input2 = HistoryInput::Dict(HashMap::from([(
        "messages".into(),
        serde_json::to_value(&[human("good bye")]).unwrap(),
    )]));
    let output = with_history.invoke(input2, Some(cfg.clone())).unwrap();
    assert_eq!(output_text(&output), "you said: hello\ngood bye");
}

/// Mirrors `test_input_dict_with_history_key` in Python.
#[test]
fn test_input_dict_with_history_key() {
    let store: Arc<Mutex<HashMap<String, Arc<Mutex<InMemoryChatMessageHistory>>>>> =
        Arc::new(Mutex::new(HashMap::new()));
    let factory = make_session_factory(store.clone());

    let with_history = RunnableWithMessageHistory::new(
        Box::new(DictWithHistoryKeyRunnable),
        factory,
        Some("input".into()),
        None,
        Some("history".into()),
        None,
    );

    let cfg = config_with(&[("session_id", "3")]);

    let input1 = HistoryInput::Dict(HashMap::from([(
        "input".into(),
        Value::String("hello".into()),
    )]));
    let output = with_history.invoke(input1, Some(cfg.clone())).unwrap();
    assert_eq!(output_text(&output), "you said: hello");

    let input2 = HistoryInput::Dict(HashMap::from([(
        "input".into(),
        Value::String("good bye".into()),
    )]));
    let output = with_history.invoke(input2, Some(cfg.clone())).unwrap();
    assert_eq!(output_text(&output), "you said: hello\ngood bye");
}

/// Mirrors `test_output_message` in Python.
///
/// The runnable returns an `AIMessage` instead of a plain string.
#[test]
fn test_output_message() {
    let store: Arc<Mutex<HashMap<String, Arc<Mutex<InMemoryChatMessageHistory>>>>> =
        Arc::new(Mutex::new(HashMap::new()));
    let factory = make_session_factory(store.clone());

    let with_history = RunnableWithMessageHistory::new(
        Box::new(DictReturnsAIMessage),
        factory,
        Some("input".into()),
        None,
        Some("history".into()),
        None,
    );

    let cfg = config_with(&[("session_id", "4")]);

    let input1 = HistoryInput::Dict(HashMap::from([(
        "input".into(),
        Value::String("hello".into()),
    )]));
    let output = with_history.invoke(input1, Some(cfg.clone())).unwrap();
    match &output {
        HistoryOutput::Message(BaseMessage::AI(m)) => {
            assert_eq!(m.content, "you said: hello");
        }
        other => panic!("expected AIMessage, got {other:?}"),
    }

    let input2 = HistoryInput::Dict(HashMap::from([(
        "input".into(),
        Value::String("good bye".into()),
    )]));
    let output = with_history.invoke(input2, Some(cfg.clone())).unwrap();
    match &output {
        HistoryOutput::Message(BaseMessage::AI(m)) => {
            assert_eq!(m.content, "you said: hello\ngood bye");
        }
        other => panic!("expected AIMessage, got {other:?}"),
    }
}

/// Mirrors `test_input_messages_output_message` in Python (LengthChatModel).
///
/// First invocation sees 1 message → "1".
/// Second invocation sees 3 messages (prev human + prev AI + new human) → "3".
#[test]
fn test_input_messages_output_message() {
    let store: Arc<Mutex<HashMap<String, Arc<Mutex<InMemoryChatMessageHistory>>>>> =
        Arc::new(Mutex::new(HashMap::new()));
    let factory = make_session_factory(store.clone());

    let with_history =
        RunnableWithMessageHistory::new(Box::new(LengthRunnable), factory, None, None, None, None);

    let cfg = config_with(&[("session_id", "5")]);

    let output = with_history
        .invoke(HistoryInput::Messages(vec![human("hi")]), Some(cfg.clone()))
        .unwrap();
    assert_eq!(output_message_content(&output), "1");

    let output = with_history
        .invoke(HistoryInput::Messages(vec![human("hi")]), Some(cfg.clone()))
        .unwrap();
    assert_eq!(output_message_content(&output), "3");
}

/// Mirrors `test_output_messages` in Python.
///
/// The runnable returns a list of messages.
#[test]
fn test_output_messages() {
    let store: Arc<Mutex<HashMap<String, Arc<Mutex<InMemoryChatMessageHistory>>>>> =
        Arc::new(Mutex::new(HashMap::new()));
    let factory = make_session_factory(store.clone());

    let with_history = RunnableWithMessageHistory::new(
        Box::new(DictReturnsMessageList),
        factory,
        Some("input".into()),
        None,
        Some("history".into()),
        None,
    );

    let cfg = config_with(&[("session_id", "6")]);

    let input1 = HistoryInput::Dict(HashMap::from([(
        "input".into(),
        Value::String("hello".into()),
    )]));
    let output = with_history.invoke(input1, Some(cfg.clone())).unwrap();
    match &output {
        HistoryOutput::Messages(msgs) => {
            assert_eq!(msgs.len(), 1);
            assert_eq!(msgs[0].content(), "you said: hello");
        }
        other => panic!("expected Messages, got {other:?}"),
    }

    let input2 = HistoryInput::Dict(HashMap::from([(
        "input".into(),
        Value::String("good bye".into()),
    )]));
    let output = with_history.invoke(input2, Some(cfg.clone())).unwrap();
    match &output {
        HistoryOutput::Messages(msgs) => {
            assert_eq!(msgs.len(), 1);
            assert_eq!(msgs[0].content(), "you said: hello\ngood bye");
        }
        other => panic!("expected Messages, got {other:?}"),
    }
}

/// Mirrors `test_output_dict` in Python.
///
/// The runnable returns a dict with an "output" key containing messages.
#[test]
fn test_output_dict() {
    let store: Arc<Mutex<HashMap<String, Arc<Mutex<InMemoryChatMessageHistory>>>>> =
        Arc::new(Mutex::new(HashMap::new()));
    let factory = make_session_factory(store.clone());

    let with_history = RunnableWithMessageHistory::new(
        Box::new(DictReturnsDictOutput),
        factory,
        Some("input".into()),
        Some("output".into()),
        Some("history".into()),
        None,
    );

    let cfg = config_with(&[("session_id", "7")]);

    let input1 = HistoryInput::Dict(HashMap::from([(
        "input".into(),
        Value::String("hello".into()),
    )]));
    let output = with_history.invoke(input1, Some(cfg.clone())).unwrap();
    match &output {
        HistoryOutput::Dict(map) => {
            let inner = map.get("output").unwrap();
            match inner {
                HistoryOutput::Messages(msgs) => {
                    assert_eq!(msgs.len(), 1);
                    assert_eq!(msgs[0].content(), "you said: hello");
                }
                other => panic!("expected Messages in dict, got {other:?}"),
            }
        }
        other => panic!("expected Dict output, got {other:?}"),
    }

    let input2 = HistoryInput::Dict(HashMap::from([(
        "input".into(),
        Value::String("good bye".into()),
    )]));
    let output = with_history.invoke(input2, Some(cfg.clone())).unwrap();
    match &output {
        HistoryOutput::Dict(map) => {
            let inner = map.get("output").unwrap();
            match inner {
                HistoryOutput::Messages(msgs) => {
                    assert_eq!(msgs.len(), 1);
                    assert_eq!(msgs[0].content(), "you said: hello\ngood bye");
                }
                other => panic!("expected Messages in dict, got {other:?}"),
            }
        }
        other => panic!("expected Dict output, got {other:?}"),
    }
}

/// Mirrors `test_using_custom_config_specs` in Python.
///
/// Uses a session factory that takes `user_id` and `conversation_id`.
#[test]
fn test_using_custom_config_specs() {
    let store: Arc<Mutex<HashMap<(String, String), Arc<Mutex<InMemoryChatMessageHistory>>>>> =
        Arc::new(Mutex::new(HashMap::new()));

    let factory: GetSessionHistoryFn = {
        let store = store.clone();
        Arc::new(move |params: &HashMap<String, String>| {
            let user_id = params.get("user_id").cloned().unwrap_or_default();
            let conversation_id = params.get("conversation_id").cloned().unwrap_or_default();
            let key = (user_id, conversation_id);
            let mut guard = store.lock().unwrap();
            let entry = guard
                .entry(key)
                .or_insert_with(|| Arc::new(Mutex::new(InMemoryChatMessageHistory::new())));
            entry.clone()
        })
    };

    let with_history = RunnableWithMessageHistory::new(
        Box::new(DictCustomConfigRunnable),
        factory,
        Some("messages".into()),
        None,
        Some("history".into()),
        Some(vec![
            ConfigurableFieldSpec {
                id: "user_id".into(),
                annotation: "str".into(),
                name: Some("User ID".into()),
                description: Some("Unique identifier for the user.".into()),
                default: Some(Value::String(String::new())),
                is_shared: true,
                dependencies: None,
            },
            ConfigurableFieldSpec {
                id: "conversation_id".into(),
                annotation: "str".into(),
                name: Some("Conversation ID".into()),
                description: Some("Unique identifier for the conversation.".into()),
                default: None,
                is_shared: true,
                dependencies: None,
            },
        ]),
    );

    // user1, conversation 1: "hello"
    let cfg1 = config_with(&[("user_id", "user1"), ("conversation_id", "1")]);
    let input1 = HistoryInput::Dict(HashMap::from([(
        "messages".into(),
        serde_json::to_value(&[human("hello")]).unwrap(),
    )]));
    let result = with_history.invoke(input1, Some(cfg1.clone())).unwrap();
    match &result {
        HistoryOutput::Messages(msgs) => {
            assert_eq!(msgs.len(), 1);
            assert_eq!(msgs[0].content(), "you said: hello");
        }
        other => panic!("expected Messages, got {other:?}"),
    }

    // Verify store has the messages
    {
        let guard = store.lock().unwrap();
        let hist = guard
            .get(&("user1".into(), "1".into()))
            .unwrap()
            .lock()
            .unwrap();
        assert_eq!(hist.messages().len(), 2);
    }

    // user1, conversation 1: "goodbye"
    let input2 = HistoryInput::Dict(HashMap::from([(
        "messages".into(),
        serde_json::to_value(&[human("goodbye")]).unwrap(),
    )]));
    let result = with_history.invoke(input2, Some(cfg1.clone())).unwrap();
    match &result {
        HistoryOutput::Messages(msgs) => {
            assert_eq!(msgs.len(), 1);
            assert_eq!(msgs[0].content(), "you said: goodbye");
        }
        other => panic!("expected Messages, got {other:?}"),
    }

    {
        let guard = store.lock().unwrap();
        let hist = guard
            .get(&("user1".into(), "1".into()))
            .unwrap()
            .lock()
            .unwrap();
        assert_eq!(hist.messages().len(), 4);
    }

    // user2, conversation 1: "meow"
    let cfg2 = config_with(&[("user_id", "user2"), ("conversation_id", "1")]);
    let input3 = HistoryInput::Dict(HashMap::from([(
        "messages".into(),
        serde_json::to_value(&[human("meow")]).unwrap(),
    )]));
    let result = with_history.invoke(input3, Some(cfg2)).unwrap();
    match &result {
        HistoryOutput::Messages(msgs) => {
            assert_eq!(msgs.len(), 1);
            assert_eq!(msgs[0].content(), "you said: meow");
        }
        other => panic!("expected Messages, got {other:?}"),
    }

    // Verify store sizes
    {
        let guard = store.lock().unwrap();
        assert_eq!(guard.len(), 2);
        let u1 = guard
            .get(&("user1".into(), "1".into()))
            .unwrap()
            .lock()
            .unwrap();
        assert_eq!(u1.messages().len(), 4);
        let u2 = guard
            .get(&("user2".into(), "1".into()))
            .unwrap()
            .lock()
            .unwrap();
        assert_eq!(u2.messages().len(), 2);
    }
}

/// Mirrors `test_ignore_session_id` in Python.
///
/// A factory that takes no session_id — a single global history.
#[test]
fn test_ignore_session_id() {
    let history = Arc::new(Mutex::new(InMemoryChatMessageHistory::new()));
    let factory: GetSessionHistoryFn = {
        let history = history.clone();
        Arc::new(move |_params: &HashMap<String, String>| history.clone())
    };

    let with_history = RunnableWithMessageHistory::new(
        Box::new(ConcatHumanMessages),
        factory,
        None,
        None,
        None,
        None,
    );

    // Invoke without meaningful config
    let _ = with_history
        .invoke(HistoryInput::Messages(vec![human("hello")]), None)
        .unwrap();
    let _ = with_history
        .invoke(HistoryInput::Messages(vec![human("hello again")]), None)
        .unwrap();

    let hist = history.lock().unwrap();
    assert_eq!(hist.messages().len(), 4);
}

/// Mirrors `test_get_output_messages_no_value_error` in Python.
///
/// A valid string output should not cause any error when extracting messages.
#[test]
fn test_get_output_messages_no_value_error() {
    let store: Arc<Mutex<HashMap<String, Arc<Mutex<InMemoryChatMessageHistory>>>>> =
        Arc::new(Mutex::new(HashMap::new()));
    let factory = make_session_factory(store.clone());

    let with_history = RunnableWithMessageHistory::new(
        Box::new(ConcatHumanMessages),
        factory,
        None,
        None,
        None,
        None,
    );

    let output = HistoryOutput::Text("you said: hello".into());
    let result = with_history.get_output_messages(&output);
    assert!(result.is_ok());
    let msgs = result.unwrap();
    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0].content(), "you said: hello");
}

/// Mirrors `test_get_output_messages_with_value_error` in Python.
///
/// An output that is not a recognised type should produce an error.
#[test]
fn test_get_output_messages_with_value_error() {
    let store: Arc<Mutex<HashMap<String, Arc<Mutex<InMemoryChatMessageHistory>>>>> =
        Arc::new(Mutex::new(HashMap::new()));
    let factory = make_session_factory(store.clone());

    let with_history = RunnableWithMessageHistory::new(
        Box::new(ConcatHumanMessages),
        factory,
        None,
        None,
        None,
        None,
    );

    // A dict with unknown nested type would cause extraction errors
    let mut bad_dict = HashMap::new();
    bad_dict.insert(
        "output".to_string(),
        HistoryOutput::Dict(HashMap::new()), // nested dict is not a valid output message type
    );
    let output = HistoryOutput::Dict(bad_dict);

    let result = with_history.get_output_messages(&output);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("Expected str, BaseMessage, list[BaseMessage], or tuple[BaseMessage]")
    );
}

/// Test that get_input_messages handles string values (converted to HumanMessage).
#[test]
fn test_get_input_messages_from_string() {
    let store: Arc<Mutex<HashMap<String, Arc<Mutex<InMemoryChatMessageHistory>>>>> =
        Arc::new(Mutex::new(HashMap::new()));
    let factory = make_session_factory(store.clone());

    let with_history = RunnableWithMessageHistory::new(
        Box::new(ConcatHumanMessages),
        factory,
        Some("input".into()),
        None,
        None,
        None,
    );

    let input = HistoryInput::Dict(HashMap::from([(
        "input".into(),
        Value::String("hello".into()),
    )]));

    let msgs = with_history.get_input_messages(&input).unwrap();
    assert_eq!(msgs.len(), 1);
    assert!(matches!(&msgs[0], BaseMessage::Human(_)));
    assert_eq!(msgs[0].content(), "hello");
}

/// Test that get_input_messages handles message arrays.
#[test]
fn test_get_input_messages_from_message_list() {
    let store: Arc<Mutex<HashMap<String, Arc<Mutex<InMemoryChatMessageHistory>>>>> =
        Arc::new(Mutex::new(HashMap::new()));
    let factory = make_session_factory(store.clone());

    let with_history = RunnableWithMessageHistory::new(
        Box::new(ConcatHumanMessages),
        factory,
        None,
        None,
        None,
        None,
    );

    let input = HistoryInput::Messages(vec![human("hi"), human("bye")]);
    let msgs = with_history.get_input_messages(&input).unwrap();
    assert_eq!(msgs.len(), 2);
    assert_eq!(msgs[0].content(), "hi");
    assert_eq!(msgs[1].content(), "bye");
}

/// Test that config_specs returns the expected default (session_id).
#[test]
fn test_config_specs_default() {
    let store: Arc<Mutex<HashMap<String, Arc<Mutex<InMemoryChatMessageHistory>>>>> =
        Arc::new(Mutex::new(HashMap::new()));
    let factory = make_session_factory(store.clone());

    let with_history = RunnableWithMessageHistory::new(
        Box::new(ConcatHumanMessages),
        factory,
        None,
        None,
        None,
        None,
    );

    let specs = with_history.config_specs();
    assert_eq!(specs.len(), 1);
    assert_eq!(specs[0].id, "session_id");
    assert!(specs[0].is_shared);
}

/// Test that custom config_specs are preserved.
#[test]
fn test_config_specs_custom() {
    let store: Arc<Mutex<HashMap<String, Arc<Mutex<InMemoryChatMessageHistory>>>>> =
        Arc::new(Mutex::new(HashMap::new()));
    let factory = make_session_factory(store.clone());

    let with_history = RunnableWithMessageHistory::new(
        Box::new(ConcatHumanMessages),
        factory,
        None,
        None,
        None,
        Some(vec![
            ConfigurableFieldSpec {
                id: "user_id".into(),
                annotation: "str".into(),
                name: Some("User ID".into()),
                description: None,
                default: None,
                is_shared: true,
                dependencies: None,
            },
            ConfigurableFieldSpec {
                id: "thread_id".into(),
                annotation: "str".into(),
                name: Some("Thread ID".into()),
                description: None,
                default: None,
                is_shared: true,
                dependencies: None,
            },
        ]),
    );

    let specs = with_history.config_specs();
    assert_eq!(specs.len(), 2);
    assert_eq!(specs[0].id, "user_id");
    assert_eq!(specs[1].id, "thread_id");
}

/// Test that separate sessions accumulate independently.
#[test]
fn test_separate_sessions() {
    let store: Arc<Mutex<HashMap<String, Arc<Mutex<InMemoryChatMessageHistory>>>>> =
        Arc::new(Mutex::new(HashMap::new()));
    let factory = make_session_factory(store.clone());

    let with_history = RunnableWithMessageHistory::new(
        Box::new(ConcatHumanMessages),
        factory,
        None,
        None,
        None,
        None,
    );

    let cfg_a = config_with(&[("session_id", "a")]);
    let cfg_b = config_with(&[("session_id", "b")]);

    let _ = with_history
        .invoke(
            HistoryInput::Messages(vec![human("A1")]),
            Some(cfg_a.clone()),
        )
        .unwrap();
    let _ = with_history
        .invoke(
            HistoryInput::Messages(vec![human("B1")]),
            Some(cfg_b.clone()),
        )
        .unwrap();
    let _ = with_history
        .invoke(
            HistoryInput::Messages(vec![human("A2")]),
            Some(cfg_a.clone()),
        )
        .unwrap();

    let guard = store.lock().unwrap();
    let hist_a = guard.get("a").unwrap().lock().unwrap();
    let hist_b = guard.get("b").unwrap().lock().unwrap();

    // Session A: 4 messages (A1, AI response, A2, AI response)
    assert_eq!(hist_a.messages().len(), 4);
    // Session B: 2 messages (B1, AI response)
    assert_eq!(hist_b.messages().len(), 2);
}

/// Test that output messages from a single AIMessage are correctly extracted.
#[test]
fn test_get_output_messages_single_message() {
    let store: Arc<Mutex<HashMap<String, Arc<Mutex<InMemoryChatMessageHistory>>>>> =
        Arc::new(Mutex::new(HashMap::new()));
    let factory = make_session_factory(store.clone());

    let with_history = RunnableWithMessageHistory::new(
        Box::new(ConcatHumanMessages),
        factory,
        None,
        None,
        None,
        None,
    );

    let output = HistoryOutput::Message(ai("response"));
    let msgs = with_history.get_output_messages(&output).unwrap();
    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0].content(), "response");
}

/// Test that output messages from a list are correctly extracted.
#[test]
fn test_get_output_messages_list() {
    let store: Arc<Mutex<HashMap<String, Arc<Mutex<InMemoryChatMessageHistory>>>>> =
        Arc::new(Mutex::new(HashMap::new()));
    let factory = make_session_factory(store.clone());

    let with_history = RunnableWithMessageHistory::new(
        Box::new(ConcatHumanMessages),
        factory,
        None,
        None,
        None,
        None,
    );

    let output = HistoryOutput::Messages(vec![ai("msg1"), ai("msg2")]);
    let msgs = with_history.get_output_messages(&output).unwrap();
    assert_eq!(msgs.len(), 2);
    assert_eq!(msgs[0].content(), "msg1");
    assert_eq!(msgs[1].content(), "msg2");
}

// ===========================================================================
// Helpers for assertions
// ===========================================================================

fn output_text(output: &HistoryOutput) -> &str {
    match output {
        HistoryOutput::Text(s) => s.as_str(),
        other => panic!("expected Text output, got {other:?}"),
    }
}

fn output_message_content(output: &HistoryOutput) -> &str {
    match output {
        HistoryOutput::Message(m) => m.content(),
        other => panic!("expected Message output, got {other:?}"),
    }
}

// ===========================================================================
// Schema tests
// ===========================================================================

/// Mirrors `test_get_input_schema_input_dict`.
///
/// When `input_messages_key` and `history_messages_key` are both set,
/// the input schema should have a single required field for the input key.
#[test]
fn test_get_input_schema_input_dict() {
    let store: Arc<Mutex<HashMap<String, Arc<Mutex<InMemoryChatMessageHistory>>>>> =
        Arc::new(Mutex::new(HashMap::new()));
    let factory = make_session_factory(store);

    let with_history = RunnableWithMessageHistory::new(
        Box::new(DictReturnsDictOutput),
        factory,
        Some("input".into()),
        Some("output".into()),
        Some("history".into()),
        None,
    );

    let schema = with_history.get_input_schema();
    assert_eq!(schema["title"], "RunnableWithChatHistoryInput");
    assert_eq!(schema["type"], "object");
    // Should have "input" as a required property
    let required = schema["required"].as_array().unwrap();
    assert!(required.contains(&serde_json::json!("input")));
    let properties = schema["properties"].as_object().unwrap();
    assert!(properties.contains_key("input"));
}

/// Mirrors `test_get_output_schema`.
#[test]
fn test_get_output_schema() {
    let store: Arc<Mutex<HashMap<String, Arc<Mutex<InMemoryChatMessageHistory>>>>> =
        Arc::new(Mutex::new(HashMap::new()));
    let factory = make_session_factory(store);

    let with_history = RunnableWithMessageHistory::new(
        Box::new(DictReturnsDictOutput),
        factory,
        Some("input".into()),
        Some("output".into()),
        Some("history".into()),
        None,
    );

    let schema = with_history.get_output_schema();
    assert_eq!(schema["title"], "RunnableWithChatHistoryOutput");
    assert_eq!(schema["type"], "object");
}

/// Mirrors `test_get_input_schema_input_messages`.
///
/// When no `input_messages_key` is set, the input schema should describe
/// a sequence (array) of messages.
#[test]
fn test_get_input_schema_input_messages() {
    let store: Arc<Mutex<HashMap<String, Arc<Mutex<InMemoryChatMessageHistory>>>>> =
        Arc::new(Mutex::new(HashMap::new()));
    let factory = make_session_factory(store);

    let with_history = RunnableWithMessageHistory::new(
        Box::new(ConcatHumanMessages),
        factory,
        None, // no input_messages_key → expects bare message list
        None,
        None,
        None,
    );

    let schema = with_history.get_input_schema();
    assert_eq!(schema["title"], "RunnableWithChatHistoryInput");
    assert_eq!(schema["type"], "array");
}
