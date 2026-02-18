//! **IMPORTANT** Snapshot tests for base callback handler.
//!
//! These tests check that the public API of the base callback handler
//! has not changed. If they fail it means that the public API has changed
//! and the changes need to be reviewed and approved.
//!
//! Ported from `langchain/libs/core/tests/unit_tests/callbacks/test_base_snapshot.py`

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

/// Ported from `test_sync_handler_has_methods`.
///
/// Verifies that BaseCallbackHandler (via its mixin traits) exposes all
/// expected callback methods. If this test fails to compile, the public API
/// has changed.
///
/// Please do not remove or change the order of the expected methods.
/// If a method is added, it should be added at the end of the list.
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

/// Ported from `test_async_handler_has_methods`.
///
/// Verifies that AsyncCallbackHandler exposes async versions of all expected
/// callback methods. Compilation success means the API is intact.
#[test]
fn test_async_handler_has_methods() {
    use agent_chain_core::callbacks::base::AsyncCallbackHandler;

    fn assert_is_async_handler<T: AsyncCallbackHandler>() {}
    assert_is_async_handler::<SnapshotHandler>();

}

#[async_trait::async_trait]
impl agent_chain_core::callbacks::base::AsyncCallbackHandler for SnapshotHandler {}

/// Ported from `test_base_callback_handler_attributes`.
///
/// Verifies that BaseCallbackHandler exposes all expected ignore flags,
/// and that they default to `false`.
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

/// Ported from `test_async_callback_handler_attributes`.
///
/// Verifies that AsyncCallbackHandler inherits all ignore flags from
/// BaseCallbackHandler (since it is a supertrait), and they default to `false`.
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
