//! End-to-end runtime tests for the macro-emitted dispatcher.
//!
//! These tests build a real adapter implementation, wrap it in the
//! generated `*Dispatcher`, register it into an `eurora_tools::Catalog`,
//! and fire `IncomingCall`s through to verify the JSON encode/decode,
//! `Origin` matching, and error paths the macro generates.

use std::sync::Arc;

use eurora_tools::{
    AcpOrigin, BrowserOrigin, Catalog, FocusedOrigin, IncomingCall, Origin, ToolError, adapter,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio_util::sync::CancellationToken;

#[derive(Serialize, Deserialize, JsonSchema, Default)]
pub struct Empty {}

#[derive(Serialize, Deserialize, JsonSchema, Debug, PartialEq)]
pub struct Echo {
    pub video_id: String,
    pub tab_id: i64,
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct Pair {
    pub a: i64,
    pub b: i64,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, PartialEq)]
pub struct Sum {
    pub total: i64,
}

/// Tools for the YouTube tab in the user's browser.
#[adapter(namespace = "browser::youtube", version = 1)]
pub trait YoutubeAdapter: Send + Sync {
    /// Mirror the routing target back to the caller as proof we
    /// destructured the right `Origin` variant.
    #[tool(
        timeout_ms = 1_000,
        source = "bridge(browser)",
        requires_context = "youtube::watch_page"
    )]
    async fn echo_target(&self, target: &BrowserOrigin, args: Empty) -> Result<Echo, ToolError>;
}

/// Local arithmetic tools — no target parameter.
#[adapter(namespace = "client::math")]
pub trait MathAdapter: Send + Sync {
    /// Add two integers.
    #[tool(timeout_ms = 100, source = "client_local")]
    async fn add(&self, args: Pair) -> Result<Sum, ToolError>;
}

struct YoutubeStub;

impl YoutubeAdapter for YoutubeStub {
    async fn echo_target(&self, target: &BrowserOrigin, _args: Empty) -> Result<Echo, ToolError> {
        Ok(Echo {
            video_id: target.page_url.clone(),
            tab_id: target.tab_id,
        })
    }
}

struct MathStub;

impl MathAdapter for MathStub {
    async fn add(&self, args: Pair) -> Result<Sum, ToolError> {
        Ok(Sum {
            total: args.a + args.b,
        })
    }
}

fn browser_origin() -> Origin {
    Origin::Browser(BrowserOrigin {
        process_id: 1,
        tab_id: 42,
        window_id: None,
        page_url: "https://youtube.com/watch?v=abc".into(),
    })
}

fn focused_origin() -> Origin {
    Origin::Focused(FocusedOrigin {
        process_id: 1,
        window_id: Some(7),
        app_name: "Code".into(),
    })
}

fn _acp_origin() -> Origin {
    Origin::Acp(AcpOrigin {
        process_id: 1,
        session_id: "session-1".into(),
    })
}

fn call(name: &'static str, args: serde_json::Value, origin: Origin) -> IncomingCall {
    IncomingCall {
        call_id: 1,
        descriptor_name: name,
        arguments: args,
        origin: std::sync::Arc::new(origin),
        cancel: CancellationToken::new(),
    }
}

#[tokio::test]
async fn descriptors_table_matches_declared_methods() {
    assert_eq!(YOUTUBE_DESCRIPTORS.len(), 1);
    let d = &YOUTUBE_DESCRIPTORS[0];
    assert_eq!(d.name, "browser::youtube::echo_target");
    assert_eq!(d.timeout.as_millis(), 1_000);
    assert_eq!(d.required_contexts, &["youtube::watch_page"]);
    assert!(!d.requires_user_approval);
}

#[tokio::test]
async fn dispatcher_descriptors_returns_static_table() {
    let dispatcher = YoutubeDispatcher::new(YoutubeStub);
    let descs = eurora_tools::Dispatcher::descriptors(&dispatcher);
    assert_eq!(descs.len(), 1);
    assert_eq!(descs[0].name, "browser::youtube::echo_target");
}

#[tokio::test]
async fn dispatch_decodes_args_and_encodes_result() {
    let dispatcher = YoutubeDispatcher::new(YoutubeStub);
    let result = eurora_tools::Dispatcher::dispatch(
        &dispatcher,
        call("browser::youtube::echo_target", json!({}), browser_origin()),
    )
    .await
    .expect("call succeeds");
    assert_eq!(result["tab_id"], 42);
    assert_eq!(result["video_id"], "https://youtube.com/watch?v=abc");
}

#[tokio::test]
async fn dispatch_returns_origin_mismatch_for_wrong_variant() {
    let dispatcher = YoutubeDispatcher::new(YoutubeStub);
    let err = eurora_tools::Dispatcher::dispatch(
        &dispatcher,
        call("browser::youtube::echo_target", json!({}), focused_origin()),
    )
    .await
    .expect_err("wrong origin must fail");
    match err {
        ToolError::OriginMismatch {
            tool,
            expected,
            got,
        } => {
            assert_eq!(tool, "browser::youtube::echo_target");
            assert_eq!(expected, "Browser");
            assert_eq!(got, "Focused");
        }
        other => panic!("expected OriginMismatch, got {other:?}"),
    }
}

#[tokio::test]
async fn dispatch_unknown_name_returns_404() {
    let dispatcher = YoutubeDispatcher::new(YoutubeStub);
    let err = eurora_tools::Dispatcher::dispatch(
        &dispatcher,
        call(
            "browser::youtube::does_not_exist",
            json!({}),
            browser_origin(),
        ),
    )
    .await
    .expect_err("unknown tool name must fail");
    match err {
        ToolError::Remote { code, message, .. } => {
            assert_eq!(code, 404);
            assert!(message.contains("does_not_exist"));
        }
        other => panic!("expected Remote 404, got {other:?}"),
    }
}

#[tokio::test]
async fn client_local_dispatcher_runs_without_target() {
    let dispatcher = MathDispatcher::new(MathStub);
    let result = eurora_tools::Dispatcher::dispatch(
        &dispatcher,
        // Any origin works — `client_local` skips the variant check.
        call(
            "client::math::add",
            json!({"a": 2, "b": 3}),
            focused_origin(),
        ),
    )
    .await
    .expect("local call succeeds");
    assert_eq!(result["total"], 5);
}

#[tokio::test]
async fn dispatch_decode_failure_returns_decode_error() {
    let dispatcher = MathDispatcher::new(MathStub);
    let err = eurora_tools::Dispatcher::dispatch(
        &dispatcher,
        call(
            "client::math::add",
            json!({"a": "not a number"}),
            focused_origin(),
        ),
    )
    .await
    .expect_err("bad args must fail");
    match err {
        ToolError::Decode { source, .. } => assert!(source.is_some()),
        other => panic!("expected Decode, got {other:?}"),
    }
}

#[tokio::test]
async fn catalog_round_trip_routes_to_emitted_dispatcher() {
    let catalog = Catalog::new();
    catalog.register(Arc::new(YoutubeDispatcher::new(YoutubeStub)));
    catalog.register(Arc::new(MathDispatcher::new(MathStub)));

    assert_eq!(catalog.len(), 2);
    let dispatcher = catalog
        .dispatcher_for("browser::youtube::echo_target")
        .expect("registered");
    let result = dispatcher
        .dispatch(call(
            "browser::youtube::echo_target",
            json!({}),
            browser_origin(),
        ))
        .await
        .expect("call succeeds");
    assert_eq!(result["tab_id"], 42);
}
