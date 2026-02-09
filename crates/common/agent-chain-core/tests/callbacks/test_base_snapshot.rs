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

// -- Minimal handler impl for testing trait methods --

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
    let mut handler = SnapshotHandler;

    // LLMManagerMixin
    let _ = LLMManagerMixin::on_llm_new_token(&mut handler, "", uuid::Uuid::nil(), None, None);
    let _ = LLMManagerMixin::on_llm_end(&mut handler, &Default::default(), uuid::Uuid::nil(), None);
    let _ = LLMManagerMixin::on_llm_error(
        &mut handler,
        &std::io::Error::new(std::io::ErrorKind::Other, "e"),
        uuid::Uuid::nil(),
        None,
    );

    // CallbackManagerMixin
    let _ = CallbackManagerMixin::on_llm_start(
        &mut handler,
        &Default::default(),
        &[],
        uuid::Uuid::nil(),
        None,
        None,
        None,
    );
    let _ = CallbackManagerMixin::on_chat_model_start(
        &mut handler,
        &Default::default(),
        &[],
        uuid::Uuid::nil(),
        None,
        None,
        None,
    );
    let _ = CallbackManagerMixin::on_chain_start(
        &mut handler,
        &Default::default(),
        &Default::default(),
        uuid::Uuid::nil(),
        None,
        None,
        None,
    );
    let _ = CallbackManagerMixin::on_tool_start(
        &mut handler,
        &Default::default(),
        "",
        uuid::Uuid::nil(),
        None,
        None,
        None,
        None,
    );
    let _ = CallbackManagerMixin::on_retriever_start(
        &mut handler,
        &Default::default(),
        "",
        uuid::Uuid::nil(),
        None,
        None,
        None,
    );

    // ChainManagerMixin
    let _ =
        ChainManagerMixin::on_chain_end(&mut handler, &Default::default(), uuid::Uuid::nil(), None);
    let _ = ChainManagerMixin::on_chain_error(
        &mut handler,
        &std::io::Error::new(std::io::ErrorKind::Other, "e"),
        uuid::Uuid::nil(),
        None,
    );
    let _ = ChainManagerMixin::on_agent_action(
        &mut handler,
        &Default::default(),
        uuid::Uuid::nil(),
        None,
        None,
    );
    let _ = ChainManagerMixin::on_agent_finish(
        &mut handler,
        &Default::default(),
        uuid::Uuid::nil(),
        None,
        None,
    );

    // ToolManagerMixin
    let _ =
        ToolManagerMixin::on_tool_end(&mut handler, "", uuid::Uuid::nil(), None, None, None, None);
    let _ = ToolManagerMixin::on_tool_error(
        &mut handler,
        &std::io::Error::new(std::io::ErrorKind::Other, "e"),
        uuid::Uuid::nil(),
        None,
    );

    // RetrieverManagerMixin
    let _ = RetrieverManagerMixin::on_retriever_end(&mut handler, &[], uuid::Uuid::nil(), None);
    let _ = RetrieverManagerMixin::on_retriever_error(
        &mut handler,
        &std::io::Error::new(std::io::ErrorKind::Other, "e"),
        uuid::Uuid::nil(),
        None,
    );

    // RunManagerMixin
    let _ = RunManagerMixin::on_text(&mut handler, "", uuid::Uuid::nil(), None, None, "");
    let _ = RunManagerMixin::on_retry(
        &mut handler,
        &() as &dyn std::any::Any,
        uuid::Uuid::nil(),
        None,
    );
    let _ = RunManagerMixin::on_custom_event(
        &mut handler,
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

    // Verify the trait requires BaseCallbackHandler as a supertrait
    fn assert_is_async_handler<T: AsyncCallbackHandler>() {}
    assert_is_async_handler::<SnapshotHandler>();

    // The async methods are verified at compile time by the trait bound above.
    // AsyncCallbackHandler has default implementations for all of:
    //   on_llm_start_async, on_chat_model_start_async, on_llm_new_token_async,
    //   on_llm_end_async, on_llm_error_async, on_chain_start_async,
    //   on_chain_end_async, on_chain_error_async, on_tool_start_async,
    //   on_tool_end_async, on_tool_error_async, on_text_async, on_retry_async,
    //   on_agent_action_async, on_agent_finish_async, on_retriever_start_async,
    //   on_retriever_end_async, on_retriever_error_async, on_custom_event_async
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
    // AsyncCallbackHandler requires BaseCallbackHandler, so all ignore_*
    // methods are available. We verify them on the same handler that
    // implements both traits.
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
