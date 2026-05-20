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
#[adapter(namespace = "browser::web", version = 1)]
pub trait WebAdapter: Send + Sync {
    /// Page-level metadata for the active tab: URL, title, host, language,
    /// charset, description, OpenGraph tags, and viewport / scroll metrics.
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

    /// Approximate accessibility tree of the active page, derived from
    /// ARIA roles, implicit element roles, and an accname-spec accessible-
    /// name walk. Defaults: depth 12, 500 nodes; hard cap 2 000 nodes.
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

    /// Reader-mode extraction of the page's primary article via Mozilla
    /// Readability, including title, byline, language, and HTML / text
    /// bodies. Per-call truncation caps both bodies at 32 KB.
    #[tool(
        timeout_ms = 5_000,
        source = "bridge(browser)",
        requires_context = "web::page"
    )]
    async fn get_readability_article(
        &self,
        target: &BrowserOrigin,
        args: Empty,
    ) -> Result<ReadabilityArticle, ToolError>;

    /// Whatever the user has highlighted in the active document right now,
    /// plus XPaths to the selection's anchor and focus endpoints. Empty
    /// selection returns an empty `text` field.
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

    /// Generalised DOM reader — returns text content, outer HTML,
    /// attributes, or bounding rect per match. Hidden, password, file,
    /// and submit-style inputs and CSRF-shaped `<meta>` tags are elided
    /// from results regardless of selector; the pre-filter match count
    /// is reported separately so the model can tell when elision happened.
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

    /// Inventory of clickable navigations on the active page (URL,
    /// accessible label, role). The model can suggest navigations from
    /// this list; the user is the one who navigates.
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

    /// Inventory of text-typed editable fields on the active page, with
    /// accessible labels and current values. Password, file, hidden,
    /// submit, disabled, and readonly fields are excluded; each entry's
    /// `field_id` is the only legal target for `insert_text`.
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
    /// from a prior `list_form_inputs` call; arbitrary selectors are
    /// rejected. Restricted to text-typed `<input>`, `<textarea>`, and
    /// `[contenteditable]`. Never fires `change`, `keydown`, `keyup`,
    /// `keypress`, `focus`, `blur`, or `submit` — sites that submit on
    /// `Enter` or on `blur` are unaffected.
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
