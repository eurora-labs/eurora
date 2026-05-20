//! End-to-end runtime tests for the macro-emitted YouTube dispatcher.
//!
//! Builds a stub `YoutubeAdapter`, wraps it in `YoutubeDispatcher`,
//! registers it in a `Catalog`, then fires `IncomingCall`s through to
//! verify JSON encode/decode, `Origin` matching, and the error paths
//! the macro generates (origin mismatch, unknown name, decode failure).

use std::sync::Arc;

use eurora_tools::{
    AcpOrigin, BrowserOrigin, Catalog, Dispatcher, Empty, FocusedOrigin, IncomingCall, Origin,
    ToolError,
};
use eurora_tools_youtube::{
    CapturedFrame, CurrentTimestamp, Transcript, TranscriptEntry, YOUTUBE_DESCRIPTORS,
    YoutubeAdapter, YoutubeDispatcher,
};
use serde_json::json;
use tokio_util::sync::CancellationToken;

const TIMESTAMP_TOOL: &str = "browser::youtube::get_current_timestamp";
const TRANSCRIPT_TOOL: &str = "browser::youtube::get_transcript";
const FRAME_TOOL: &str = "browser::youtube::get_current_frame";

struct YoutubeStub;

impl YoutubeAdapter for YoutubeStub {
    async fn get_current_timestamp(
        &self,
        target: &BrowserOrigin,
        _args: Empty,
    ) -> Result<CurrentTimestamp, ToolError> {
        Ok(CurrentTimestamp {
            video_id: format!("tab-{}", target.tab_id),
            current_time: 12.5,
            duration: 240.0,
            playing: true,
        })
    }

    async fn get_transcript(
        &self,
        target: &BrowserOrigin,
        _args: Empty,
    ) -> Result<Transcript, ToolError> {
        Ok(Transcript {
            video_id: format!("tab-{}", target.tab_id),
            language: "en-US".into(),
            entries: vec![TranscriptEntry {
                start: 0.0,
                duration: 1.0,
                text: "hello world".into(),
            }],
        })
    }

    async fn get_current_frame(
        &self,
        target: &BrowserOrigin,
        _args: Empty,
    ) -> Result<CapturedFrame, ToolError> {
        Ok(CapturedFrame {
            video_id: format!("tab-{}", target.tab_id),
            current_time: 12.5,
            width: 1280,
            height: 720,
            image_base64: "iVBORw0KGgo=".into(),
        })
    }
}

fn browser_origin() -> Origin {
    Origin::Browser(BrowserOrigin {
        process_id: 1,
        tab_id: 42,
        window_id: Some("win-1".into()),
        page_url: "https://www.youtube.com/watch?v=dQw4w9WgXcQ".into(),
    })
}

fn focused_origin() -> Origin {
    Origin::Focused(FocusedOrigin {
        process_id: 1,
        window_id: Some(7),
        app_name: "Code".into(),
    })
}

fn acp_origin() -> Origin {
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

#[test]
fn descriptor_table_matches_declared_methods() {
    assert_eq!(YOUTUBE_DESCRIPTORS.len(), 3);
    let names: Vec<_> = YOUTUBE_DESCRIPTORS.iter().map(|d| d.name).collect();
    assert_eq!(names, [TIMESTAMP_TOOL, TRANSCRIPT_TOOL, FRAME_TOOL]);

    let timeouts_ms: Vec<_> = YOUTUBE_DESCRIPTORS
        .iter()
        .map(|d| u32::try_from(d.timeout.as_millis()).expect("fits in u32"))
        .collect();
    assert_eq!(timeouts_ms, [2_000, 10_000, 5_000]);

    for descriptor in YOUTUBE_DESCRIPTORS.iter() {
        assert_eq!(descriptor.required_contexts, &["youtube::watch_page"]);
        assert!(!descriptor.requires_user_approval);
    }
}

#[tokio::test]
async fn dispatcher_descriptors_returns_static_table() {
    let dispatcher = YoutubeDispatcher::new(YoutubeStub);
    let descs = Dispatcher::descriptors(&dispatcher);
    assert_eq!(descs.len(), 3);
    assert!(descs.iter().any(|d| d.name == TIMESTAMP_TOOL));
}

#[tokio::test]
async fn dispatch_get_current_timestamp_round_trips() {
    let dispatcher = YoutubeDispatcher::new(YoutubeStub);
    let result = dispatcher
        .dispatch(call(TIMESTAMP_TOOL, json!({}), browser_origin()))
        .await
        .expect("call succeeds");

    let decoded: CurrentTimestamp =
        serde_json::from_value(result).expect("decode CurrentTimestamp");
    assert_eq!(
        decoded,
        CurrentTimestamp {
            video_id: "tab-42".into(),
            current_time: 12.5,
            duration: 240.0,
            playing: true,
        }
    );
}

#[tokio::test]
async fn dispatch_get_transcript_round_trips() {
    let dispatcher = YoutubeDispatcher::new(YoutubeStub);
    let result = dispatcher
        .dispatch(call(TRANSCRIPT_TOOL, json!({}), browser_origin()))
        .await
        .expect("call succeeds");

    let decoded: Transcript = serde_json::from_value(result).expect("decode Transcript");
    assert_eq!(decoded.video_id, "tab-42");
    assert_eq!(decoded.language, "en-US");
    assert_eq!(decoded.entries.len(), 1);
    assert_eq!(decoded.entries[0].text, "hello world");
}

#[tokio::test]
async fn dispatch_get_current_frame_round_trips() {
    let dispatcher = YoutubeDispatcher::new(YoutubeStub);
    let result = dispatcher
        .dispatch(call(FRAME_TOOL, json!({}), browser_origin()))
        .await
        .expect("call succeeds");

    let decoded: CapturedFrame = serde_json::from_value(result).expect("decode CapturedFrame");
    assert_eq!(decoded.width, 1280);
    assert_eq!(decoded.height, 720);
    assert_eq!(decoded.image_base64, "iVBORw0KGgo=");
}

#[tokio::test]
async fn dispatch_returns_origin_mismatch_for_focused_origin() {
    let dispatcher = YoutubeDispatcher::new(YoutubeStub);
    let err = dispatcher
        .dispatch(call(TIMESTAMP_TOOL, json!({}), focused_origin()))
        .await
        .expect_err("wrong origin must fail");

    match err {
        ToolError::OriginMismatch {
            tool,
            expected,
            got,
        } => {
            assert_eq!(tool, TIMESTAMP_TOOL);
            assert_eq!(expected, "Browser");
            assert_eq!(got, "Focused");
        }
        other => panic!("expected OriginMismatch, got {other:?}"),
    }
}

#[tokio::test]
async fn dispatch_returns_origin_mismatch_for_acp_origin() {
    let dispatcher = YoutubeDispatcher::new(YoutubeStub);
    let err = dispatcher
        .dispatch(call(TRANSCRIPT_TOOL, json!({}), acp_origin()))
        .await
        .expect_err("wrong origin must fail");

    match err {
        ToolError::OriginMismatch { expected, got, .. } => {
            assert_eq!(expected, "Browser");
            assert_eq!(got, "Acp");
        }
        other => panic!("expected OriginMismatch, got {other:?}"),
    }
}

#[tokio::test]
async fn dispatch_unknown_name_returns_404() {
    let dispatcher = YoutubeDispatcher::new(YoutubeStub);
    let err = dispatcher
        .dispatch(call(
            "browser::youtube::does_not_exist",
            json!({}),
            browser_origin(),
        ))
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
async fn dispatch_decode_failure_returns_decode_error() {
    let dispatcher = YoutubeDispatcher::new(YoutubeStub);
    // `Empty` decodes any JSON object, but a non-object argument is a
    // hard decode failure — exercises the macro-generated decode arm.
    let err = dispatcher
        .dispatch(call(
            TIMESTAMP_TOOL,
            json!("not an object"),
            browser_origin(),
        ))
        .await
        .expect_err("bad args must fail");

    match err {
        ToolError::Decode { source, .. } => assert!(source.is_some()),
        other => panic!("expected Decode, got {other:?}"),
    }
}

#[tokio::test]
async fn catalog_routes_each_descriptor_to_the_dispatcher() {
    let catalog = Catalog::new();
    catalog.register(Arc::new(YoutubeDispatcher::new(YoutubeStub)));

    assert_eq!(catalog.len(), 3);
    for name in [TIMESTAMP_TOOL, TRANSCRIPT_TOOL, FRAME_TOOL] {
        let dispatcher = catalog
            .dispatcher_for(name)
            .unwrap_or_else(|| panic!("descriptor `{name}` should be registered"));
        let result = dispatcher
            .dispatch(call(name, json!({}), browser_origin()))
            .await
            .unwrap_or_else(|err| panic!("call to `{name}` should succeed, got {err:?}"));
        assert!(result.is_object(), "tool `{name}` returns a JSON object");
    }
}
