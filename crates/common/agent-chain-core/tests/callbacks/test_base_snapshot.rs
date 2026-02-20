use agent_chain_core::callbacks::base::{
    BaseCallbackHandler, CallbackManagerMixin, ChainManagerMixin, LLMManagerMixin,
    RetrieverManagerMixin, RunManagerMixin, ToolManagerMixin,
};

#[derive(Debug)]
struct SnapshotHandler;

impl LLMManagerMixin for SnapshotHandler {}
impl ChainManagerMixin for SnapshotHandler {}
impl ToolManagerMixin for SnapshotHandler {}
impl RetrieverManagerMixin for SnapshotHandler {}
impl CallbackManagerMixin for SnapshotHandler {}
impl RunManagerMixin for SnapshotHandler {}
impl BaseCallbackHandler for SnapshotHandler {
    fn name(&self) -> &str {
        "SnapshotHandler"
    }
}

#[test]
fn test_sync_handler_has_methods() {
    let handler = SnapshotHandler;

    LLMManagerMixin::on_llm_new_token(&handler, "", uuid::Uuid::nil(), None, None);
    LLMManagerMixin::on_llm_end(&handler, &Default::default(), uuid::Uuid::nil(), None);
    LLMManagerMixin::on_llm_error(
        &handler,
        &std::io::Error::other("e"),
        uuid::Uuid::nil(),
        None,
    );

    CallbackManagerMixin::on_llm_start(
        &handler,
        &Default::default(),
        &[],
        uuid::Uuid::nil(),
        None,
        None,
        None,
    );
    CallbackManagerMixin::on_chat_model_start(
        &handler,
        &Default::default(),
        &[],
        uuid::Uuid::nil(),
        None,
        None,
        None,
    );
    CallbackManagerMixin::on_chain_start(
        &handler,
        &Default::default(),
        &Default::default(),
        uuid::Uuid::nil(),
        None,
        None,
        None,
        None,
    );
    CallbackManagerMixin::on_tool_start(
        &handler,
        &Default::default(),
        "",
        uuid::Uuid::nil(),
        None,
        None,
        None,
        None,
    );
    CallbackManagerMixin::on_retriever_start(
        &handler,
        &Default::default(),
        "",
        uuid::Uuid::nil(),
        None,
        None,
        None,
        None,
    );

    ChainManagerMixin::on_chain_end(&handler, &Default::default(), uuid::Uuid::nil(), None);
    ChainManagerMixin::on_chain_error(
        &handler,
        &std::io::Error::other("e"),
        uuid::Uuid::nil(),
        None,
    );
    ChainManagerMixin::on_agent_action(
        &handler,
        &Default::default(),
        uuid::Uuid::nil(),
        None,
        None,
    );
    ChainManagerMixin::on_agent_finish(
        &handler,
        &Default::default(),
        uuid::Uuid::nil(),
        None,
        None,
    );

    ToolManagerMixin::on_tool_end(&handler, "", uuid::Uuid::nil(), None, None, None, None);
    ToolManagerMixin::on_tool_error(
        &handler,
        &std::io::Error::other("e"),
        uuid::Uuid::nil(),
        None,
    );

    RetrieverManagerMixin::on_retriever_end(&handler, &[], uuid::Uuid::nil(), None);
    RetrieverManagerMixin::on_retriever_error(
        &handler,
        &std::io::Error::other("e"),
        uuid::Uuid::nil(),
        None,
    );

    RunManagerMixin::on_text(&handler, "", uuid::Uuid::nil(), None, None, "");
    RunManagerMixin::on_retry(&handler, &() as &dyn std::any::Any, uuid::Uuid::nil(), None);
    RunManagerMixin::on_custom_event(
        &handler,
        "",
        &() as &dyn std::any::Any,
        uuid::Uuid::nil(),
        None,
        None,
    );
}

#[test]
fn test_async_handler_has_methods() {
    use agent_chain_core::callbacks::base::AsyncCallbackHandler;

    fn assert_is_async_handler<T: AsyncCallbackHandler>() {}
    assert_is_async_handler::<SnapshotHandler>();
}

#[async_trait::async_trait]
impl agent_chain_core::callbacks::base::AsyncCallbackHandler for SnapshotHandler {}

#[test]
fn test_base_callback_handler_attributes() {
    let handler = SnapshotHandler;

    assert!(!handler.ignore_llm());
    assert!(!handler.ignore_retry());
    assert!(!handler.ignore_chain());
    assert!(!handler.ignore_agent());
    assert!(!handler.ignore_retriever());
    assert!(!handler.ignore_chat_model());
    assert!(!handler.ignore_custom_event());
    assert!(!handler.raise_error());
    assert!(!handler.run_inline());
}

#[test]
fn test_async_callback_handler_attributes() {
    let handler = SnapshotHandler;

    assert!(!handler.ignore_llm());
    assert!(!handler.ignore_retry());
    assert!(!handler.ignore_chain());
    assert!(!handler.ignore_agent());
    assert!(!handler.ignore_retriever());
    assert!(!handler.ignore_chat_model());
    assert!(!handler.ignore_custom_event());
    assert!(!handler.raise_error());
    assert!(!handler.run_inline());
}
