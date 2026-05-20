//! The web-page adapter trait — eight tools, one namespace.
//!
//! Read-only tools observe the active `http(s)` tab; [`insert_text`] is
//! the only mutator. The `#[adapter]` macro expands this module into:
//!
//! - [`WEB_DESCRIPTORS`] — the static descriptor table consumed by the
//!   server-side agent loop via `WireToolDescriptor`.
//! - [`WebDispatcher<T>`] — the runtime dispatcher that decodes
//!   `IncomingCall::arguments`, validates the [`Origin`](eurora_tools::Origin)
//!   variant, awaits the user-written adapter, and re-encodes the result.
//! - A Send-bounded [`WebAdapter`] trait plus a non-Send
//!   [`WebAdapterLocal`] variant produced by `trait_variant::make`.
//!   Production code should `impl WebAdapter for …`; the `Local` variant
//!   comes for free via the blanket impl and is convenient when stubbing
//!   single-threaded tests.
//!
//! The first paragraph of each method's rustdoc is extracted by the
//! macro as the tool description sent to the LLM, so the wording here
//! is part of the runtime surface — keep it user-facing.
//!
//! [`insert_text`]: WebAdapter::insert_text

use eurora_tools::{BrowserOrigin, Empty, ToolError, adapter};

use crate::types::{
    AccessibilityTree, FormInputsList, GetAccessibilityTreeArgs, InsertTextArgs, InsertTextResult,
    LinksList, ListFormInputsArgs, ListLinksArgs, PageMetadata, QuerySelectorArgs,
    QuerySelectorResult, ReadabilityArticle, SelectedText,
};

/// Generic web-page tools — available on every `http(s)` tab the user
/// has focused in a registered browser.
#[adapter(namespace = "browser_web", version = 1)]
pub trait WebAdapter: Send + Sync {
    /// Get the active tab's URL, title, host, language, charset,
    /// description, OpenGraph tags, and viewport/scroll metrics.
    #[tool(
        timeout_ms = 2_000,
        source = "bridge(browser)",
        requires_context = "web::page"
    )]
    async fn get_page_metadata(
        &self,
        target: &BrowserOrigin,
        args: Empty,
    ) -> Result<PageMetadata, ToolError>;

    /// Get the active page's accessibility tree derived from ARIA roles
    /// and implicit element roles. Defaults: depth 12, 500 nodes; hard
    /// cap 2 000 nodes.
    #[tool(
        timeout_ms = 5_000,
        source = "bridge(browser)",
        requires_context = "web::page"
    )]
    async fn get_accessibility_tree(
        &self,
        target: &BrowserOrigin,
        args: GetAccessibilityTreeArgs,
    ) -> Result<AccessibilityTree, ToolError>;

    /// Get the readable text and HTML of the active page's main article
    /// via Mozilla Readability. Use this to transcribe, summarise, or
    /// quote the page's primary content. Bodies are truncated at 32 KB.
    #[tool(
        timeout_ms = 5_000,
        source = "bridge(browser)",
        requires_context = "web::page"
    )]
    async fn read_article(
        &self,
        target: &BrowserOrigin,
        args: Empty,
    ) -> Result<ReadabilityArticle, ToolError>;

    /// Get whatever text the user has highlighted in the active document
    /// right now, plus XPaths to the selection's anchor and focus
    /// endpoints. Returns empty `text` when nothing is selected.
    #[tool(
        timeout_ms = 1_000,
        source = "bridge(browser)",
        requires_context = "web::page"
    )]
    async fn get_selected_text(
        &self,
        target: &BrowserOrigin,
        args: Empty,
    ) -> Result<SelectedText, ToolError>;

    /// Query the active page's DOM with a CSS selector. Returns text,
    /// outer HTML, attributes, or bounding rect per match (caller picks
    /// which via `include`). Hidden, password, file, and submit-style
    /// inputs are always elided.
    #[tool(
        timeout_ms = 5_000,
        source = "bridge(browser)",
        requires_context = "web::page"
    )]
    async fn query_selector(
        &self,
        target: &BrowserOrigin,
        args: QuerySelectorArgs,
    ) -> Result<QuerySelectorResult, ToolError>;

    /// List the clickable navigation links on the active page with their
    /// URL, accessible label, and role. Use this to suggest where the
    /// user can navigate; the user performs the navigation themselves.
    #[tool(
        timeout_ms = 3_000,
        source = "bridge(browser)",
        requires_context = "web::page"
    )]
    async fn list_links(
        &self,
        target: &BrowserOrigin,
        args: ListLinksArgs,
    ) -> Result<LinksList, ToolError>;

    /// List the text-typed editable form fields on the active page with
    /// their labels and current values. Use this to find a `field_id`
    /// before calling `browser_web_insert_text`. Password, file,
    /// hidden, submit, disabled, and readonly fields are excluded.
    #[tool(
        timeout_ms = 3_000,
        source = "bridge(browser)",
        requires_context = "web::page"
    )]
    async fn list_form_inputs(
        &self,
        target: &BrowserOrigin,
        args: ListFormInputsArgs,
    ) -> Result<FormInputsList, ToolError>;

    /// Insert text into a known editable field. The `field_id` must come
    /// from a prior `browser_web_list_form_inputs` call. Accepts only
    /// text-typed `<input>`, `<textarea>`, and `[contenteditable]`.
    /// Never fires `change`, `keydown`, `keyup`, `keypress`, `focus`,
    /// `blur`, or `submit`.
    #[tool(
        timeout_ms = 2_000,
        source = "bridge(browser)",
        requires_context = "web::page",
        requires_user_approval = true
    )]
    async fn insert_text(
        &self,
        target: &BrowserOrigin,
        args: InsertTextArgs,
    ) -> Result<InsertTextResult, ToolError>;
}
