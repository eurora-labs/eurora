//! Tests for RunnableWithMessageHistory and related types.
//!
//! Mirrors `langchain/libs/core/tests/unit_tests/runnables/test_history.py`

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use agent_chain_core::chat_history::{BaseChatMessageHistory, InMemoryChatMessageHistory};
use agent_chain_core::messages::{AIMessage, BaseMessage, HumanMessage, SystemMessage};
use agent_chain_core::runnables::config::RunnableConfig;
use agent_chain_core::runnables::history::{
    GetSessionHistoryFn, HistoryRunnable, RunnableWithMessageHistory,
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
// Runnable factories
// ---------------------------------------------------------------------------

/// A runnable that concatenates human-message contents, prefixed with "you said: ".
/// Returns a single AIMessage.
///
/// Mirrors the Python lambda used across many of the tests.
fn concat_human_messages() -> HistoryRunnable {
    HistoryRunnable::from_fn(|messages, _config| {
        let human_contents: Vec<String> = messages
            .iter()
            .filter_map(|m| match m {
                BaseMessage::Human(h) => Some(h.content.as_text()),
                _ => None,
            })
            .collect();
        Ok(vec![BaseMessage::AI(
            AIMessage::builder()
                .content(format!("you said: {}", human_contents.join("\n")))
                .build(),
        )])
    })
}

/// A runnable that returns the message count as a single AIMessage.
/// Mirrors the Python LengthChatModel.
fn length_runnable() -> HistoryRunnable {
    HistoryRunnable::from_fn(|messages, _config| {
        let count = messages.len();
        Ok(vec![BaseMessage::AI(
            AIMessage::builder().content(count.to_string()).build(),
        )])
    })
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

    let with_history = RunnableWithMessageHistory::new(concat_human_messages(), factory, None);

    let cfg = config_with(&[("session_id", "1")]);

    // First invocation
    let output = with_history
        .invoke(vec![human("hello")], Some(cfg.clone()))
        .unwrap();
    assert_eq!(output.len(), 1);
    assert_eq!(output[0].content(), "you said: hello");

    // Second invocation — history should include previous messages
    let output = with_history
        .invoke(vec![human("good bye")], Some(cfg.clone()))
        .unwrap();
    assert_eq!(output.len(), 1);
    assert_eq!(output[0].content(), "you said: hello\ngood bye");

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

/// Mirrors `test_input_messages_output_message` in Python (LengthChatModel).
///
/// First invocation sees 1 message -> "1".
/// Second invocation sees 3 messages (prev human + prev AI + new human) -> "3".
#[test]
fn test_input_messages_output_message() {
    let store: Arc<Mutex<HashMap<String, Arc<Mutex<InMemoryChatMessageHistory>>>>> =
        Arc::new(Mutex::new(HashMap::new()));
    let factory = make_session_factory(store.clone());

    let with_history = RunnableWithMessageHistory::new(length_runnable(), factory, None);

    let cfg = config_with(&[("session_id", "5")]);

    let output = with_history
        .invoke(vec![human("hi")], Some(cfg.clone()))
        .unwrap();
    assert_eq!(output[0].content(), "1");

    let output = with_history
        .invoke(vec![human("hi")], Some(cfg.clone()))
        .unwrap();
    assert_eq!(output[0].content(), "3");
}

/// Mirrors `test_using_custom_config_specs` in Python.
///
/// Uses a session factory that takes `user_id` and `conversation_id`.
#[test]
fn test_using_custom_config_specs() {
    #[allow(clippy::type_complexity)]
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
        concat_human_messages(),
        factory,
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
    let result = with_history
        .invoke(vec![human("hello")], Some(cfg1.clone()))
        .unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].content(), "you said: hello");

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

    // user1, conversation 1: "goodbye" — history now includes prior messages
    let result = with_history
        .invoke(vec![human("goodbye")], Some(cfg1.clone()))
        .unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].content(), "you said: hello\ngoodbye");

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
    let result = with_history
        .invoke(vec![human("meow")], Some(cfg2))
        .unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].content(), "you said: meow");

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

    let with_history = RunnableWithMessageHistory::new(concat_human_messages(), factory, None);

    // Invoke without meaningful config
    let _ = with_history.invoke(vec![human("hello")], None).unwrap();
    let _ = with_history
        .invoke(vec![human("hello again")], None)
        .unwrap();

    let hist = history.lock().unwrap();
    assert_eq!(hist.messages().len(), 4);
}

/// Test that multiple sessions maintain separate histories.
#[test]
fn test_multiple_sessions() {
    let store: Arc<Mutex<HashMap<String, Arc<Mutex<InMemoryChatMessageHistory>>>>> =
        Arc::new(Mutex::new(HashMap::new()));
    let factory = make_session_factory(store.clone());

    let with_history = RunnableWithMessageHistory::new(concat_human_messages(), factory, None);

    let cfg_a = config_with(&[("session_id", "a")]);
    let cfg_b = config_with(&[("session_id", "b")]);

    let _ = with_history
        .invoke(vec![human("A1")], Some(cfg_a.clone()))
        .unwrap();
    let _ = with_history
        .invoke(vec![human("B1")], Some(cfg_b.clone()))
        .unwrap();
    let _ = with_history
        .invoke(vec![human("A2")], Some(cfg_a.clone()))
        .unwrap();

    let guard = store.lock().unwrap();
    let hist_a = guard.get("a").unwrap().lock().unwrap();
    let hist_b = guard.get("b").unwrap().lock().unwrap();

    // Session A: 4 messages (A1, AI response, A2, AI response)
    assert_eq!(hist_a.messages().len(), 4);
    // Session B: 2 messages (B1, AI response)
    assert_eq!(hist_b.messages().len(), 2);
}

// ===========================================================================
// Schema tests
// ===========================================================================

/// Mirrors `test_get_input_schema_input_messages`.
#[test]
fn test_get_input_schema_input_messages() {
    let store: Arc<Mutex<HashMap<String, Arc<Mutex<InMemoryChatMessageHistory>>>>> =
        Arc::new(Mutex::new(HashMap::new()));
    let factory = make_session_factory(store);

    let with_history = RunnableWithMessageHistory::new(concat_human_messages(), factory, None);

    let schema = with_history.get_input_schema();
    assert_eq!(schema["title"], "RunnableWithChatHistoryInput");
    assert_eq!(schema["type"], "array");
}

/// Mirrors `test_get_output_schema`.
#[test]
fn test_get_output_schema() {
    let store: Arc<Mutex<HashMap<String, Arc<Mutex<InMemoryChatMessageHistory>>>>> =
        Arc::new(Mutex::new(HashMap::new()));
    let factory = make_session_factory(store);

    let with_history = RunnableWithMessageHistory::new(concat_human_messages(), factory, None);

    let schema = with_history.get_output_schema();
    assert_eq!(schema["title"], "RunnableWithChatHistoryOutput");
    assert_eq!(schema["type"], "array");
}
