//! Tests for RunnableWithMessageHistory and related types.
//!
//! Mirrors `langchain/libs/core/tests/unit_tests/runnables/test_history.py`

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use agent_chain_core::chat_history::{BaseChatMessageHistory, InMemoryChatMessageHistory};
use agent_chain_core::messages::{AIMessage, BaseMessage, HumanMessage, SystemMessage};
use agent_chain_core::runnables::base::Runnable;
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

    let with_history = RunnableWithMessageHistory::from_history_runnable(
        concat_human_messages(),
        factory,
        None,
        None,
        None,
        None,
    );

    let cfg = config_with(&[("session_id", "1")]);

    // First invocation
    let output = with_history
        .invoke_messages(vec![human("hello")], Some(cfg.clone()))
        .unwrap();
    assert_eq!(output.len(), 1);
    assert_eq!(output[0].content(), "you said: hello");

    // Second invocation — history should include previous messages
    let output = with_history
        .invoke_messages(vec![human("good bye")], Some(cfg.clone()))
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

    let with_history = RunnableWithMessageHistory::from_history_runnable(
        length_runnable(),
        factory,
        None,
        None,
        None,
        None,
    );

    let cfg = config_with(&[("session_id", "5")]);

    let output = with_history
        .invoke_messages(vec![human("hi")], Some(cfg.clone()))
        .unwrap();
    assert_eq!(output[0].content(), "1");

    let output = with_history
        .invoke_messages(vec![human("hi")], Some(cfg.clone()))
        .unwrap();
    assert_eq!(output[0].content(), "3");
}

/// Mirrors `test_using_custom_config_specs` in Python.
///
/// Uses a session factory that takes `user_id` and `thread_id`.
#[test]
fn test_using_custom_config_specs() {
    #[allow(clippy::type_complexity)]
    let store: Arc<Mutex<HashMap<(String, String), Arc<Mutex<InMemoryChatMessageHistory>>>>> =
        Arc::new(Mutex::new(HashMap::new()));

    let factory: GetSessionHistoryFn = {
        let store = store.clone();
        Arc::new(move |params: &HashMap<String, String>| {
            let user_id = params.get("user_id").cloned().unwrap_or_default();
            let thread_id = params.get("thread_id").cloned().unwrap_or_default();
            let key = (user_id, thread_id);
            let mut guard = store.lock().unwrap();
            let entry = guard
                .entry(key)
                .or_insert_with(|| Arc::new(Mutex::new(InMemoryChatMessageHistory::new())));
            entry.clone()
        })
    };

    let with_history = RunnableWithMessageHistory::from_history_runnable(
        concat_human_messages(),
        factory,
        None,
        None,
        None,
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
                id: "thread_id".into(),
                annotation: "str".into(),
                name: Some("Thread ID".into()),
                description: Some("Unique identifier for the thread.".into()),
                default: None,
                is_shared: true,
                dependencies: None,
            },
        ]),
    );

    // user1, thread 1: "hello"
    let cfg1 = config_with(&[("user_id", "user1"), ("thread_id", "1")]);
    let result = with_history
        .invoke_messages(vec![human("hello")], Some(cfg1.clone()))
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

    // user1, thread 1: "goodbye" — history now includes prior messages
    let result = with_history
        .invoke_messages(vec![human("goodbye")], Some(cfg1.clone()))
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

    // user2, thread 1: "meow"
    let cfg2 = config_with(&[("user_id", "user2"), ("thread_id", "1")]);
    let result = with_history
        .invoke_messages(vec![human("meow")], Some(cfg2))
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

    let with_history = RunnableWithMessageHistory::from_history_runnable(
        concat_human_messages(),
        factory,
        None,
        None,
        None,
        None,
    );

    // Invoke without meaningful config
    let _ = with_history
        .invoke_messages(vec![human("hello")], None)
        .unwrap();
    let _ = with_history
        .invoke_messages(vec![human("hello again")], None)
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

    let with_history = RunnableWithMessageHistory::from_history_runnable(
        concat_human_messages(),
        factory,
        None,
        None,
        None,
        None,
    );

    let cfg_a = config_with(&[("session_id", "a")]);
    let cfg_b = config_with(&[("session_id", "b")]);

    let _ = with_history
        .invoke_messages(vec![human("A1")], Some(cfg_a.clone()))
        .unwrap();
    let _ = with_history
        .invoke_messages(vec![human("B1")], Some(cfg_b.clone()))
        .unwrap();
    let _ = with_history
        .invoke_messages(vec![human("A2")], Some(cfg_a.clone()))
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

    let with_history = RunnableWithMessageHistory::from_history_runnable(
        concat_human_messages(),
        factory,
        None,
        None,
        None,
        None,
    );

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

    let with_history = RunnableWithMessageHistory::from_history_runnable(
        concat_human_messages(),
        factory,
        None,
        None,
        None,
        None,
    );

    let schema = with_history.get_output_schema();
    assert_eq!(schema["title"], "RunnableWithChatHistoryOutput");
    assert_eq!(schema["type"], "array");
}

// ===========================================================================
// Dict input / output tests
// ===========================================================================

fn human_as_value(content: &str) -> Value {
    serde_json::to_value(human(content)).expect("human message serialization should not fail")
}

fn ai_as_value(content: &str) -> Value {
    serde_json::to_value(ai(content)).expect("ai message serialization should not fail")
}

/// Test dict input with `input_messages_key` and `history_messages_key`.
///
/// The inner runnable receives a dict with "question" and "history" keys.
/// History accumulates across invocations.
///
/// Mirrors the Python test with `input_messages_key="question"` and
/// `history_messages_key="history"`.
#[test]
fn test_dict_input_with_history_messages_key() {
    use agent_chain_core::error::Error;
    use agent_chain_core::runnables::history::HistoryInvokeFn;

    let store: Arc<Mutex<HashMap<String, Arc<Mutex<InMemoryChatMessageHistory>>>>> =
        Arc::new(Mutex::new(HashMap::new()));
    let factory = make_session_factory(store.clone());

    let runnable: HistoryInvokeFn = Arc::new(|input: Value, _config| {
        let obj = input
            .as_object()
            .ok_or_else(|| Error::Other("expected dict".into()))?;
        let history: Vec<BaseMessage> =
            serde_json::from_value(obj.get("history").cloned().unwrap_or(Value::Array(vec![])))
                .map_err(|e| Error::Other(format!("history deser: {}", e)))?;
        let question: Vec<BaseMessage> =
            serde_json::from_value(obj.get("question").cloned().unwrap_or(Value::Array(vec![])))
                .map_err(|e| Error::Other(format!("question deser: {}", e)))?;

        let question_text = question
            .iter()
            .map(|m| m.content().to_string())
            .collect::<Vec<_>>()
            .join(", ");
        let response = format!("history={}, question={}", history.len(), question_text);
        let output = vec![BaseMessage::AI(
            AIMessage::builder().content(response).build(),
        )];
        serde_json::to_value(&output).map_err(|e| Error::Other(format!("ser: {}", e)))
    });

    let with_history = RunnableWithMessageHistory::new(
        runnable,
        None,
        factory,
        Some("question".to_string()),
        None,
        Some("history".to_string()),
        None,
    );

    let cfg = config_with(&[("session_id", "dict1")]);

    // First invocation: no history yet
    let output = with_history
        .invoke(
            serde_json::json!({"question": [human_as_value("What is 2+2?")], "ability": "math"}),
            Some(cfg.clone()),
        )
        .expect("first invoke should succeed");

    let messages: Vec<BaseMessage> =
        serde_json::from_value(output).expect("output should deserialize to messages");
    assert!(
        messages[0].content().contains("history=0"),
        "first call should have no history, got: {}",
        messages[0].content()
    );
    assert!(
        messages[0].content().contains("What is 2+2?"),
        "first call should echo question, got: {}",
        messages[0].content()
    );

    // Second invocation: should have history from first call
    let output = with_history
        .invoke(
            serde_json::json!({"question": [human_as_value("What is its inverse?")], "ability": "math"}),
            Some(cfg.clone()),
        )
        .expect("second invoke should succeed");

    let messages: Vec<BaseMessage> =
        serde_json::from_value(output).expect("output should deserialize to messages");
    // History should contain 1 human input + 1 AI output from first call = 2 messages
    assert!(
        messages[0].content().contains("history=2"),
        "second call should see 2 history messages, got: {}",
        messages[0].content()
    );
    assert!(
        messages[0].content().contains("What is its inverse?"),
        "second call should echo question, got: {}",
        messages[0].content()
    );
}

/// Test that `output_messages_key` extracts output messages from a dict output.
///
/// The inner runnable returns a dict with an "answer" key. History should
/// store only the messages from the "answer" key.
#[test]
fn test_dict_input_with_output_messages_key() {
    use agent_chain_core::error::Error;
    use agent_chain_core::runnables::history::HistoryInvokeFn;

    let store: Arc<Mutex<HashMap<String, Arc<Mutex<InMemoryChatMessageHistory>>>>> =
        Arc::new(Mutex::new(HashMap::new()));
    let factory = make_session_factory(store.clone());

    let runnable: HistoryInvokeFn = Arc::new(|input: Value, _config| {
        let messages: Vec<BaseMessage> =
            serde_json::from_value(input).map_err(|e| Error::Other(format!("deser: {}", e)))?;
        let human_texts: Vec<String> = messages
            .iter()
            .filter_map(|m| match m {
                BaseMessage::Human(_) => Some(m.content().to_string()),
                _ => None,
            })
            .collect();
        let response = BaseMessage::AI(
            AIMessage::builder()
                .content(format!("you said: {}", human_texts.join(", ")))
                .build(),
        );
        let response_value =
            serde_json::to_value(&response).map_err(|e| Error::Other(format!("ser: {}", e)))?;
        Ok(serde_json::json!({"answer": [response_value], "extra": "data"}))
    });

    let with_history = RunnableWithMessageHistory::new(
        runnable,
        None,
        factory,
        None,
        Some("answer".to_string()),
        None,
        None,
    );

    let cfg = config_with(&[("session_id", "out1")]);

    let input =
        serde_json::to_value(vec![human("hello")]).expect("input serialization should not fail");
    let output = with_history
        .invoke(input, Some(cfg.clone()))
        .expect("invoke should succeed");

    // Output should be a dict with "answer" key
    assert!(
        output.get("answer").is_some(),
        "output should contain 'answer' key, got: {}",
        output
    );

    // History should have the human input + the AI answer
    let guard = store.lock().expect("store lock should not be poisoned");
    let hist = guard
        .get("out1")
        .expect("session 'out1' should exist")
        .lock()
        .expect("history lock should not be poisoned");
    assert_eq!(
        hist.messages().len(),
        2,
        "history should have 2 messages (input + output)"
    );
    assert_eq!(hist.messages()[0].content(), "hello");
    assert!(
        hist.messages()[1].content().contains("you said: hello"),
        "AI response should contain 'you said: hello', got: {}",
        hist.messages()[1].content()
    );
}

/// Test `get_input_messages` normalization logic:
/// - String input becomes a HumanMessage
/// - Array input is deserialized as Vec<BaseMessage>
/// - Dict input with `input_messages_key` extracts from the correct key
#[test]
fn test_get_input_messages_normalization() {
    use agent_chain_core::runnables::history::HistoryInvokeFn;

    let store: Arc<Mutex<HashMap<String, Arc<Mutex<InMemoryChatMessageHistory>>>>> =
        Arc::new(Mutex::new(HashMap::new()));
    let factory = make_session_factory(store);
    let dummy_runnable: HistoryInvokeFn = Arc::new(|_input, _config| Ok(Value::Array(vec![])));

    // No input_messages_key: string input -> HumanMessage
    let rwmh = RunnableWithMessageHistory::new(
        dummy_runnable.clone(),
        None,
        factory.clone(),
        None,
        None,
        None,
        None,
    );

    let msgs = rwmh
        .get_input_messages(&Value::String("hello".to_string()))
        .expect("string input should parse");
    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0].content(), "hello");
    assert!(
        matches!(msgs[0], BaseMessage::Human(_)),
        "string input should become HumanMessage"
    );

    // No input_messages_key: array input -> Vec<BaseMessage>
    let arr = serde_json::to_value(vec![human("a"), human("b")])
        .expect("array serialization should not fail");
    let msgs = rwmh
        .get_input_messages(&arr)
        .expect("array input should parse");
    assert_eq!(msgs.len(), 2);
    assert_eq!(msgs[0].content(), "a");
    assert_eq!(msgs[1].content(), "b");

    // With input_messages_key: extracts from the specified key in a dict
    let rwmh2 = RunnableWithMessageHistory::new(
        dummy_runnable.clone(),
        None,
        factory.clone(),
        Some("question".to_string()),
        None,
        None,
        None,
    );
    let dict_input = serde_json::json!({"question": [human_as_value("what?")], "other": "data"});
    let msgs = rwmh2
        .get_input_messages(&dict_input)
        .expect("dict input with key should parse");
    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0].content(), "what?");
}

/// Test `get_output_messages` normalization logic:
/// - String output becomes an AIMessage
/// - Dict output with `output_messages_key` extracts from the correct key
/// - Array output is deserialized as Vec<BaseMessage>
#[test]
fn test_get_output_messages_normalization() {
    use agent_chain_core::runnables::history::HistoryInvokeFn;

    let store: Arc<Mutex<HashMap<String, Arc<Mutex<InMemoryChatMessageHistory>>>>> =
        Arc::new(Mutex::new(HashMap::new()));
    let factory = make_session_factory(store);
    let dummy: HistoryInvokeFn = Arc::new(|_input, _config| Ok(Value::Array(vec![])));

    // No output_messages_key: string output -> AIMessage
    let rwmh = RunnableWithMessageHistory::new(
        dummy.clone(),
        None,
        factory.clone(),
        None,
        None,
        None,
        None,
    );

    let msgs = rwmh
        .get_output_messages(&Value::String("response".to_string()))
        .expect("string output should parse");
    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0].content(), "response");
    assert!(
        matches!(msgs[0], BaseMessage::AI(_)),
        "string output should become AIMessage"
    );

    // Array output -> Vec<BaseMessage>
    let arr = serde_json::to_value(vec![ai("first"), ai("second")])
        .expect("array serialization should not fail");
    let msgs = rwmh
        .get_output_messages(&arr)
        .expect("array output should parse");
    assert_eq!(msgs.len(), 2);
    assert_eq!(msgs[0].content(), "first");
    assert_eq!(msgs[1].content(), "second");

    // With output_messages_key: extracts from the specified key in a dict
    let rwmh2 = RunnableWithMessageHistory::new(
        dummy.clone(),
        None,
        factory.clone(),
        None,
        Some("answer".to_string()),
        None,
        None,
    );
    let dict_output = serde_json::json!({"answer": [ai_as_value("42")], "meta": "info"});
    let msgs = rwmh2
        .get_output_messages(&dict_output)
        .expect("dict output with key should parse");
    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0].content(), "42");
}
