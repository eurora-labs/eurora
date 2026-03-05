use agent_chain_core::callbacks::BaseCallbackHandler;

#[derive(Debug)]
struct SnapshotHandler;

impl BaseCallbackHandler for SnapshotHandler {
    fn name(&self) -> &str {
        "SnapshotHandler"
    }
}

#[test]
fn test_sync_handler_has_methods() {
    let handler = SnapshotHandler;

    BaseCallbackHandler::on_llm_new_token(&handler, "", uuid::Uuid::nil(), None, None);
    BaseCallbackHandler::on_llm_end(&handler, &Default::default(), uuid::Uuid::nil(), None);
    BaseCallbackHandler::on_llm_error(
        &handler,
        &std::io::Error::other("e"),
        uuid::Uuid::nil(),
        None,
    );

    BaseCallbackHandler::on_llm_start(
        &handler,
        &Default::default(),
        &[],
        uuid::Uuid::nil(),
        None,
        None,
        None,
    );
    BaseCallbackHandler::on_chat_model_start(
        &handler,
        &Default::default(),
        &[],
        uuid::Uuid::nil(),
        None,
        None,
        None,
        None,
    );
    BaseCallbackHandler::on_chain_start(
        &handler,
        &Default::default(),
        &Default::default(),
        uuid::Uuid::nil(),
        None,
        None,
        None,
        None,
    );
    BaseCallbackHandler::on_tool_start(
        &handler,
        &Default::default(),
        "",
        uuid::Uuid::nil(),
        None,
        None,
        None,
        None,
    );
    BaseCallbackHandler::on_retriever_start(
        &handler,
        &Default::default(),
        "",
        uuid::Uuid::nil(),
        None,
        None,
        None,
        None,
    );

    BaseCallbackHandler::on_chain_end(&handler, &Default::default(), uuid::Uuid::nil(), None);
    BaseCallbackHandler::on_chain_error(
        &handler,
        &std::io::Error::other("e"),
        uuid::Uuid::nil(),
        None,
    );
    BaseCallbackHandler::on_agent_action(
        &handler,
        &Default::default(),
        uuid::Uuid::nil(),
        None,
        None,
    );
    BaseCallbackHandler::on_agent_finish(
        &handler,
        &Default::default(),
        uuid::Uuid::nil(),
        None,
        None,
    );

    BaseCallbackHandler::on_tool_end(&handler, "", uuid::Uuid::nil(), None, None, None, None);
    BaseCallbackHandler::on_tool_error(
        &handler,
        &std::io::Error::other("e"),
        uuid::Uuid::nil(),
        None,
    );

    BaseCallbackHandler::on_retriever_end(&handler, &[], uuid::Uuid::nil(), None);
    BaseCallbackHandler::on_retriever_error(
        &handler,
        &std::io::Error::other("e"),
        uuid::Uuid::nil(),
        None,
    );

    BaseCallbackHandler::on_text(&handler, "", uuid::Uuid::nil(), None, None, "");
    BaseCallbackHandler::on_retry(&handler, &() as &dyn std::any::Any, uuid::Uuid::nil(), None);
    BaseCallbackHandler::on_custom_event(
        &handler,
        "",
        &() as &dyn std::any::Any,
        uuid::Uuid::nil(),
        None,
        None,
    );
}

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
