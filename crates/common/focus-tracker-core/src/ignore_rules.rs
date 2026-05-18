//! Composable ignore rules for focus events.
//!
//! Each [`IgnoreRule`] is a conjunction of two predicates — one over the
//! process name, one over the window title. A focus event is suppressed when
//! **any** rule in an [`IgnoreRules`] set matches (i.e. logical OR across
//! rules, logical AND inside a rule).
//!
//! Matching against the process name and the window title is byte-exact and
//! case-sensitive, matching the rest of this crate's "provide every spelling"
//! philosophy: process-name and title sources vary per platform and locale,
//! so silent fuzzy matching would mask bugs.
//!
//! [`WindowTitleMatch::Missing`] deliberately collapses `None` and
//! `Some("")` into the same category: "no meaningful title". Platforms
//! disagree about which of those they emit for a titleless window, so the
//! API hides that divergence.
//!
//! # Example
//!
//! ```
//! use focus_tracker_core::{IgnoreRule, IgnoreRules, WindowTitleMatch};
//!
//! // Suppress "whatever" only when it has no title; keep it when titled.
//! let rules = IgnoreRules::new([
//!     IgnoreRule::builder()
//!         .process_name("whatever")
//!         .window_title(WindowTitleMatch::Missing)
//!         .build(),
//! ]);
//!
//! assert!(rules.matches("whatever", None));
//! assert!(rules.matches("whatever", Some("")));
//! assert!(!rules.matches("whatever", Some("Untitled Document")));
//! assert!(!rules.matches("something-else", None));
//! ```

use bon::bon;

/// Predicate over [`FocusedWindow::process_name`].
///
/// [`FocusedWindow::process_name`]: crate::FocusedWindow::process_name
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProcessNameMatch {
    /// Matches any process name.
    Any,
    /// Byte-exact, case-sensitive match.
    Exact(String),
}

impl ProcessNameMatch {
    #[must_use]
    pub fn matches(&self, process_name: &str) -> bool {
        match self {
            Self::Any => true,
            Self::Exact(expected) => expected == process_name,
        }
    }
}

/// Predicate over [`FocusedWindow::window_title`].
///
/// `None` and `Some("")` are treated identically as "missing"; platforms
/// disagree on which they emit for a titleless window.
///
/// [`FocusedWindow::window_title`]: crate::FocusedWindow::window_title
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WindowTitleMatch {
    /// Matches whether or not a window title is present.
    Any,
    /// Matches when the title is absent or empty (`None` or `Some("")`).
    Missing,
    /// Matches when a non-empty title is present (any value).
    Present,
    /// Byte-exact, case-sensitive match against a non-empty title.
    Exact(String),
}

impl WindowTitleMatch {
    #[must_use]
    pub fn matches(&self, window_title: Option<&str>) -> bool {
        let is_missing = window_title.is_none_or(str::is_empty);
        match self {
            Self::Any => true,
            Self::Missing => is_missing,
            Self::Present => !is_missing,
            Self::Exact(expected) => window_title.is_some_and(|t| t == expected),
        }
    }
}

/// A single ignore predicate: matches when both the process-name and
/// window-title predicates match (logical AND).
///
/// Construct with [`IgnoreRule::builder`]. Both fields default to their
/// `Any` variant, so omitting a setter means "don't constrain on that
/// dimension". `.process_name(s)` accepts any `Into<String>` and is sugar
/// for [`ProcessNameMatch::Exact`]; `.window_title(m)` takes a
/// [`WindowTitleMatch`] directly.
///
/// # Example
///
/// ```
/// use focus_tracker_core::{IgnoreRule, WindowTitleMatch};
///
/// // Ignore Explorer.EXE only when it has no title.
/// let rule = IgnoreRule::builder()
///     .process_name("Explorer.EXE")
///     .window_title(WindowTitleMatch::Missing)
///     .build();
///
/// assert!(rule.matches("Explorer.EXE", None));
/// assert!(!rule.matches("Explorer.EXE", Some("Documents")));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IgnoreRule {
    process_name: ProcessNameMatch,
    window_title: WindowTitleMatch,
}

#[bon]
impl IgnoreRule {
    /// Builds an ignore rule. Both fields default to their `Any` variant.
    ///
    /// `.process_name(s)` takes a string and stores
    /// [`ProcessNameMatch::Exact`]; omit the setter to leave the
    /// process-name predicate as [`ProcessNameMatch::Any`]. To match any
    /// process and constrain only on title, omit `.process_name(...)`.
    #[builder]
    pub fn new(
        #[builder(
            default = ProcessNameMatch::Any,
            with = |name: impl Into<String>| ProcessNameMatch::Exact(name.into()),
        )]
        process_name: ProcessNameMatch,
        #[builder(default = WindowTitleMatch::Any)] window_title: WindowTitleMatch,
    ) -> Self {
        Self {
            process_name,
            window_title,
        }
    }
}

impl IgnoreRule {
    /// Returns the rule's process-name predicate.
    #[must_use]
    pub fn process_name_match(&self) -> &ProcessNameMatch {
        &self.process_name
    }

    /// Returns the rule's window-title predicate.
    #[must_use]
    pub fn window_title_match(&self) -> &WindowTitleMatch {
        &self.window_title
    }

    /// Returns `true` when the rule matches the given focus event fields.
    #[must_use]
    pub fn matches(&self, process_name: &str, window_title: Option<&str>) -> bool {
        self.process_name.matches(process_name) && self.window_title.matches(window_title)
    }
}

/// A set of ignore rules. A focus event is ignored when **any** rule matches.
///
/// Order is preserved for debugging; matching is order-independent.
#[derive(Debug, Clone, Default)]
pub struct IgnoreRules {
    rules: Vec<IgnoreRule>,
}

impl IgnoreRules {
    /// Builds a rule set from an iterator of rules.
    pub fn new<I>(rules: I) -> Self
    where
        I: IntoIterator<Item = IgnoreRule>,
    {
        Self {
            rules: rules.into_iter().collect(),
        }
    }

    /// Returns `true` when at least one rule matches.
    #[must_use]
    pub fn matches(&self, process_name: &str, window_title: Option<&str>) -> bool {
        self.rules
            .iter()
            .any(|rule| rule.matches(process_name, window_title))
    }

    /// Returns the number of rules in the set.
    #[must_use]
    pub fn len(&self) -> usize {
        self.rules.len()
    }

    /// Returns `true` when the set has no rules.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    /// Iterates over the rules in insertion order.
    pub fn iter(&self) -> impl Iterator<Item = &IgnoreRule> {
        self.rules.iter()
    }
}

impl FromIterator<IgnoreRule> for IgnoreRules {
    fn from_iter<I: IntoIterator<Item = IgnoreRule>>(iter: I) -> Self {
        Self::new(iter)
    }
}

impl<'a> IntoIterator for &'a IgnoreRules {
    type Item = &'a IgnoreRule;
    type IntoIter = std::slice::Iter<'a, IgnoreRule>;

    fn into_iter(self) -> Self::IntoIter {
        self.rules.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn process_name_any_matches_anything() {
        let m = ProcessNameMatch::Any;
        assert!(m.matches(""));
        assert!(m.matches("firefox"));
        assert!(m.matches("Firefox"));
    }

    #[test]
    fn process_name_exact_is_byte_exact() {
        let m = ProcessNameMatch::Exact("firefox".into());
        assert!(m.matches("firefox"));
        assert!(!m.matches("Firefox"));
        assert!(!m.matches("firefox.exe"));
        assert!(!m.matches(""));
    }

    #[test]
    fn window_title_any_matches_anything() {
        let m = WindowTitleMatch::Any;
        assert!(m.matches(None));
        assert!(m.matches(Some("")));
        assert!(m.matches(Some("hello")));
    }

    #[test]
    fn window_title_missing_treats_none_and_empty_alike() {
        let m = WindowTitleMatch::Missing;
        assert!(m.matches(None));
        assert!(m.matches(Some("")));
        assert!(!m.matches(Some("hello")));
        assert!(!m.matches(Some(" ")));
    }

    #[test]
    fn window_title_present_excludes_none_and_empty() {
        let m = WindowTitleMatch::Present;
        assert!(!m.matches(None));
        assert!(!m.matches(Some("")));
        assert!(m.matches(Some("hello")));
        assert!(m.matches(Some(" ")));
    }

    #[test]
    fn window_title_exact_is_byte_exact_and_never_matches_missing() {
        let m = WindowTitleMatch::Exact("Inbox".into());
        assert!(m.matches(Some("Inbox")));
        assert!(!m.matches(Some("inbox")));
        assert!(!m.matches(Some("Inbox ")));
        assert!(!m.matches(Some("")));
        assert!(!m.matches(None));
    }

    #[test]
    fn builder_defaults_to_any_any() {
        let rule = IgnoreRule::builder().build();
        assert_eq!(rule.process_name_match(), &ProcessNameMatch::Any);
        assert_eq!(rule.window_title_match(), &WindowTitleMatch::Any);
        assert!(rule.matches("anything", None));
        assert!(rule.matches("anything", Some("titled")));
    }

    #[test]
    fn builder_process_name_with_any_title() {
        let rule = IgnoreRule::builder().process_name("firefox").build();
        assert!(rule.matches("firefox", None));
        assert!(rule.matches("firefox", Some("")));
        assert!(rule.matches("firefox", Some("News")));
        assert!(!rule.matches("Firefox", None));
        assert!(!rule.matches("chrome", Some("News")));
    }

    #[test]
    fn builder_process_name_with_title_missing_matches_the_user_case() {
        let rule = IgnoreRule::builder()
            .process_name("whatever")
            .window_title(WindowTitleMatch::Missing)
            .build();
        assert!(rule.matches("whatever", None));
        assert!(rule.matches("whatever", Some("")));
        assert!(!rule.matches("whatever", Some("Doc")));
        assert!(!rule.matches("other", None));
    }

    #[test]
    fn builder_process_name_with_title_present() {
        let rule = IgnoreRule::builder()
            .process_name("whatever")
            .window_title(WindowTitleMatch::Present)
            .build();
        assert!(!rule.matches("whatever", None));
        assert!(!rule.matches("whatever", Some("")));
        assert!(rule.matches("whatever", Some("Doc")));
        assert!(!rule.matches("other", Some("Doc")));
    }

    #[test]
    fn builder_process_name_with_title_exact() {
        let rule = IgnoreRule::builder()
            .process_name("whatever")
            .window_title(WindowTitleMatch::Exact("Splash".into()))
            .build();
        assert!(rule.matches("whatever", Some("Splash")));
        assert!(!rule.matches("whatever", Some("splash")));
        assert!(!rule.matches("whatever", None));
        assert!(!rule.matches("whatever", Some("")));
        assert!(!rule.matches("other", Some("Splash")));
    }

    #[test]
    fn builder_any_process_with_title_missing() {
        let rule = IgnoreRule::builder()
            .window_title(WindowTitleMatch::Missing)
            .build();
        assert!(rule.matches("anything", None));
        assert!(rule.matches("anything-else", Some("")));
        assert!(!rule.matches("anything", Some("Titled")));
    }

    #[test]
    fn builder_accepts_string_and_str() {
        let rule_from_str = IgnoreRule::builder().process_name("p").build();
        let rule_from_string = IgnoreRule::builder()
            .process_name(String::from("p"))
            .build();
        assert_eq!(rule_from_str, rule_from_string);
    }

    #[test]
    fn rule_accessors_expose_matchers() {
        let rule = IgnoreRule::builder()
            .process_name("p")
            .window_title(WindowTitleMatch::Missing)
            .build();
        assert_eq!(
            rule.process_name_match(),
            &ProcessNameMatch::Exact("p".into())
        );
        assert_eq!(rule.window_title_match(), &WindowTitleMatch::Missing);
    }

    #[test]
    fn rules_default_is_empty_and_matches_nothing() {
        let rules = IgnoreRules::default();
        assert!(rules.is_empty());
        assert_eq!(rules.len(), 0);
        assert!(!rules.matches("anything", None));
        assert!(!rules.matches("anything", Some("x")));
    }

    #[test]
    fn rules_or_across_rules() {
        let rules = IgnoreRules::new([
            IgnoreRule::builder()
                .process_name("whatever")
                .window_title(WindowTitleMatch::Missing)
                .build(),
            IgnoreRule::builder().process_name("chrome").build(),
        ]);
        assert!(rules.matches("whatever", None));
        assert!(rules.matches("chrome", Some("News")));
        assert!(!rules.matches("whatever", Some("Doc")));
        assert!(!rules.matches("other", None));
    }

    #[test]
    fn rules_len_reflects_input() {
        let rules = IgnoreRules::new([
            IgnoreRule::builder().process_name("a").build(),
            IgnoreRule::builder().process_name("b").build(),
            IgnoreRule::builder().process_name("a").build(),
        ]);
        // Rules are not deduplicated — duplicates are preserved so callers
        // can reason about debug output. Matching semantics are unaffected.
        assert_eq!(rules.len(), 3);
    }

    #[test]
    fn rules_iter_preserves_insertion_order() {
        let rules = IgnoreRules::new([
            IgnoreRule::builder().process_name("a").build(),
            IgnoreRule::builder().process_name("b").build(),
        ]);
        let names: Vec<_> = rules
            .iter()
            .map(|r| match r.process_name_match() {
                ProcessNameMatch::Exact(s) => s.as_str(),
                ProcessNameMatch::Any => "",
            })
            .collect();
        assert_eq!(names, ["a", "b"]);
    }

    #[test]
    fn rules_from_iterator() {
        let rules: IgnoreRules = [
            IgnoreRule::builder().process_name("a").build(),
            IgnoreRule::builder().process_name("b").build(),
        ]
        .into_iter()
        .collect();
        assert_eq!(rules.len(), 2);
        assert!(rules.matches("a", None));
        assert!(rules.matches("b", Some("x")));
    }

    #[test]
    fn rules_into_iter_by_reference() {
        let rules = IgnoreRules::new([IgnoreRule::builder().process_name("a").build()]);
        let count = (&rules).into_iter().count();
        assert_eq!(count, 1);
    }
}
