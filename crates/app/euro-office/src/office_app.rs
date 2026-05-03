//! Catalog of Microsoft Office desktop applications Eurora can identify
//! from the focused-window process name.
//!
//! Mirrors the `Browser` enum in `euro-process`: adding support for a new
//! Office app is a single variant in [`OfficeApp`] plus the matching arm
//! in [`OfficeApp::process_name`]; the compiler enforces the latter.

use euro_process::{os_pick, process_name_matches};

/// Office apps Eurora can identify from a focused-window process name.
///
/// Only Word ships with a working integration today. Excel and
/// PowerPoint will land as additional variants once their add-in
/// runtimes are in place.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OfficeApp {
    Word,
}

impl OfficeApp {
    /// Every known Office app, in declaration order.
    ///
    /// Order is not part of the public contract; iterate with `.iter()`
    /// if you need a stable view.
    pub const ALL: &'static [OfficeApp] = &[OfficeApp::Word];

    /// Executable / process name reported by the focus tracker on the
    /// current target OS.
    ///
    /// Linux values are included for symmetry only — Word does not run
    /// natively on Linux, so the focus tracker will never report them.
    pub fn process_name(self) -> &'static str {
        match self {
            OfficeApp::Word => os_pick("WINWORD.EXE", "winword", "Microsoft Word"),
        }
    }

    /// Resolve a focused-process executable name to a known Office app.
    ///
    /// Matching is case-insensitive on Windows and byte-exact elsewhere;
    /// see `process_name_matches` in `euro-process`.
    pub fn from_process_name(name: &str) -> Option<Self> {
        if name.is_empty() {
            return None;
        }
        OfficeApp::ALL
            .iter()
            .copied()
            .find(|app| process_name_matches(app.process_name(), name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn every_office_app_round_trips_through_process_name() {
        for app in OfficeApp::ALL {
            assert_eq!(
                OfficeApp::from_process_name(app.process_name()),
                Some(*app),
                "round-trip failed for {app:?}"
            );
        }
    }

    #[test]
    fn process_names_are_unique() {
        let mut seen = HashSet::new();
        for app in OfficeApp::ALL {
            let name = app.process_name();
            assert!(
                seen.insert(name),
                "duplicate process name {name:?} for {app:?}"
            );
        }
    }

    #[test]
    fn unknown_process_does_not_resolve() {
        assert_eq!(OfficeApp::from_process_name(""), None);
        assert_eq!(OfficeApp::from_process_name("not-office"), None);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_lookup_is_case_insensitive() {
        assert_eq!(
            OfficeApp::from_process_name("WINWORD.EXE"),
            Some(OfficeApp::Word)
        );
        assert_eq!(
            OfficeApp::from_process_name("winword.exe"),
            Some(OfficeApp::Word)
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn macos_matches_full_app_name() {
        assert_eq!(
            OfficeApp::from_process_name("Microsoft Word"),
            Some(OfficeApp::Word)
        );
        assert_eq!(OfficeApp::from_process_name("Microsoft Excel"), None);
    }
}
