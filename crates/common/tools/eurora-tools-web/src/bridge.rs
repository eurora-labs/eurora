//! Bridge-backed [`WebAdapter`](crate::adapter::WebAdapter) implementation.
//!
//! Each method on [`WebBridgeImpl`] translates one [`WebAdapter`] call
//! into a [`euro_bridge::BridgeService::send_request`] round trip,
//! targeting the browser process identified by the frozen
//! [`BrowserOrigin`] from the per-turn snapshot. The browser extension
//! satisfies the request via the matching `WEB_*` action constant and
//! the response payload is decoded into the typed return.
//!
//! Payload framing, response decoding, and transport-error mapping all
//! flow through the shared [`eurora_tools::bridge`] helpers so every
//! adapter — YouTube today, web here, X / Google Docs / … later —
//! speaks the same wire shape and surfaces identical [`ToolError`]
//! semantics. The browser extension reads `tab_id` off the top level of
//! every payload via `tab-rpc.ts::parseTabId`; arg fields ride along
//! alongside it.
//!
//! This module is gated behind the crate's `bridge` feature so non-
//! desktop consumers (the agent loop, codegen utilities) don't pull
//! [`euro_bridge`] and its transitive dependencies.

use euro_bridge::BridgeService;
use eurora_tools::bridge::{build_payload, decode_payload, map_bridge_err};
use eurora_tools::{BrowserOrigin, Empty, ToolError};
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::adapter::WebAdapter;
use crate::types::{
    AccessibilityTree, FormInputsList, GetAccessibilityTreeArgs, InsertTextArgs, InsertTextResult,
    LinksList, ListFormInputsArgs, ListLinksArgs, PageMetadata, QuerySelectorArgs,
    QuerySelectorResult, ReadabilityArticle, SelectedText,
};

/// Bridge action emitted for `browser::web::get_page_metadata`.
pub const WEB_GET_PAGE_METADATA: &str = "WEB_GET_PAGE_METADATA";
/// Bridge action emitted for `browser::web::get_accessibility_tree`.
pub const WEB_GET_ACCESSIBILITY_TREE: &str = "WEB_GET_ACCESSIBILITY_TREE";
/// Bridge action emitted for `browser::web::get_readability_article`.
pub const WEB_GET_READABILITY_ARTICLE: &str = "WEB_GET_READABILITY_ARTICLE";
/// Bridge action emitted for `browser::web::get_selected_text`.
pub const WEB_GET_SELECTED_TEXT: &str = "WEB_GET_SELECTED_TEXT";
/// Bridge action emitted for `browser::web::query_selector`.
pub const WEB_QUERY_SELECTOR: &str = "WEB_QUERY_SELECTOR";
/// Bridge action emitted for `browser::web::list_links`.
pub const WEB_LIST_LINKS: &str = "WEB_LIST_LINKS";
/// Bridge action emitted for `browser::web::list_form_inputs`.
pub const WEB_LIST_FORM_INPUTS: &str = "WEB_LIST_FORM_INPUTS";
/// Bridge action emitted for `browser::web::insert_text`.
pub const WEB_INSERT_TEXT: &str = "WEB_INSERT_TEXT";

const METADATA_TOOL: &str = "browser::web::get_page_metadata";
const AX_TREE_TOOL: &str = "browser::web::get_accessibility_tree";
const READABILITY_TOOL: &str = "browser::web::get_readability_article";
const SELECTION_TOOL: &str = "browser::web::get_selected_text";
const QUERY_TOOL: &str = "browser::web::query_selector";
const LINKS_TOOL: &str = "browser::web::list_links";
const FORM_INPUTS_TOOL: &str = "browser::web::list_form_inputs";
const INSERT_TEXT_TOOL: &str = "browser::web::insert_text";

/// Wrapper that fulfils every [`WebAdapter`] method by hitting the
/// browser process registered with [`BridgeService`].
///
/// Constructed once per process from
/// [`BridgeService::get_or_init`](euro_bridge::BridgeService::get_or_init).
/// The `'static` reference matches that initializer's return type so the
/// struct is cheaply `Clone`-able and trivially sharable across threads.
pub struct WebBridgeImpl {
    bridge: &'static BridgeService,
}

impl WebBridgeImpl {
    pub fn new(bridge: &'static BridgeService) -> Self {
        Self { bridge }
    }

    async fn call_action<A, T>(
        &self,
        target: &BrowserOrigin,
        action: &'static str,
        tool: &'static str,
        args: &A,
    ) -> Result<T, ToolError>
    where
        A: Serialize + ?Sized,
        T: DeserializeOwned,
    {
        let payload = build_payload(target, args)?;
        let response = self
            .bridge
            .send_request(target.process_id, action, Some(payload))
            .await
            .map_err(|err| map_bridge_err(tool, err))?;
        decode_payload(tool, response.payload)
    }
}

impl WebAdapter for WebBridgeImpl {
    async fn get_page_metadata(
        &self,
        target: &BrowserOrigin,
        args: Empty,
    ) -> Result<PageMetadata, ToolError> {
        self.call_action(target, WEB_GET_PAGE_METADATA, METADATA_TOOL, &args)
            .await
    }

    async fn get_accessibility_tree(
        &self,
        target: &BrowserOrigin,
        args: GetAccessibilityTreeArgs,
    ) -> Result<AccessibilityTree, ToolError> {
        self.call_action(target, WEB_GET_ACCESSIBILITY_TREE, AX_TREE_TOOL, &args)
            .await
    }

    async fn get_readability_article(
        &self,
        target: &BrowserOrigin,
        args: Empty,
    ) -> Result<ReadabilityArticle, ToolError> {
        self.call_action(target, WEB_GET_READABILITY_ARTICLE, READABILITY_TOOL, &args)
            .await
    }

    async fn get_selected_text(
        &self,
        target: &BrowserOrigin,
        args: Empty,
    ) -> Result<SelectedText, ToolError> {
        self.call_action(target, WEB_GET_SELECTED_TEXT, SELECTION_TOOL, &args)
            .await
    }

    async fn query_selector(
        &self,
        target: &BrowserOrigin,
        args: QuerySelectorArgs,
    ) -> Result<QuerySelectorResult, ToolError> {
        self.call_action(target, WEB_QUERY_SELECTOR, QUERY_TOOL, &args)
            .await
    }

    async fn list_links(
        &self,
        target: &BrowserOrigin,
        args: ListLinksArgs,
    ) -> Result<LinksList, ToolError> {
        self.call_action(target, WEB_LIST_LINKS, LINKS_TOOL, &args)
            .await
    }

    async fn list_form_inputs(
        &self,
        target: &BrowserOrigin,
        args: ListFormInputsArgs,
    ) -> Result<FormInputsList, ToolError> {
        self.call_action(target, WEB_LIST_FORM_INPUTS, FORM_INPUTS_TOOL, &args)
            .await
    }

    async fn insert_text(
        &self,
        target: &BrowserOrigin,
        args: InsertTextArgs,
    ) -> Result<InsertTextResult, ToolError> {
        self.call_action(target, WEB_INSERT_TEXT, INSERT_TEXT_TOOL, &args)
            .await
    }
}
