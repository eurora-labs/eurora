//! Bridge-backed [`WebAdapter`] implementation.
//!
//! Each method on [`WebBridgeImpl`] translates one [`WebAdapter`] call
//! into a single
//! [`BridgeClient::call_action`](crate::BridgeClient::call_action)
//! round trip, targeting the browser process identified by the frozen
//! [`BrowserOrigin`] from the per-turn snapshot. The browser extension
//! satisfies the request via the matching `WEB_*` action constant and
//! the response payload is decoded into the typed return.

use euro_bridge::BridgeService;
use eurora_tools::{BrowserOrigin, Empty, ToolError};
use eurora_tools_browser::web::{
    AccessibilityTree, FormInputsList, GetAccessibilityTreeArgs, InsertTextArgs, InsertTextResult,
    LinksList, ListFormInputsArgs, ListLinksArgs, PageMetadata, QuerySelectorArgs,
    QuerySelectorResult, ReadabilityArticle, SelectedText, WebAdapter,
};

use crate::BridgeClient;

/// Bridge action emitted for `browser_web_get_page_metadata`.
pub const WEB_GET_PAGE_METADATA: &str = "WEB_GET_PAGE_METADATA";
/// Bridge action emitted for `browser_web_get_accessibility_tree`.
pub const WEB_GET_ACCESSIBILITY_TREE: &str = "WEB_GET_ACCESSIBILITY_TREE";
/// Bridge action emitted for `browser_web_read_article`.
pub const WEB_GET_READABILITY_ARTICLE: &str = "WEB_GET_READABILITY_ARTICLE";
/// Bridge action emitted for `browser_web_get_selected_text`.
pub const WEB_GET_SELECTED_TEXT: &str = "WEB_GET_SELECTED_TEXT";
/// Bridge action emitted for `browser_web_query_selector`.
pub const WEB_QUERY_SELECTOR: &str = "WEB_QUERY_SELECTOR";
/// Bridge action emitted for `browser_web_list_links`.
pub const WEB_LIST_LINKS: &str = "WEB_LIST_LINKS";
/// Bridge action emitted for `browser_web_list_form_inputs`.
pub const WEB_LIST_FORM_INPUTS: &str = "WEB_LIST_FORM_INPUTS";
/// Bridge action emitted for `browser_web_insert_text`.
pub const WEB_INSERT_TEXT: &str = "WEB_INSERT_TEXT";

const METADATA_TOOL: &str = "browser_web_get_page_metadata";
const AX_TREE_TOOL: &str = "browser_web_get_accessibility_tree";
const READABILITY_TOOL: &str = "browser_web_read_article";
const SELECTION_TOOL: &str = "browser_web_get_selected_text";
const QUERY_TOOL: &str = "browser_web_query_selector";
const LINKS_TOOL: &str = "browser_web_list_links";
const FORM_INPUTS_TOOL: &str = "browser_web_list_form_inputs";
const INSERT_TEXT_TOOL: &str = "browser_web_insert_text";

/// Fulfils every [`WebAdapter`] method by hitting the browser process
/// registered with the underlying [`BridgeService`].
pub struct WebBridgeImpl {
    client: BridgeClient,
}

impl WebBridgeImpl {
    /// Bind to the process-wide [`BridgeService`] singleton.
    pub const fn new(bridge: &'static BridgeService) -> Self {
        Self {
            client: BridgeClient::new(bridge),
        }
    }

    /// Bind to a pre-constructed [`BridgeClient`] — convenient when the
    /// desktop wiring builds one client and shares it across every
    /// bridge-backed adapter on the same `BridgeService`.
    pub const fn with_client(client: BridgeClient) -> Self {
        Self { client }
    }
}

impl WebAdapter for WebBridgeImpl {
    async fn get_page_metadata(
        &self,
        target: &BrowserOrigin,
        args: Empty,
    ) -> Result<PageMetadata, ToolError> {
        self.client
            .call_action(target, WEB_GET_PAGE_METADATA, METADATA_TOOL, &args)
            .await
    }

    async fn get_accessibility_tree(
        &self,
        target: &BrowserOrigin,
        args: GetAccessibilityTreeArgs,
    ) -> Result<AccessibilityTree, ToolError> {
        self.client
            .call_action(target, WEB_GET_ACCESSIBILITY_TREE, AX_TREE_TOOL, &args)
            .await
    }

    async fn read_article(
        &self,
        target: &BrowserOrigin,
        args: Empty,
    ) -> Result<ReadabilityArticle, ToolError> {
        self.client
            .call_action(target, WEB_GET_READABILITY_ARTICLE, READABILITY_TOOL, &args)
            .await
    }

    async fn get_selected_text(
        &self,
        target: &BrowserOrigin,
        args: Empty,
    ) -> Result<SelectedText, ToolError> {
        self.client
            .call_action(target, WEB_GET_SELECTED_TEXT, SELECTION_TOOL, &args)
            .await
    }

    async fn query_selector(
        &self,
        target: &BrowserOrigin,
        args: QuerySelectorArgs,
    ) -> Result<QuerySelectorResult, ToolError> {
        self.client
            .call_action(target, WEB_QUERY_SELECTOR, QUERY_TOOL, &args)
            .await
    }

    async fn list_links(
        &self,
        target: &BrowserOrigin,
        args: ListLinksArgs,
    ) -> Result<LinksList, ToolError> {
        self.client
            .call_action(target, WEB_LIST_LINKS, LINKS_TOOL, &args)
            .await
    }

    async fn list_form_inputs(
        &self,
        target: &BrowserOrigin,
        args: ListFormInputsArgs,
    ) -> Result<FormInputsList, ToolError> {
        self.client
            .call_action(target, WEB_LIST_FORM_INPUTS, FORM_INPUTS_TOOL, &args)
            .await
    }

    async fn insert_text(
        &self,
        target: &BrowserOrigin,
        args: InsertTextArgs,
    ) -> Result<InsertTextResult, ToolError> {
        self.client
            .call_action(target, WEB_INSERT_TEXT, INSERT_TEXT_TOOL, &args)
            .await
    }
}
