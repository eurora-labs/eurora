//! Eurora's own desktop processes, identified by their executable name.
//!
//! Adding a new variant requires updating [`AppProcess::ALL`] and the
//! [`AppProcess::process_name`] match arm; the compiler enforces the latter.

use crate::{os_pick, process_name_matches};

/// A Eurora-owned desktop process as identified by the focused-window
/// process name reported by the OS.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AppProcess {
    /// The main Eurora desktop app. Reports a different executable name in
    /// debug builds (`euro-tauri`) than in release builds (`eurora`).
    Eurora,
    /// The nightly channel build of the desktop app.
    EuroraNightly,
}

impl AppProcess {
    /// Every known Eurora process, in declaration order.
    ///
    /// Order is not part of the public contract; iterate with `.iter()` if
    /// you need a stable view.
    pub const ALL: &'static [AppProcess] = &[AppProcess::Eurora, AppProcess::EuroraNightly];

    /// Executable / process name reported by the focus tracker on the
    /// current target OS.
    pub fn process_name(self) -> &'static str {
        match self {
            AppProcess::Eurora => {
                if cfg!(debug_assertions) {
                    os_pick("euro-tauri.exe", "euro-tauri", "euro-tauri")
                } else {
                    os_pick("eurora.exe", "eurora", "Eurora")
                }
            }
            AppProcess::EuroraNightly => {
                os_pick("eurora-nightly.exe", "eurora-nightly", "Eurora Nightly")
            }
        }
    }

    /// Resolve a focused-process executable name to a known Eurora process.
    ///
    /// Matching is case-insensitive on Windows and byte-exact elsewhere; see
    /// `process_name_matches` in the crate root.
    pub fn from_process_name(name: &str) -> Option<Self> {
        if name.is_empty() {
            return None;
        }
        AppProcess::ALL
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
    fn every_app_process_round_trips_through_process_name() {
        for app in AppProcess::ALL {
            assert_eq!(
                AppProcess::from_process_name(app.process_name()),
                Some(*app),
                "round-trip failed for {app:?}"
            );
        }
    }

    #[test]
    fn process_names_are_unique() {
        let mut seen = HashSet::new();
        for app in AppProcess::ALL {
            let name = app.process_name();
            assert!(
                seen.insert(name),
                "duplicate process name {name:?} for {app:?}"
            );
        }
    }

    #[test]
    fn unknown_process_does_not_resolve() {
        assert_eq!(AppProcess::from_process_name(""), None);
        assert_eq!(AppProcess::from_process_name("not-eurora"), None);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_lookup_is_case_insensitive() {
        let known = AppProcess::Eurora.process_name();
        let upper = known.to_ascii_uppercase();
        assert_eq!(
            AppProcess::from_process_name(&upper),
            Some(AppProcess::Eurora)
        );
    }
}
