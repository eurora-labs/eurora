//! End-to-end runtime tests for the macro-emitted web dispatcher.
//!
//! Builds a stub `WebAdapter`, wraps it in `WebDispatcher`, registers
//! it in a `Catalog`, then fires `IncomingCall`s through to verify JSON
//! encode/decode, `Origin` matching, the descriptor table itself, and
//! the error paths the macro generates (origin mismatch, unknown name,
//! decode failure).

use std::collections::HashMap;
use std::sync::Arc;

use eurora_tools::{
    AcpOrigin, BrowserOrigin, Catalog, Dispatcher, Empty, FocusedOrigin, IncomingCall, Origin,
    ToolError,
};
use eurora_tools_web::{
    AccessibilityTree, AxNode, BoundingBox, DomNode, FormInput, FormInputKind, FormInputsList,
    GetAccessibilityTreeArgs, InsertTextArgs, InsertTextResult, Link, LinksList,
    ListFormInputsArgs, ListLinksArgs, PageMetadata, QuerySelectorArgs, QuerySelectorInclude,
    QuerySelectorResult, ReadabilityArticle, SelectedText, ViewportMetrics, WEB_DESCRIPTORS,
    WebAdapter, WebDispatcher,
};
use serde_json::json;
use tokio_util::sync::CancellationToken;

const GET_PAGE_METADATA_TOOL: &str = "browser::web::get_page_metadata";
const GET_ACCESSIBILITY_TREE_TOOL: &str = "browser::web::get_accessibility_tree";
const GET_READABILITY_ARTICLE_TOOL: &str = "browser::web::get_readability_article";
const GET_SELECTED_TEXT_TOOL: &str = "browser::web::get_selected_text";
const QUERY_SELECTOR_TOOL: &str = "browser::web::query_selector";
const LIST_LINKS_TOOL: &str = "browser::web::list_links";
const LIST_FORM_INPUTS_TOOL: &str = "browser::web::list_form_inputs";
const INSERT_TEXT_TOOL: &str = "browser::web::insert_text";

const ALL_TOOLS: [&str; 8] = [
    GET_PAGE_METADATA_TOOL,
    GET_ACCESSIBILITY_TREE_TOOL,
    GET_READABILITY_ARTICLE_TOOL,
    GET_SELECTED_TEXT_TOOL,
    QUERY_SELECTOR_TOOL,
    LIST_LINKS_TOOL,
    LIST_FORM_INPUTS_TOOL,
    INSERT_TEXT_TOOL,
];

const PAGE_URL: &str = "https://example.com/path";

/// In-memory adapter that returns deterministic, target-aware payloads
/// so the dispatcher's arg-decode / origin-match / re-encode wiring is
/// observably exercised end-to-end.
struct WebStub;

impl WebAdapter for WebStub {
    async fn get_page_metadata(
        &self,
        target: &BrowserOrigin,
        _args: Empty,
    ) -> Result<PageMetadata, ToolError> {
        Ok(PageMetadata {
            url: target.page_url.clone(),
            title: format!("tab-{}", target.tab_id),
            host: "example.com".into(),
            language: Some("en".into()),
            charset: Some("utf-8".into()),
            description: None,
            og: HashMap::new(),
            viewport: ViewportMetrics {
                scroll_x: 0.0,
                scroll_y: 0.0,
                inner_width: 1280.0,
                inner_height: 800.0,
                document_height: 4200.0,
            },
        })
    }

    async fn get_accessibility_tree(
        &self,
        _target: &BrowserOrigin,
        args: GetAccessibilityTreeArgs,
    ) -> Result<AccessibilityTree, ToolError> {
        Ok(AccessibilityTree {
            root: AxNode {
                role: "main".into(),
                name: args.root_selector.clone(),
                value: None,
                description: None,
                selector_path: args.root_selector,
                children: vec![],
            },
            node_count: 1,
            truncated: false,
        })
    }

    async fn get_readability_article(
        &self,
        target: &BrowserOrigin,
        _args: Empty,
    ) -> Result<ReadabilityArticle, ToolError> {
        Ok(ReadabilityArticle {
            title: Some(format!("article-{}", target.tab_id)),
            byline: None,
            site_name: None,
            language: None,
            excerpt: None,
            content_html: "<p>body</p>".into(),
            text_content: "body".into(),
            length: 4,
        })
    }

    async fn get_selected_text(
        &self,
        _target: &BrowserOrigin,
        _args: Empty,
    ) -> Result<SelectedText, ToolError> {
        Ok(SelectedText {
            text: "highlighted".into(),
            anchor_xpath: None,
            focus_xpath: None,
        })
    }

    async fn query_selector(
        &self,
        _target: &BrowserOrigin,
        args: QuerySelectorArgs,
    ) -> Result<QuerySelectorResult, ToolError> {
        Ok(QuerySelectorResult {
            matches: vec![DomNode {
                selector_path: args.selector.clone(),
                text: args
                    .include
                    .contains(&QuerySelectorInclude::Text)
                    .then(|| "match".into()),
                html: args
                    .include
                    .contains(&QuerySelectorInclude::Html)
                    .then(|| "<p/>".into()),
                attributes: args
                    .include
                    .contains(&QuerySelectorInclude::Attributes)
                    .then(HashMap::new),
                bounds: args
                    .include
                    .contains(&QuerySelectorInclude::Bounds)
                    .then_some(BoundingBox {
                        x: 0.0,
                        y: 0.0,
                        width: 0.0,
                        height: 0.0,
                    }),
            }],
            total_match_count: 1,
            truncated: false,
        })
    }

    async fn list_links(
        &self,
        _target: &BrowserOrigin,
        _args: ListLinksArgs,
    ) -> Result<LinksList, ToolError> {
        Ok(LinksList {
            links: vec![Link {
                url: "https://example.com/about".into(),
                label: Some("About".into()),
                role: "link".into(),
                selector_path: "a:nth-of-type(1)".into(),
            }],
            total: 1,
        })
    }

    async fn list_form_inputs(
        &self,
        _target: &BrowserOrigin,
        _args: ListFormInputsArgs,
    ) -> Result<FormInputsList, ToolError> {
        Ok(FormInputsList {
            inputs: vec![FormInput {
                field_id: "#q".into(),
                label: Some("Search".into()),
                kind: FormInputKind::Search,
                value: String::new(),
                placeholder: Some("Search…".into()),
                required: false,
            }],
            total: 1,
        })
    }

    async fn insert_text(
        &self,
        _target: &BrowserOrigin,
        args: InsertTextArgs,
    ) -> Result<InsertTextResult, ToolError> {
        let new_value = if args.replace {
            args.text.clone()
        } else {
            format!("prefix {}", args.text)
        };
        Ok(InsertTextResult {
            field_id: args.field_id,
            previous_value: "prefix".into(),
            new_value,
        })
    }
}

fn browser_origin() -> Origin {
    Origin::Browser(BrowserOrigin {
        process_id: 1,
        tab_id: 42,
        window_id: Some("win-1".into()),
        page_url: PAGE_URL.into(),
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
        origin: Arc::new(origin),
        cancel: CancellationToken::new(),
    }
}

#[test]
fn descriptor_table_matches_declared_methods() {
    assert_eq!(WEB_DESCRIPTORS.len(), 8);
    let names: Vec<_> = WEB_DESCRIPTORS.iter().map(|d| d.name).collect();
    assert_eq!(names, ALL_TOOLS);

    let timeouts_ms: Vec<_> = WEB_DESCRIPTORS
        .iter()
        .map(|d| u32::try_from(d.timeout.as_millis()).expect("fits in u32"))
        .collect();
    assert_eq!(
        timeouts_ms,
        [2_000, 5_000, 5_000, 1_000, 5_000, 3_000, 3_000, 2_000]
    );

    for descriptor in WEB_DESCRIPTORS.iter() {
        assert_eq!(descriptor.required_contexts, &["web::page"]);
    }
}

#[test]
fn requires_user_approval_is_set_only_on_insert_text() {
    for descriptor in WEB_DESCRIPTORS.iter() {
        let expected = descriptor.name == INSERT_TEXT_TOOL;
        assert_eq!(
            descriptor.requires_user_approval, expected,
            "tool `{}` requires_user_approval should be {}",
            descriptor.name, expected,
        );
    }
}

#[tokio::test]
async fn dispatcher_descriptors_returns_static_table() {
    let dispatcher = WebDispatcher::new(WebStub);
    let descs = Dispatcher::descriptors(&dispatcher);
    assert_eq!(descs.len(), 8);
    assert!(descs.iter().any(|d| d.name == GET_PAGE_METADATA_TOOL));
}

#[tokio::test]
async fn dispatch_get_page_metadata_round_trips() {
    let dispatcher = WebDispatcher::new(WebStub);
    let result = dispatcher
        .dispatch(call(GET_PAGE_METADATA_TOOL, json!({}), browser_origin()))
        .await
        .expect("call succeeds");

    let decoded: PageMetadata = serde_json::from_value(result).expect("decode PageMetadata");
    assert_eq!(decoded.url, PAGE_URL);
    assert_eq!(decoded.title, "tab-42");
    assert_eq!(decoded.language.as_deref(), Some("en"));
}

#[tokio::test]
async fn dispatch_get_accessibility_tree_propagates_args() {
    let dispatcher = WebDispatcher::new(WebStub);
    let result = dispatcher
        .dispatch(call(
            GET_ACCESSIBILITY_TREE_TOOL,
            json!({ "root_selector": "main", "max_depth": 4 }),
            browser_origin(),
        ))
        .await
        .expect("call succeeds");

    let decoded: AccessibilityTree =
        serde_json::from_value(result).expect("decode AccessibilityTree");
    assert_eq!(decoded.root.role, "main");
    assert_eq!(decoded.root.selector_path.as_deref(), Some("main"));
}

#[tokio::test]
async fn dispatch_get_readability_article_round_trips() {
    let dispatcher = WebDispatcher::new(WebStub);
    let result = dispatcher
        .dispatch(call(
            GET_READABILITY_ARTICLE_TOOL,
            json!({}),
            browser_origin(),
        ))
        .await
        .expect("call succeeds");

    let decoded: ReadabilityArticle =
        serde_json::from_value(result).expect("decode ReadabilityArticle");
    assert_eq!(decoded.title.as_deref(), Some("article-42"));
    assert_eq!(decoded.text_content, "body");
}

#[tokio::test]
async fn dispatch_get_selected_text_round_trips() {
    let dispatcher = WebDispatcher::new(WebStub);
    let result = dispatcher
        .dispatch(call(GET_SELECTED_TEXT_TOOL, json!({}), browser_origin()))
        .await
        .expect("call succeeds");

    let decoded: SelectedText = serde_json::from_value(result).expect("decode SelectedText");
    assert_eq!(decoded.text, "highlighted");
}

#[tokio::test]
async fn dispatch_query_selector_respects_include_flags() {
    let dispatcher = WebDispatcher::new(WebStub);
    let result = dispatcher
        .dispatch(call(
            QUERY_SELECTOR_TOOL,
            json!({ "selector": "a", "include": ["text", "bounds"] }),
            browser_origin(),
        ))
        .await
        .expect("call succeeds");

    let decoded: QuerySelectorResult =
        serde_json::from_value(result).expect("decode QuerySelectorResult");
    let node = decoded.matches.first().expect("one match");
    assert_eq!(node.selector_path, "a");
    assert_eq!(node.text.as_deref(), Some("match"));
    assert!(node.bounds.is_some());
    assert!(node.html.is_none());
    assert!(node.attributes.is_none());
}

#[tokio::test]
async fn dispatch_query_selector_applies_default_limit_when_omitted() {
    let dispatcher = WebDispatcher::new(WebStub);
    // Only `selector` is provided; `limit` should default to 50 inside
    // the dispatcher's arg-decode step, and `include` to an empty Vec.
    let result = dispatcher
        .dispatch(call(
            QUERY_SELECTOR_TOOL,
            json!({ "selector": "h1" }),
            browser_origin(),
        ))
        .await
        .expect("call succeeds");

    let decoded: QuerySelectorResult =
        serde_json::from_value(result).expect("decode QuerySelectorResult");
    let node = decoded.matches.first().expect("one match");
    assert!(node.text.is_none(), "no facet was requested");
}

#[tokio::test]
async fn dispatch_list_links_round_trips() {
    let dispatcher = WebDispatcher::new(WebStub);
    let result = dispatcher
        .dispatch(call(LIST_LINKS_TOOL, json!({}), browser_origin()))
        .await
        .expect("call succeeds");

    let decoded: LinksList = serde_json::from_value(result).expect("decode LinksList");
    assert_eq!(decoded.total, 1);
    assert_eq!(decoded.links[0].label.as_deref(), Some("About"));
}

#[tokio::test]
async fn dispatch_list_form_inputs_round_trips() {
    let dispatcher = WebDispatcher::new(WebStub);
    let result = dispatcher
        .dispatch(call(LIST_FORM_INPUTS_TOOL, json!({}), browser_origin()))
        .await
        .expect("call succeeds");

    let decoded: FormInputsList = serde_json::from_value(result).expect("decode FormInputsList");
    assert_eq!(decoded.total, 1);
    assert_eq!(decoded.inputs[0].kind, FormInputKind::Search);
}

#[tokio::test]
async fn dispatch_insert_text_returns_appended_value_by_default() {
    let dispatcher = WebDispatcher::new(WebStub);
    let result = dispatcher
        .dispatch(call(
            INSERT_TEXT_TOOL,
            json!({ "field_id": "#q", "text": "hello" }),
            browser_origin(),
        ))
        .await
        .expect("call succeeds");

    let decoded: InsertTextResult =
        serde_json::from_value(result).expect("decode InsertTextResult");
    assert_eq!(decoded.field_id, "#q");
    assert_eq!(decoded.previous_value, "prefix");
    assert_eq!(decoded.new_value, "prefix hello");
}

#[tokio::test]
async fn dispatch_insert_text_replaces_value_when_replace_true() {
    let dispatcher = WebDispatcher::new(WebStub);
    let result = dispatcher
        .dispatch(call(
            INSERT_TEXT_TOOL,
            json!({ "field_id": "#q", "text": "hello", "replace": true }),
            browser_origin(),
        ))
        .await
        .expect("call succeeds");

    let decoded: InsertTextResult =
        serde_json::from_value(result).expect("decode InsertTextResult");
    assert_eq!(decoded.new_value, "hello");
}

#[tokio::test]
async fn dispatch_returns_origin_mismatch_for_focused_origin() {
    let dispatcher = WebDispatcher::new(WebStub);
    let err = dispatcher
        .dispatch(call(GET_PAGE_METADATA_TOOL, json!({}), focused_origin()))
        .await
        .expect_err("wrong origin must fail");

    match err {
        ToolError::OriginMismatch {
            tool,
            expected,
            got,
        } => {
            assert_eq!(tool, GET_PAGE_METADATA_TOOL);
            assert_eq!(expected, "Browser");
            assert_eq!(got, "Focused");
        }
        other => panic!("expected OriginMismatch, got {other:?}"),
    }
}

#[tokio::test]
async fn dispatch_returns_origin_mismatch_for_acp_origin() {
    let dispatcher = WebDispatcher::new(WebStub);
    let err = dispatcher
        .dispatch(call(
            INSERT_TEXT_TOOL,
            json!({ "field_id": "#q", "text": "x" }),
            acp_origin(),
        ))
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
    let dispatcher = WebDispatcher::new(WebStub);
    let err = dispatcher
        .dispatch(call(
            "browser::web::does_not_exist",
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
    let dispatcher = WebDispatcher::new(WebStub);
    // `QuerySelectorArgs` cannot deserialize from a JSON string — exercises
    // the macro-generated decode arm for a non-`Empty` arg type.
    let err = dispatcher
        .dispatch(call(
            QUERY_SELECTOR_TOOL,
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
    catalog.register(Arc::new(WebDispatcher::new(WebStub)));

    assert_eq!(catalog.len(), 8);
    for name in ALL_TOOLS {
        let dispatcher = catalog
            .dispatcher_for(name)
            .unwrap_or_else(|| panic!("descriptor `{name}` should be registered"));
        let args = match name {
            QUERY_SELECTOR_TOOL => json!({ "selector": "h1" }),
            INSERT_TEXT_TOOL => json!({ "field_id": "#q", "text": "x" }),
            _ => json!({}),
        };
        let result = dispatcher
            .dispatch(call(name, args, browser_origin()))
            .await
            .unwrap_or_else(|err| panic!("call to `{name}` should succeed, got {err:?}"));
        assert!(result.is_object(), "tool `{name}` returns a JSON object");
    }
}
