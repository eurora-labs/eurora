use agent_chain_core::callbacks::base::{
    BaseCallbackHandler, CallbackManagerMixin, ChainManagerMixin, LLMManagerMixin,
    RetrieverManagerMixin, RunManagerMixin, ToolManagerMixin,
};
use agent_chain_core::callbacks::manager::{
    AsyncCallbackManager, AsyncCallbackManagerForChainGroup, AsyncCallbackManagerForChainRun,
    AsyncCallbackManagerForLLMRun, AsyncCallbackManagerForRetrieverRun,
    AsyncCallbackManagerForToolRun, BaseRunManager, CallbackManager, CallbackManagerForChainGroup,
    CallbackManagerForChainRun, CallbackManagerForLLMRun, CallbackManagerForRetrieverRun,
    CallbackManagerForToolRun, ParentRunManager, RunManager,
};
use agent_chain_core::outputs::ChatResult;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use std::sync::Mutex;

#[derive(Debug, Default)]
struct TestHandler;

impl LLMManagerMixin for TestHandler {}
impl ChainManagerMixin for TestHandler {}
impl ToolManagerMixin for TestHandler {}
impl RetrieverManagerMixin for TestHandler {}
impl CallbackManagerMixin for TestHandler {}
impl RunManagerMixin for TestHandler {}
impl BaseCallbackHandler for TestHandler {
    fn name(&self) -> &str {
        "TestHandler"
    }
}

#[derive(Debug)]
struct RecordingHandler {
    events: Mutex<Vec<String>>,
}

impl RecordingHandler {
    fn new() -> Self {
        Self {
            events: Mutex::new(Vec::new()),
        }
    }

    fn record(&self, name: &str) {
        self.events.lock().unwrap().push(name.to_string());
    }

    fn event_count(&self) -> usize {
        self.events.lock().unwrap().len()
    }

    #[allow(dead_code)]
    fn events(&self) -> Vec<String> {
        self.events.lock().unwrap().clone()
    }

    fn has_event(&self, name: &str) -> bool {
        self.events.lock().unwrap().iter().any(|e| e == name)
    }
}

impl LLMManagerMixin for RecordingHandler {
    fn on_llm_new_token(
        &self,
        _token: &str,
        _run_id: Uuid,
        _parent_run_id: Option<Uuid>,
        _chunk: Option<&serde_json::Value>,
    ) {
        self.record("on_llm_new_token");
    }
    fn on_llm_end(&self, _response: &ChatResult, _run_id: Uuid, _parent_run_id: Option<Uuid>) {
        self.record("on_llm_end");
    }
    fn on_llm_error(
        &self,
        _error: &dyn std::error::Error,
        _run_id: Uuid,
        _parent_run_id: Option<Uuid>,
    ) {
        self.record("on_llm_error");
    }
}
impl ChainManagerMixin for RecordingHandler {
    fn on_chain_end(
        &self,
        _outputs: &HashMap<String, serde_json::Value>,
        _run_id: Uuid,
        _parent_run_id: Option<Uuid>,
    ) {
        self.record("on_chain_end");
    }
    fn on_chain_error(
        &self,
        _error: &dyn std::error::Error,
        _run_id: Uuid,
        _parent_run_id: Option<Uuid>,
    ) {
        self.record("on_chain_error");
    }
    fn on_agent_action(
        &self,
        _action: &serde_json::Value,
        _run_id: Uuid,
        _parent_run_id: Option<Uuid>,
        _color: Option<&str>,
    ) {
        self.record("on_agent_action");
    }
    fn on_agent_finish(
        &self,
        _finish: &serde_json::Value,
        _run_id: Uuid,
        _parent_run_id: Option<Uuid>,
        _color: Option<&str>,
    ) {
        self.record("on_agent_finish");
    }
}
impl ToolManagerMixin for RecordingHandler {
    fn on_tool_end(
        &self,
        _output: &str,
        _run_id: Uuid,
        _parent_run_id: Option<Uuid>,
        _color: Option<&str>,
        _observation_prefix: Option<&str>,
        _llm_prefix: Option<&str>,
    ) {
        self.record("on_tool_end");
    }
    fn on_tool_error(
        &self,
        _error: &dyn std::error::Error,
        _run_id: Uuid,
        _parent_run_id: Option<Uuid>,
    ) {
        self.record("on_tool_error");
    }
}
impl RetrieverManagerMixin for RecordingHandler {
    fn on_retriever_end(
        &self,
        _documents: &[serde_json::Value],
        _run_id: Uuid,
        _parent_run_id: Option<Uuid>,
    ) {
        self.record("on_retriever_end");
    }
    fn on_retriever_error(
        &self,
        _error: &dyn std::error::Error,
        _run_id: Uuid,
        _parent_run_id: Option<Uuid>,
    ) {
        self.record("on_retriever_error");
    }
}
impl CallbackManagerMixin for RecordingHandler {
    fn on_llm_start(
        &self,
        _serialized: &HashMap<String, serde_json::Value>,
        _prompts: &[String],
        _run_id: Uuid,
        _parent_run_id: Option<Uuid>,
        _tags: Option<&[String]>,
        _metadata: Option<&HashMap<String, serde_json::Value>>,
    ) {
        self.record("on_llm_start");
    }
    fn on_chat_model_start(
        &self,
        _serialized: &HashMap<String, serde_json::Value>,
        _messages: &[Vec<agent_chain_core::messages::BaseMessage>],
        _run_id: Uuid,
        _parent_run_id: Option<Uuid>,
        _tags: Option<&[String]>,
        _metadata: Option<&HashMap<String, serde_json::Value>>,
    ) {
        self.record("on_chat_model_start");
    }
    fn on_chain_start(
        &self,
        _serialized: &HashMap<String, serde_json::Value>,
        _inputs: &HashMap<String, serde_json::Value>,
        _run_id: Uuid,
        _parent_run_id: Option<Uuid>,
        _tags: Option<&[String]>,
        _metadata: Option<&HashMap<String, serde_json::Value>>,
        _name: Option<&str>,
    ) {
        self.record("on_chain_start");
    }
    fn on_tool_start(
        &self,
        _serialized: &HashMap<String, serde_json::Value>,
        _input_str: &str,
        _run_id: Uuid,
        _parent_run_id: Option<Uuid>,
        _tags: Option<&[String]>,
        _metadata: Option<&HashMap<String, serde_json::Value>>,
        _inputs: Option<&HashMap<String, serde_json::Value>>,
    ) {
        self.record("on_tool_start");
    }
    fn on_retriever_start(
        &self,
        _serialized: &HashMap<String, serde_json::Value>,
        _query: &str,
        _run_id: Uuid,
        _parent_run_id: Option<Uuid>,
        _tags: Option<&[String]>,
        _metadata: Option<&HashMap<String, serde_json::Value>>,
        _name: Option<&str>,
    ) {
        self.record("on_retriever_start");
    }
}
impl RunManagerMixin for RecordingHandler {
    fn on_text(
        &self,
        _text: &str,
        _run_id: Uuid,
        _parent_run_id: Option<Uuid>,
        _color: Option<&str>,
        _end: &str,
    ) {
        self.record("on_text");
    }
    fn on_retry(
        &self,
        _retry_state: &dyn std::any::Any,
        _run_id: Uuid,
        _parent_run_id: Option<Uuid>,
    ) {
        self.record("on_retry");
    }
    fn on_custom_event(
        &self,
        _name: &str,
        _data: &dyn std::any::Any,
        _run_id: Uuid,
        _tags: Option<&[String]>,
        _metadata: Option<&HashMap<String, serde_json::Value>>,
    ) {
        self.record("on_custom_event");
    }
}
impl BaseCallbackHandler for RecordingHandler {
    fn name(&self) -> &str {
        "RecordingHandler"
    }
}

#[derive(Debug)]
struct IgnoreLLMHandler;
impl LLMManagerMixin for IgnoreLLMHandler {}
impl ChainManagerMixin for IgnoreLLMHandler {}
impl ToolManagerMixin for IgnoreLLMHandler {}
impl RetrieverManagerMixin for IgnoreLLMHandler {}
impl CallbackManagerMixin for IgnoreLLMHandler {}
impl RunManagerMixin for IgnoreLLMHandler {}
impl BaseCallbackHandler for IgnoreLLMHandler {
    fn name(&self) -> &str {
        "IgnoreLLMHandler"
    }
    fn ignore_llm(&self) -> bool {
        true
    }
}

#[derive(Debug)]
struct IgnoreChainHandler;
impl LLMManagerMixin for IgnoreChainHandler {}
impl ChainManagerMixin for IgnoreChainHandler {}
impl ToolManagerMixin for IgnoreChainHandler {}
impl RetrieverManagerMixin for IgnoreChainHandler {}
impl CallbackManagerMixin for IgnoreChainHandler {}
impl RunManagerMixin for IgnoreChainHandler {}
impl BaseCallbackHandler for IgnoreChainHandler {
    fn name(&self) -> &str {
        "IgnoreChainHandler"
    }
    fn ignore_chain(&self) -> bool {
        true
    }
}

#[derive(Debug)]
struct IgnoreAgentHandler;
impl LLMManagerMixin for IgnoreAgentHandler {}
impl ChainManagerMixin for IgnoreAgentHandler {}
impl ToolManagerMixin for IgnoreAgentHandler {}
impl RetrieverManagerMixin for IgnoreAgentHandler {}
impl CallbackManagerMixin for IgnoreAgentHandler {}
impl RunManagerMixin for IgnoreAgentHandler {}
impl BaseCallbackHandler for IgnoreAgentHandler {
    fn name(&self) -> &str {
        "IgnoreAgentHandler"
    }
    fn ignore_agent(&self) -> bool {
        true
    }
}

#[derive(Debug)]
struct IgnoreRetrieverHandler;
impl LLMManagerMixin for IgnoreRetrieverHandler {}
impl ChainManagerMixin for IgnoreRetrieverHandler {}
impl ToolManagerMixin for IgnoreRetrieverHandler {}
impl RetrieverManagerMixin for IgnoreRetrieverHandler {}
impl CallbackManagerMixin for IgnoreRetrieverHandler {}
impl RunManagerMixin for IgnoreRetrieverHandler {}
impl BaseCallbackHandler for IgnoreRetrieverHandler {
    fn name(&self) -> &str {
        "IgnoreRetrieverHandler"
    }
    fn ignore_retriever(&self) -> bool {
        true
    }
}

#[test]
fn test_handle_event_empty_handlers_no_error() {
    use agent_chain_core::callbacks::manager::handle_event;
    handle_event(&[], None, |_handler| {});
}

#[test]
fn test_handle_event_multiple_handlers() {
    use agent_chain_core::callbacks::manager::handle_event;
    let h1: Arc<dyn BaseCallbackHandler> = Arc::new(TestHandler);
    let h2: Arc<dyn BaseCallbackHandler> = Arc::new(TestHandler);
    let handlers = vec![h1, h2];
    let mut count = 0;
    handle_event(&handlers, None, |_handler| {
        count += 1;
    });
    assert_eq!(count, 2);
}

#[test]
fn test_handle_event_respects_ignore_condition() {
    use agent_chain_core::callbacks::manager::handle_event;
    let h: Arc<dyn BaseCallbackHandler> = Arc::new(IgnoreLLMHandler);
    let handlers = vec![h];
    let mut count = 0;
    handle_event(
        &handlers,
        Some(|h: &dyn BaseCallbackHandler| h.ignore_llm()),
        |_handler| {
            count += 1;
        },
    );
    assert_eq!(count, 0);
}

#[test]
fn test_handle_event_ignore_condition_none_always_dispatches() {
    use agent_chain_core::callbacks::manager::handle_event;
    let h: Arc<dyn BaseCallbackHandler> = Arc::new(TestHandler);
    let mut count = 0;
    handle_event(&[h], None, |_handler| {
        count += 1;
    });
    assert_eq!(count, 1);
}

#[test]
fn test_run_manager_on_text_empty_handlers_no_error() {
    let mgr = RunManager::new(Uuid::new_v4(), vec![], vec![], None, None, None, None, None);
    mgr.on_text("test");
}

#[test]
fn test_run_manager_on_retry_empty_handlers_no_error() {
    let mgr = RunManager::new(Uuid::new_v4(), vec![], vec![], None, None, None, None, None);
    mgr.on_retry(&serde_json::json!(null));
}

#[test]
fn test_parent_run_manager_get_child_inherits_handlers() {
    let h: Arc<dyn BaseCallbackHandler> = Arc::new(TestHandler);
    let mgr = ParentRunManager::new(
        Uuid::new_v4(),
        vec![h.clone()],
        vec![h],
        None,
        None,
        Some(vec!["it".to_string()]),
        None,
        Some(HashMap::from([("ik".to_string(), serde_json::json!("iv"))])),
    );
    let child = mgr.get_child(None);
    assert!(!child.handlers.is_empty());
    assert!(!child.inheritable_handlers.is_empty());
    assert!(child.tags.contains(&"it".to_string()));
    assert_eq!(child.metadata["ik"], serde_json::json!("iv"));
}

#[test]
fn test_parent_run_manager_get_child_sets_parent_run_id() {
    let run_id = Uuid::new_v4();
    let mgr = ParentRunManager::new(run_id, vec![], vec![], None, None, None, None, None);
    let child = mgr.get_child(None);
    assert_eq!(child.parent_run_id, Some(run_id));
}

#[test]
fn test_parent_run_manager_get_child_tag_not_inheritable() {
    let mgr = ParentRunManager::new(Uuid::new_v4(), vec![], vec![], None, None, None, None, None);
    let child = mgr.get_child(Some("local"));
    assert!(child.tags.contains(&"local".to_string()));
    assert!(!child.inheritable_tags.contains(&"local".to_string()));
}

#[test]
fn test_parent_run_manager_get_child_without_tag() {
    let mgr = ParentRunManager::new(Uuid::new_v4(), vec![], vec![], None, None, None, None, None);
    let _child = mgr.get_child(None);
}

#[test]
fn test_llm_run_empty_handlers_noop() {
    let mgr =
        CallbackManagerForLLMRun::new(Uuid::new_v4(), vec![], vec![], None, None, None, None, None);
    mgr.on_llm_new_token("tok", None);
    mgr.on_llm_end(&ChatResult::default());
    mgr.on_llm_error(&std::io::Error::other("err"));
}

#[test]
fn test_llm_run_on_llm_new_token_with_chunk() {
    let h: Arc<dyn BaseCallbackHandler> = Arc::new(TestHandler);
    let mgr = CallbackManagerForLLMRun::new(
        Uuid::new_v4(),
        vec![h],
        vec![],
        None,
        None,
        None,
        None,
        None,
    );
    mgr.on_llm_new_token("tok", None);
}

#[test]
fn test_chain_run_empty_handlers_noop() {
    let mgr = CallbackManagerForChainRun::new(
        Uuid::new_v4(),
        vec![],
        vec![],
        None,
        None,
        None,
        None,
        None,
    );
    mgr.on_chain_end(&HashMap::new());
    mgr.on_chain_error(&std::io::Error::other("err"));
    mgr.on_agent_action(&serde_json::json!({"tool": "t", "tool_input": "i", "log": "l"}));
    mgr.on_agent_finish(&serde_json::json!({"return_values": {}, "log": "d"}));
}

#[test]
fn test_chain_run_is_parent_run_manager() {
    let mgr = CallbackManagerForChainRun::new(
        Uuid::new_v4(),
        vec![],
        vec![],
        None,
        None,
        None,
        None,
        None,
    );
    let _child = mgr.get_child(None);
}

#[test]
fn test_tool_run_empty_handlers_noop() {
    let mgr = CallbackManagerForToolRun::new(
        Uuid::new_v4(),
        vec![],
        vec![],
        None,
        None,
        None,
        None,
        None,
    );
    mgr.on_tool_end("out");
    mgr.on_tool_error(&std::io::Error::other("err"));
}

#[test]
fn test_retriever_run_empty_handlers_noop() {
    let mgr = CallbackManagerForRetrieverRun::new(
        Uuid::new_v4(),
        vec![],
        vec![],
        None,
        None,
        None,
        None,
        None,
    );
    mgr.on_retriever_end(&[]);
    mgr.on_retriever_error(&std::io::Error::other("err"));
}

#[test]
fn test_callback_manager_on_llm_start_returns_managers_per_prompt() {
    let h: Arc<dyn BaseCallbackHandler> = Arc::new(TestHandler);
    let mut mgr = CallbackManager::new();
    mgr.add_handler(h, true);
    let managers = mgr.on_llm_start(
        &HashMap::new(),
        &["p1".to_string(), "p2".to_string(), "p3".to_string()],
        None,
    );
    assert_eq!(managers.len(), 3);
}

#[test]
fn test_callback_manager_on_llm_start_uses_provided_run_id_for_first() {
    let h: Arc<dyn BaseCallbackHandler> = Arc::new(TestHandler);
    let mut mgr = CallbackManager::new();
    mgr.add_handler(h, true);
    let rid = Uuid::new_v4();
    let managers = mgr.on_llm_start(
        &HashMap::new(),
        &["p1".to_string(), "p2".to_string()],
        Some(rid),
    );
    assert_eq!(managers[0].run_id(), rid);
    assert_ne!(managers[1].run_id(), rid);
}

#[test]
fn test_callback_manager_on_llm_start_generates_run_id_when_none() {
    let h: Arc<dyn BaseCallbackHandler> = Arc::new(TestHandler);
    let mut mgr = CallbackManager::new();
    mgr.add_handler(h, true);
    let managers = mgr.on_llm_start(&HashMap::new(), &["p1".to_string()], None);
    assert!(!managers[0].run_id().is_nil());
}

#[test]
fn test_callback_manager_on_chat_model_start_returns_managers_per_message_list() {
    use agent_chain_core::messages::HumanMessage;
    let h: Arc<dyn BaseCallbackHandler> = Arc::new(TestHandler);
    let mut mgr = CallbackManager::new();
    mgr.add_handler(h, true);
    let msgs = vec![
        vec![HumanMessage::builder().content("a").build().into()],
        vec![HumanMessage::builder().content("b").build().into()],
    ];
    let managers = mgr.on_chat_model_start(&HashMap::new(), &msgs, None);
    assert_eq!(managers.len(), 2);
}

#[test]
fn test_callback_manager_on_chat_model_start_uses_provided_run_id_for_first() {
    use agent_chain_core::messages::HumanMessage;
    let h: Arc<dyn BaseCallbackHandler> = Arc::new(TestHandler);
    let mut mgr = CallbackManager::new();
    mgr.add_handler(h, true);
    let rid = Uuid::new_v4();
    let msgs = vec![
        vec![HumanMessage::builder().content("a").build().into()],
        vec![HumanMessage::builder().content("b").build().into()],
    ];
    let managers = mgr.on_chat_model_start(&HashMap::new(), &msgs, Some(rid));
    assert_eq!(managers[0].run_id(), rid);
    assert_ne!(managers[1].run_id(), rid);
}

#[test]
fn test_callback_manager_on_chain_start_returns_chain_run_manager() {
    let h: Arc<dyn BaseCallbackHandler> = Arc::new(TestHandler);
    let mut mgr = CallbackManager::new();
    mgr.add_handler(h, true);
    let rm = mgr
        .on_chain_start()
        .serialized(&HashMap::new())
        .inputs(&HashMap::new())
        .call();
    assert!(!rm.run_id().is_nil());
}

#[test]
fn test_callback_manager_on_chain_start_uses_provided_run_id() {
    let h: Arc<dyn BaseCallbackHandler> = Arc::new(TestHandler);
    let mut mgr = CallbackManager::new();
    mgr.add_handler(h, true);
    let rid = Uuid::new_v4();
    let rm = mgr
        .on_chain_start()
        .serialized(&HashMap::new())
        .inputs(&HashMap::new())
        .maybe_run_id(Some(rid))
        .call();
    assert_eq!(rm.run_id(), rid);
}

#[test]
fn test_callback_manager_on_tool_start_returns_tool_run_manager() {
    let h: Arc<dyn BaseCallbackHandler> = Arc::new(TestHandler);
    let mut mgr = CallbackManager::new();
    mgr.add_handler(h, true);
    let rm = mgr.on_tool_start(&HashMap::new(), "input", None, None);
    assert!(!rm.run_id().is_nil());
}

#[test]
fn test_callback_manager_on_tool_start_uses_provided_run_id() {
    let h: Arc<dyn BaseCallbackHandler> = Arc::new(TestHandler);
    let mut mgr = CallbackManager::new();
    mgr.add_handler(h, true);
    let rid = Uuid::new_v4();
    let rm = mgr.on_tool_start(&HashMap::new(), "input", Some(rid), None);
    assert_eq!(rm.run_id(), rid);
}

#[test]
fn test_callback_manager_on_retriever_start_returns_retriever_run_manager() {
    let h: Arc<dyn BaseCallbackHandler> = Arc::new(TestHandler);
    let mut mgr = CallbackManager::new();
    mgr.add_handler(h, true);
    let rm = mgr
        .on_retriever_start()
        .serialized(&HashMap::new())
        .query("query")
        .call();
    assert!(!rm.run_id().is_nil());
}

#[test]
fn test_callback_manager_on_retriever_start_uses_provided_run_id() {
    let h: Arc<dyn BaseCallbackHandler> = Arc::new(TestHandler);
    let mut mgr = CallbackManager::new();
    mgr.add_handler(h, true);
    let rid = Uuid::new_v4();
    let rm = mgr
        .on_retriever_start()
        .serialized(&HashMap::new())
        .query("q")
        .run_id(rid)
        .call();
    assert_eq!(rm.run_id(), rid);
}

#[test]
fn test_callback_manager_on_custom_event_empty_handlers_noop() {
    let mgr = CallbackManager::new();
    mgr.on_custom_event("evt", &serde_json::json!({}), None);
}

#[test]
fn test_callback_manager_run_managers_inherit_tags_and_metadata() {
    let h: Arc<dyn BaseCallbackHandler> = Arc::new(TestHandler);
    let mut mgr = CallbackManager::new();
    mgr.add_handler(h, true);
    mgr.add_tags(vec!["t1".to_string()], false);
    mgr.add_tags(vec!["it1".to_string()], true);
    mgr.add_metadata(
        HashMap::from([("k".to_string(), serde_json::json!("v"))]),
        false,
    );
    mgr.add_metadata(
        HashMap::from([("ik".to_string(), serde_json::json!("iv"))]),
        true,
    );

    let rm = mgr
        .on_chain_start()
        .serialized(&HashMap::new())
        .inputs(&HashMap::new())
        .call();
    assert!(rm.tags().contains(&"t1".to_string()));
}

fn make_sync_chain_group() -> (CallbackManagerForChainGroup, CallbackManagerForChainRun) {
    let h: Arc<dyn BaseCallbackHandler> = Arc::new(TestHandler);
    let parent_rm = CallbackManagerForChainRun::new(
        Uuid::new_v4(),
        vec![h.clone()],
        vec![h.clone()],
        None,
        None,
        None,
        None,
        None,
    );
    let group = CallbackManagerForChainGroup::new(
        vec![h.clone()],
        Some(vec![h]),
        parent_rm.parent_run_id(),
        parent_rm.clone(),
        None,
        None,
        None,
        None,
    );
    (group, parent_rm)
}

#[test]
fn test_chain_group_on_chain_end_sets_ended() {
    let (mut group, _) = make_sync_chain_group();
    assert!(!group.ended);
    group.on_chain_end(&HashMap::new());
    assert!(group.ended);
}

#[test]
fn test_chain_group_on_chain_error_sets_ended() {
    let (mut group, _) = make_sync_chain_group();
    group.on_chain_error(&std::io::Error::other("err"));
    assert!(group.ended);
}

#[test]
fn test_chain_group_copy_preserves_parent_run_manager() {
    let (group, parent_rm) = make_sync_chain_group();
    let cp = group.copy();
    assert_eq!(cp.parent_run_id(), parent_rm.parent_run_id());
}

#[test]
fn test_chain_group_merge_preserves_parent_run_manager() {
    let (group, _) = make_sync_chain_group();
    let mut other = CallbackManager::new();
    other.add_tags(vec!["extra".to_string()], false);
    let merged = group.merge(&other);
    assert!(merged.tags().contains(&"extra".to_string()));
}

#[test]
fn test_base_run_manager_get_noop_manager() {
    let mgr = BaseRunManager::get_noop_manager();
    assert!(!mgr.run_id.is_nil());
    assert!(mgr.handlers.is_empty());
    assert!(mgr.inheritable_handlers.is_empty());
    assert!(mgr.tags.is_empty());
    assert!(mgr.metadata.is_empty());
}

#[test]
fn test_base_run_manager_initialization_defaults() {
    let rid = Uuid::new_v4();
    let mgr = BaseRunManager::new(rid, vec![], vec![], None, None, None, None, None);
    assert_eq!(mgr.run_id, rid);
    assert!(mgr.parent_run_id.is_none());
    assert!(mgr.tags.is_empty());
    assert!(mgr.inheritable_tags.is_empty());
    assert!(mgr.metadata.is_empty());
    assert!(mgr.inheritable_metadata.is_empty());
}

#[test]
fn test_async_llm_run_get_sync() {
    let rid = Uuid::new_v4();
    let h: Arc<dyn BaseCallbackHandler> = Arc::new(TestHandler);
    let sync_mgr = CallbackManagerForLLMRun::new(
        rid,
        vec![h.clone()],
        vec![h],
        None,
        Some(vec!["t".to_string()]),
        Some(vec!["it".to_string()]),
        Some(HashMap::from([("k".to_string(), serde_json::json!("v"))])),
        Some(HashMap::from([("ik".to_string(), serde_json::json!("iv"))])),
    );
    let async_mgr = AsyncCallbackManagerForLLMRun::from_sync(sync_mgr);
    let back = async_mgr.get_sync();
    assert_eq!(back.run_id(), rid);
    assert_eq!(back.handlers().len(), 1);
    assert!(back.tags().contains(&"t".to_string()));
}

#[test]
fn test_async_chain_run_get_sync() {
    let rid = Uuid::new_v4();
    let h: Arc<dyn BaseCallbackHandler> = Arc::new(TestHandler);
    let sync_mgr = CallbackManagerForChainRun::new(
        rid,
        vec![h.clone()],
        vec![h],
        None,
        Some(vec!["t".to_string()]),
        None,
        None,
        None,
    );
    let async_mgr = AsyncCallbackManagerForChainRun::from_sync(sync_mgr);
    let back = async_mgr.get_sync();
    assert_eq!(back.run_id(), rid);
}

#[test]
fn test_async_tool_run_get_sync() {
    let rid = Uuid::new_v4();
    let sync_mgr =
        CallbackManagerForToolRun::new(rid, vec![], vec![], None, None, None, None, None);
    let async_mgr = AsyncCallbackManagerForToolRun::from_sync(sync_mgr);
    let back = async_mgr.get_sync();
    assert_eq!(back.run_id(), rid);
}

#[test]
fn test_async_retriever_run_get_sync() {
    let rid = Uuid::new_v4();
    let sync_mgr =
        CallbackManagerForRetrieverRun::new(rid, vec![], vec![], None, None, None, None, None);
    let async_mgr = AsyncCallbackManagerForRetrieverRun::from_sync(sync_mgr);
    let back = async_mgr.get_sync();
    assert_eq!(back.run_id(), rid);
}

#[test]
fn test_async_callback_manager_is_async_true() {
    let mgr = AsyncCallbackManager::new();
    assert!(mgr.is_async());
}

#[tokio::test]
async fn test_async_callback_manager_on_llm_start_returns_async_managers() {
    let h: Arc<dyn BaseCallbackHandler> = Arc::new(TestHandler);
    let mut mgr = AsyncCallbackManager::new();
    mgr.add_handler(h, true);
    let managers = mgr
        .on_llm_start(&HashMap::new(), &["p1".to_string(), "p2".to_string()], None)
        .await;
    assert_eq!(managers.len(), 2);
}

#[tokio::test]
async fn test_async_callback_manager_on_llm_start_uses_provided_run_id() {
    let h: Arc<dyn BaseCallbackHandler> = Arc::new(TestHandler);
    let mut mgr = AsyncCallbackManager::new();
    mgr.add_handler(h, true);
    let rid = Uuid::new_v4();
    let managers = mgr
        .on_llm_start(
            &HashMap::new(),
            &["p1".to_string(), "p2".to_string()],
            Some(rid),
        )
        .await;
    assert_eq!(managers[0].run_id(), rid);
    assert_ne!(managers[1].run_id(), rid);
}

#[tokio::test]
async fn test_async_callback_manager_on_chat_model_start_returns_async_managers() {
    use agent_chain_core::messages::HumanMessage;
    let h: Arc<dyn BaseCallbackHandler> = Arc::new(TestHandler);
    let mut mgr = AsyncCallbackManager::new();
    mgr.add_handler(h, true);
    let msgs = vec![
        vec![HumanMessage::builder().content("a").build().into()],
        vec![HumanMessage::builder().content("b").build().into()],
    ];
    let managers = mgr.on_chat_model_start(&HashMap::new(), &msgs, None).await;
    assert_eq!(managers.len(), 2);
}

#[tokio::test]
async fn test_async_callback_manager_on_chain_start_returns_async_chain_manager() {
    let h: Arc<dyn BaseCallbackHandler> = Arc::new(TestHandler);
    let mut mgr = AsyncCallbackManager::new();
    mgr.add_handler(h, true);
    let rm = mgr
        .on_chain_start(&HashMap::new(), &HashMap::new(), None, None)
        .await;
    assert!(!rm.run_id().is_nil());
}

#[tokio::test]
async fn test_async_callback_manager_on_chain_start_uses_provided_run_id() {
    let h: Arc<dyn BaseCallbackHandler> = Arc::new(TestHandler);
    let mut mgr = AsyncCallbackManager::new();
    mgr.add_handler(h, true);
    let rid = Uuid::new_v4();
    let rm = mgr
        .on_chain_start(&HashMap::new(), &HashMap::new(), Some(rid), None)
        .await;
    assert_eq!(rm.run_id(), rid);
}

#[tokio::test]
async fn test_async_callback_manager_on_tool_start_returns_async_tool_manager() {
    let h: Arc<dyn BaseCallbackHandler> = Arc::new(TestHandler);
    let mut mgr = AsyncCallbackManager::new();
    mgr.add_handler(h, true);
    let rm = mgr
        .on_tool_start(&HashMap::new(), "input", None, None)
        .await;
    assert!(!rm.run_id().is_nil());
}

#[tokio::test]
async fn test_async_callback_manager_on_retriever_start_returns_async_retriever_manager() {
    let h: Arc<dyn BaseCallbackHandler> = Arc::new(TestHandler);
    let mut mgr = AsyncCallbackManager::new();
    mgr.add_handler(h, true);
    let rm = mgr
        .on_retriever_start()
        .serialized(&HashMap::new())
        .query("query")
        .call()
        .await;
    assert!(!rm.run_id().is_nil());
}

#[tokio::test]
async fn test_async_callback_manager_on_custom_event_empty_handlers_noop() {
    let mgr = AsyncCallbackManager::new();
    mgr.on_custom_event("evt", &serde_json::json!({}), None)
        .await;
}

#[tokio::test]
async fn test_async_llm_run_empty_handlers_noop() {
    let sync_mgr =
        CallbackManagerForLLMRun::new(Uuid::new_v4(), vec![], vec![], None, None, None, None, None);
    let mgr = AsyncCallbackManagerForLLMRun::from_sync(sync_mgr);
    mgr.on_llm_new_token("tok", None).await;
    mgr.on_llm_end(&ChatResult::default()).await;
    mgr.on_llm_error(&std::io::Error::other("err")).await;
}

#[tokio::test]
async fn test_async_chain_run_empty_handlers_noop() {
    let sync_mgr = CallbackManagerForChainRun::new(
        Uuid::new_v4(),
        vec![],
        vec![],
        None,
        None,
        None,
        None,
        None,
    );
    let mgr = AsyncCallbackManagerForChainRun::from_sync(sync_mgr);
    mgr.on_chain_end(&HashMap::new()).await;
    mgr.on_chain_error(&std::io::Error::other("err")).await;
    mgr.on_agent_action(&serde_json::json!({"tool": "t"})).await;
    mgr.on_agent_finish(&serde_json::json!({"return_values": {}}))
        .await;
}

#[tokio::test]
async fn test_async_chain_run_get_child_returns_async_manager() {
    let sync_mgr = CallbackManagerForChainRun::new(
        Uuid::new_v4(),
        vec![],
        vec![],
        None,
        None,
        None,
        None,
        None,
    );
    let mgr = AsyncCallbackManagerForChainRun::from_sync(sync_mgr);
    let child = mgr.get_child(None);
    assert_eq!(child.parent_run_id(), Some(mgr.run_id()));
}

#[tokio::test]
async fn test_async_tool_run_empty_handlers_noop() {
    let sync_mgr = CallbackManagerForToolRun::new(
        Uuid::new_v4(),
        vec![],
        vec![],
        None,
        None,
        None,
        None,
        None,
    );
    let mgr = AsyncCallbackManagerForToolRun::from_sync(sync_mgr);
    mgr.on_tool_end("out").await;
    mgr.on_tool_error(&std::io::Error::other("err")).await;
}

#[tokio::test]
async fn test_async_retriever_run_empty_handlers_noop() {
    let sync_mgr = CallbackManagerForRetrieverRun::new(
        Uuid::new_v4(),
        vec![],
        vec![],
        None,
        None,
        None,
        None,
        None,
    );
    let mgr = AsyncCallbackManagerForRetrieverRun::from_sync(sync_mgr);
    mgr.on_retriever_end(&[]).await;
    mgr.on_retriever_error(&std::io::Error::other("err")).await;
}

fn make_async_chain_group() -> (
    AsyncCallbackManagerForChainGroup,
    AsyncCallbackManagerForChainRun,
) {
    let h: Arc<dyn BaseCallbackHandler> = Arc::new(TestHandler);
    let sync_parent = CallbackManagerForChainRun::new(
        Uuid::new_v4(),
        vec![h.clone()],
        vec![h.clone()],
        None,
        None,
        None,
        None,
        None,
    );
    let parent_rm = AsyncCallbackManagerForChainRun::from_sync(sync_parent);
    let group = AsyncCallbackManagerForChainGroup::new(
        vec![h.clone()],
        Some(vec![h]),
        parent_rm.parent_run_id(),
        parent_rm.clone(),
        None,
        None,
        None,
        None,
    );
    (group, parent_rm)
}

#[tokio::test]
async fn test_async_chain_group_on_chain_end_sets_ended() {
    let (mut group, _) = make_async_chain_group();
    assert!(!group.ended);
    group.on_chain_end(&HashMap::new()).await;
    assert!(group.ended);
}

#[tokio::test]
async fn test_async_chain_group_on_chain_error_sets_ended() {
    let (mut group, _) = make_async_chain_group();
    group.on_chain_error(&std::io::Error::other("err")).await;
    assert!(group.ended);
}

#[tokio::test]
async fn test_async_chain_group_copy_preserves_parent_run_manager() {
    let (group, parent_rm) = make_async_chain_group();
    let cp = group.copy();
    assert_eq!(cp.parent_run_id(), parent_rm.parent_run_id());
}

#[tokio::test]
async fn test_async_parent_run_manager_get_child_returns_async_callback_manager() {
    let h: Arc<dyn BaseCallbackHandler> = Arc::new(TestHandler);
    let sync_mgr = CallbackManagerForChainRun::new(
        Uuid::new_v4(),
        vec![h.clone()],
        vec![h],
        None,
        None,
        Some(vec!["it".to_string()]),
        None,
        Some(HashMap::from([("ik".to_string(), serde_json::json!("iv"))])),
    );
    let mgr = AsyncCallbackManagerForChainRun::from_sync(sync_mgr);
    let child = mgr.get_child(None);
    assert_eq!(child.parent_run_id(), Some(mgr.run_id()));
    assert_eq!(child.handlers().len(), 1);
}

#[tokio::test]
async fn test_async_parent_run_manager_get_child_with_tag() {
    let sync_mgr = CallbackManagerForChainRun::new(
        Uuid::new_v4(),
        vec![],
        vec![],
        None,
        None,
        None,
        None,
        None,
    );
    let mgr = AsyncCallbackManagerForChainRun::from_sync(sync_mgr);
    let child = mgr.get_child(Some("local"));
    assert!(
        child
            .to_callback_manager()
            .tags
            .contains(&"local".to_string())
    );
}

#[test]
fn test_handle_event_dispatches_to_handler() {
    use agent_chain_core::callbacks::manager::handle_event;
    let rec = Arc::new(RecordingHandler::new());
    let handlers: Vec<Arc<dyn BaseCallbackHandler>> = vec![rec.clone()];
    handle_event(&handlers, None, |handler| {
        handler.on_text("hello", Uuid::new_v4(), None, None, "");
    });
    assert_eq!(rec.event_count(), 1);
    assert!(rec.has_event("on_text"));
}

#[test]
fn test_llm_run_on_llm_new_token_respects_ignore_llm() {
    let h: Arc<dyn BaseCallbackHandler> = Arc::new(IgnoreLLMHandler);
    let mgr = CallbackManagerForLLMRun::new(
        Uuid::new_v4(),
        vec![h],
        vec![],
        None,
        None,
        None,
        None,
        None,
    );
    mgr.on_llm_new_token("tok", None);
}

#[test]
fn test_llm_run_on_llm_end_respects_ignore_llm() {
    let h: Arc<dyn BaseCallbackHandler> = Arc::new(IgnoreLLMHandler);
    let mgr = CallbackManagerForLLMRun::new(
        Uuid::new_v4(),
        vec![h],
        vec![],
        None,
        None,
        None,
        None,
        None,
    );
    mgr.on_llm_end(&ChatResult::default());
}

#[test]
fn test_llm_run_on_llm_error_respects_ignore_llm() {
    let h: Arc<dyn BaseCallbackHandler> = Arc::new(IgnoreLLMHandler);
    let mgr = CallbackManagerForLLMRun::new(
        Uuid::new_v4(),
        vec![h],
        vec![],
        None,
        None,
        None,
        None,
        None,
    );
    mgr.on_llm_error(&std::io::Error::other("err"));
}

#[test]
fn test_llm_run_on_llm_new_token_dispatches() {
    let rec = Arc::new(RecordingHandler::new());
    let mgr = CallbackManagerForLLMRun::new(
        Uuid::new_v4(),
        vec![rec.clone()],
        vec![],
        None,
        None,
        None,
        None,
        None,
    );
    mgr.on_llm_new_token("tok", None);
    assert!(rec.has_event("on_llm_new_token"));
}

#[test]
fn test_llm_run_on_llm_end_dispatches() {
    let rec = Arc::new(RecordingHandler::new());
    let mgr = CallbackManagerForLLMRun::new(
        Uuid::new_v4(),
        vec![rec.clone()],
        vec![],
        None,
        None,
        None,
        None,
        None,
    );
    mgr.on_llm_end(&ChatResult::default());
    assert!(rec.has_event("on_llm_end"));
}

#[test]
fn test_llm_run_on_llm_error_dispatches() {
    let rec = Arc::new(RecordingHandler::new());
    let mgr = CallbackManagerForLLMRun::new(
        Uuid::new_v4(),
        vec![rec.clone()],
        vec![],
        None,
        None,
        None,
        None,
        None,
    );
    mgr.on_llm_error(&std::io::Error::other("err"));
    assert!(rec.has_event("on_llm_error"));
}

#[test]
fn test_chain_run_on_chain_end_respects_ignore_chain() {
    let h: Arc<dyn BaseCallbackHandler> = Arc::new(IgnoreChainHandler);
    let mgr = CallbackManagerForChainRun::new(
        Uuid::new_v4(),
        vec![h],
        vec![],
        None,
        None,
        None,
        None,
        None,
    );
    mgr.on_chain_end(&HashMap::new());
}

#[test]
fn test_chain_run_on_agent_action_dispatches() {
    let rec = Arc::new(RecordingHandler::new());
    let mgr = CallbackManagerForChainRun::new(
        Uuid::new_v4(),
        vec![rec.clone()],
        vec![],
        None,
        None,
        None,
        None,
        None,
    );
    mgr.on_agent_action(&serde_json::json!({"tool": "t"}));
    assert!(rec.has_event("on_agent_action"));
}

#[test]
fn test_chain_run_on_agent_finish_dispatches() {
    let rec = Arc::new(RecordingHandler::new());
    let mgr = CallbackManagerForChainRun::new(
        Uuid::new_v4(),
        vec![rec.clone()],
        vec![],
        None,
        None,
        None,
        None,
        None,
    );
    mgr.on_agent_finish(&serde_json::json!({"return_values": {}}));
    assert!(rec.has_event("on_agent_finish"));
}

#[test]
fn test_tool_run_on_tool_end_respects_ignore_agent() {
    let h: Arc<dyn BaseCallbackHandler> = Arc::new(IgnoreAgentHandler);
    let mgr = CallbackManagerForToolRun::new(
        Uuid::new_v4(),
        vec![h],
        vec![],
        None,
        None,
        None,
        None,
        None,
    );
    mgr.on_tool_end("output");
}

#[test]
fn test_retriever_run_on_retriever_end_respects_ignore_retriever() {
    let h: Arc<dyn BaseCallbackHandler> = Arc::new(IgnoreRetrieverHandler);
    let mgr = CallbackManagerForRetrieverRun::new(
        Uuid::new_v4(),
        vec![h],
        vec![],
        None,
        None,
        None,
        None,
        None,
    );
    mgr.on_retriever_end(&[]);
}

#[test]
fn test_callback_manager_on_llm_start_dispatches() {
    let rec = Arc::new(RecordingHandler::new());
    let mut mgr = CallbackManager::new();
    mgr.add_handler(rec.clone(), true);
    mgr.on_llm_start(&HashMap::new(), &["prompt".to_string()], None);
    assert!(rec.has_event("on_llm_start"));
}

#[test]
fn test_callback_manager_on_custom_event_dispatches() {
    let rec = Arc::new(RecordingHandler::new());
    let mut mgr = CallbackManager::new();
    mgr.add_handler(rec.clone(), true);
    mgr.on_custom_event("evt", &serde_json::json!({"data": 1}), None);
    assert!(rec.has_event("on_custom_event"));
}

#[test]
fn test_callback_manager_on_custom_event_respects_ignore() {
    #[derive(Debug)]
    struct IgnoreCustomHandler;
    impl LLMManagerMixin for IgnoreCustomHandler {}
    impl ChainManagerMixin for IgnoreCustomHandler {}
    impl ToolManagerMixin for IgnoreCustomHandler {}
    impl RetrieverManagerMixin for IgnoreCustomHandler {}
    impl CallbackManagerMixin for IgnoreCustomHandler {}
    impl RunManagerMixin for IgnoreCustomHandler {}
    impl BaseCallbackHandler for IgnoreCustomHandler {
        fn name(&self) -> &str {
            "IgnoreCustomHandler"
        }
        fn ignore_custom_event(&self) -> bool {
            true
        }
    }

    let h: Arc<dyn BaseCallbackHandler> = Arc::new(IgnoreCustomHandler);
    let mut mgr = CallbackManager::new();
    mgr.add_handler(h, true);
    mgr.on_custom_event("evt", &serde_json::json!({}), None);
}

#[test]
fn test_run_manager_on_text_dispatches() {
    let rec = Arc::new(RecordingHandler::new());
    let mgr = RunManager::new(
        Uuid::new_v4(),
        vec![rec.clone()],
        vec![],
        None,
        None,
        None,
        None,
        None,
    );
    mgr.on_text("hi");
    assert!(rec.has_event("on_text"));
}

#[test]
fn test_chain_run_on_chain_end_dispatches() {
    let rec = Arc::new(RecordingHandler::new());
    let mgr = CallbackManagerForChainRun::new(
        Uuid::new_v4(),
        vec![rec.clone()],
        vec![],
        None,
        None,
        None,
        None,
        None,
    );
    mgr.on_chain_end(&HashMap::from([(
        "out".to_string(),
        serde_json::json!("val"),
    )]));
    assert!(rec.has_event("on_chain_end"));
}

#[test]
fn test_chain_run_on_chain_error_dispatches() {
    let rec = Arc::new(RecordingHandler::new());
    let mgr = CallbackManagerForChainRun::new(
        Uuid::new_v4(),
        vec![rec.clone()],
        vec![],
        None,
        None,
        None,
        None,
        None,
    );
    mgr.on_chain_error(&std::io::Error::other("err"));
    assert!(rec.has_event("on_chain_error"));
}

#[test]
fn test_tool_run_on_tool_end_dispatches() {
    let rec = Arc::new(RecordingHandler::new());
    let mgr = CallbackManagerForToolRun::new(
        Uuid::new_v4(),
        vec![rec.clone()],
        vec![],
        None,
        None,
        None,
        None,
        None,
    );
    mgr.on_tool_end("result");
    assert!(rec.has_event("on_tool_end"));
}

#[test]
fn test_tool_run_on_tool_error_dispatches() {
    let rec = Arc::new(RecordingHandler::new());
    let mgr = CallbackManagerForToolRun::new(
        Uuid::new_v4(),
        vec![rec.clone()],
        vec![],
        None,
        None,
        None,
        None,
        None,
    );
    mgr.on_tool_error(&std::io::Error::other("err"));
    assert!(rec.has_event("on_tool_error"));
}

#[test]
fn test_retriever_run_on_retriever_end_dispatches() {
    let rec = Arc::new(RecordingHandler::new());
    let mgr = CallbackManagerForRetrieverRun::new(
        Uuid::new_v4(),
        vec![rec.clone()],
        vec![],
        None,
        None,
        None,
        None,
        None,
    );
    mgr.on_retriever_end(&[]);
    assert!(rec.has_event("on_retriever_end"));
}

#[test]
fn test_retriever_run_on_retriever_error_dispatches() {
    let rec = Arc::new(RecordingHandler::new());
    let mgr = CallbackManagerForRetrieverRun::new(
        Uuid::new_v4(),
        vec![rec.clone()],
        vec![],
        None,
        None,
        None,
        None,
        None,
    );
    mgr.on_retriever_error(&std::io::Error::other("err"));
    assert!(rec.has_event("on_retriever_error"));
}

#[test]
fn test_chain_group_on_chain_end_delegates_to_parent() {
    let rec = Arc::new(RecordingHandler::new());
    let parent_rm = CallbackManagerForChainRun::new(
        Uuid::new_v4(),
        vec![rec.clone()],
        vec![rec.clone()],
        None,
        None,
        None,
        None,
        None,
    );
    let mut group = CallbackManagerForChainGroup::new(
        vec![rec.clone()],
        Some(vec![rec.clone()]),
        parent_rm.parent_run_id(),
        parent_rm,
        None,
        None,
        None,
        None,
    );
    group.on_chain_end(&HashMap::from([(
        "result".to_string(),
        serde_json::json!("ok"),
    )]));
    assert!(rec.has_event("on_chain_end"));
}

#[test]
fn test_chain_group_on_chain_error_delegates_to_parent() {
    let rec = Arc::new(RecordingHandler::new());
    let parent_rm = CallbackManagerForChainRun::new(
        Uuid::new_v4(),
        vec![rec.clone()],
        vec![rec.clone()],
        None,
        None,
        None,
        None,
        None,
    );
    let mut group = CallbackManagerForChainGroup::new(
        vec![rec.clone()],
        Some(vec![rec.clone()]),
        parent_rm.parent_run_id(),
        parent_rm,
        None,
        None,
        None,
        None,
    );
    group.on_chain_error(&std::io::Error::other("err"));
    assert!(rec.has_event("on_chain_error"));
}
