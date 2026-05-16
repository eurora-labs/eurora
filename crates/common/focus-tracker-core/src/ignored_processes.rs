use std::collections::HashSet;

/// An exact-match, case-sensitive set of process names to ignore.
///
/// Matching is performed against [`FocusedWindow::process_name`] exactly as
/// emitted by the platform. **No normalization is applied** — `"firefox"`
/// does not match `"firefox.exe"`, and `"Firefox"` does not match
/// `"firefox"`.
///
/// This is deliberate: process-name sources vary per platform (and across
/// processes in surprising ways), so silent fuzzy matching would mask bugs.
/// Provide every spelling you want to suppress, per platform.
///
/// [`FocusedWindow::process_name`]: crate::FocusedWindow::process_name
///
/// # Example
///
/// ```
/// use focus_tracker_core::IgnoredProcesses;
///
/// let ignored = IgnoredProcesses::new(["firefox.exe", "chrome.exe"]);
/// assert!(ignored.contains("firefox.exe"));
/// assert!(!ignored.contains("firefox"));
/// assert!(!ignored.contains("Firefox.exe"));
/// ```
#[derive(Debug, Clone, Default)]
pub struct IgnoredProcesses {
    names: HashSet<String>,
}

impl IgnoredProcesses {
    /// Builds a new ignore list from an iterator of process names.
    pub fn new<I, S>(names: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            names: names.into_iter().map(Into::into).collect(),
        }
    }

    /// Returns `true` if `process_name` is in the ignore list.
    ///
    /// Matching is byte-exact; no case folding or suffix stripping.
    #[must_use]
    pub fn contains(&self, process_name: &str) -> bool {
        self.names.contains(process_name)
    }

    /// Returns `true` if the ignore list is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }

    /// Returns the number of distinct names in the ignore list.
    #[must_use]
    pub fn len(&self) -> usize {
        self.names.len()
    }

    /// Returns an iterator over the names in the ignore list.
    pub fn iter(&self) -> impl Iterator<Item = &str> {
        self.names.iter().map(String::as_str)
    }
}

impl<S: Into<String>> FromIterator<S> for IgnoredProcesses {
    fn from_iter<I: IntoIterator<Item = S>>(iter: I) -> Self {
        Self::new(iter)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_empty() {
        let ignored = IgnoredProcesses::default();
        assert!(ignored.is_empty());
        assert_eq!(ignored.len(), 0);
        assert!(!ignored.contains("anything"));
    }

    #[test]
    fn exact_match() {
        let ignored = IgnoredProcesses::new(["firefox.exe"]);
        assert!(ignored.contains("firefox.exe"));
    }

    #[test]
    fn no_suffix_stripping() {
        let ignored = IgnoredProcesses::new(["firefox"]);
        assert!(!ignored.contains("firefox.exe"));
    }

    #[test]
    fn no_suffix_extension() {
        let ignored = IgnoredProcesses::new(["firefox.exe"]);
        assert!(!ignored.contains("firefox"));
    }

    #[test]
    fn case_sensitive() {
        let ignored = IgnoredProcesses::new(["Firefox"]);
        assert!(ignored.contains("Firefox"));
        assert!(!ignored.contains("firefox"));
        assert!(!ignored.contains("FIREFOX"));
    }

    #[test]
    fn duplicates_collapse() {
        let ignored = IgnoredProcesses::new(["chrome", "chrome", "chrome"]);
        assert_eq!(ignored.len(), 1);
        assert!(ignored.contains("chrome"));
    }

    #[test]
    fn accepts_string_and_str() {
        let ignored = IgnoredProcesses::new([String::from("a"), String::from("b")]);
        assert!(ignored.contains("a"));
        assert!(ignored.contains("b"));

        let ignored = IgnoredProcesses::new(["a", "b"]);
        assert!(ignored.contains("a"));
        assert!(ignored.contains("b"));
    }

    #[test]
    fn from_iterator() {
        let names = vec!["a", "b", "c"];
        let ignored: IgnoredProcesses = names.into_iter().collect();
        assert_eq!(ignored.len(), 3);
        assert!(ignored.contains("a"));
        assert!(ignored.contains("b"));
        assert!(ignored.contains("c"));
    }

    #[test]
    fn iter_returns_all_names() {
        let ignored = IgnoredProcesses::new(["a", "b"]);
        let collected: HashSet<&str> = ignored.iter().collect();
        assert_eq!(collected, HashSet::from(["a", "b"]));
    }

    #[test]
    fn empty_input_is_empty() {
        let ignored = IgnoredProcesses::new::<_, &str>([]);
        assert!(ignored.is_empty());
    }
}
