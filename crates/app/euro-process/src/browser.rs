//! Catalog of browsers Eurora can identify by their executable name and the
//! extension stores that distribute the Eurora extension for each.
//!
//! Adding support for a new browser is a single variant in [`Browser`] plus
//! the corresponding arms in [`Browser::process_name`] and [`Browser::store`].
//! The compiler enforces that both match expressions cover every variant.

use crate::{os_pick, process_name_matches};

/// Public extension store that distributes the Eurora extension for a
/// browser.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BrowserStore {
    /// [Chrome Web Store](https://chromewebstore.google.com/).
    ChromeWebStore,
    /// [Mozilla Add-ons](https://addons.mozilla.org/).
    MozillaAddons,
    /// [Microsoft Edge Add-ons](https://microsoftedge.microsoft.com/addons/).
    EdgeAddons,
    /// The browser ships the extension bundled with the host application
    /// (e.g. Safari) or otherwise has no public, directly-linkable listing.
    Bundled,
}

impl BrowserStore {
    /// Public landing page for the Eurora extension on this store, or
    /// `None` for [`BrowserStore::Bundled`] which has no public listing.
    pub const fn extension_url(self) -> Option<&'static str> {
        match self {
            BrowserStore::ChromeWebStore => Some(
                "https://chromewebstore.google.com/detail/eurora/bfndcocdeinignobnnjplgoggmgebihm",
            ),
            BrowserStore::MozillaAddons => {
                Some("https://addons.mozilla.org/en-US/firefox/addon/eurora/")
            }
            BrowserStore::EdgeAddons => Some(
                "https://microsoftedge.microsoft.com/addons/detail/eurora/jldnbebjeaegfgpboohhoipokpbpncke",
            ),
            BrowserStore::Bundled => None,
        }
    }
}

/// Browsers Eurora can identify by their focused-window process name.
///
/// Variants are grouped by family (Chromium-based, then Firefox-based, then
/// standalone) for readability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Browser {
    Chrome,
    Brave,
    Edge,
    Opera,
    Vivaldi,
    Arc,
    Chromium,
    Firefox,
    Librewolf,
    Waterfox,
    Zen,
    Tor,
    DuckDuckGo,
    Falkon,
    Midori,
    SeaMonkey,
    Safari,
    PaleMoon,
}

impl Browser {
    /// Every known browser, in declaration order.
    ///
    /// Order is not part of the public contract; iterate with `.iter()` if
    /// you need a stable view.
    pub const ALL: &'static [Browser] = &[
        Browser::Chrome,
        Browser::Brave,
        Browser::Edge,
        Browser::Opera,
        Browser::Vivaldi,
        Browser::Arc,
        Browser::Chromium,
        Browser::Firefox,
        Browser::Librewolf,
        Browser::Waterfox,
        Browser::Zen,
        Browser::Tor,
        Browser::DuckDuckGo,
        Browser::Falkon,
        Browser::Midori,
        Browser::SeaMonkey,
        Browser::Safari,
        Browser::PaleMoon,
    ];

    /// Executable / process name reported by the focus tracker on the
    /// current target OS.
    pub fn process_name(self) -> &'static str {
        match self {
            Browser::Chrome => os_pick("chrome.exe", "chrome", "Google Chrome"),
            Browser::Brave => os_pick("brave.exe", "brave", "Brave Browser"),
            Browser::Edge => os_pick("msedge.exe", "msedge", "Microsoft Edge"),
            Browser::Opera => os_pick("opera.exe", "opera", "Opera"),
            Browser::Vivaldi => os_pick("vivaldi.exe", "vivaldi-bin", "Vivaldi"),
            Browser::Arc => os_pick("Arc.exe", "arc", "Arc"),
            Browser::Chromium => os_pick("chromium.exe", "chromium", "Chromium"),
            Browser::Firefox => os_pick("firefox.exe", "firefox", "Firefox"),
            Browser::Librewolf => os_pick("librewolf.exe", "librewolf", "LibreWolf"),
            Browser::Waterfox => os_pick("waterfox.exe", "waterfox", "Waterfox"),
            Browser::Zen => os_pick("zen.exe", "zen", "Zen Browser"),
            Browser::Tor => os_pick("tor.exe", "tor-browser", "Tor Browser"),
            Browser::DuckDuckGo => os_pick("DuckDuckGo.exe", "duckduckgo", "DuckDuckGo"),
            Browser::Falkon => os_pick("falkon.exe", "falkon", "Falkon"),
            Browser::Midori => os_pick("midori.exe", "midori", "Midori"),
            Browser::SeaMonkey => os_pick("seamonkey.exe", "seamonkey", "SeaMonkey"),
            Browser::Safari => os_pick("safari.exe", "safari", "Safari"),
            Browser::PaleMoon => os_pick("palemoon.exe", "palemoon", "Pale Moon"),
        }
    }

    /// Extension store that serves the Eurora extension for this browser.
    pub const fn store(self) -> BrowserStore {
        // TODO(opera-store): switch Opera to its own add-ons store once the
        // Eurora listing is published there.
        match self {
            Browser::Chrome
            | Browser::Brave
            | Browser::Opera
            | Browser::Vivaldi
            | Browser::Arc
            | Browser::Chromium => BrowserStore::ChromeWebStore,
            Browser::Edge => BrowserStore::EdgeAddons,
            Browser::Firefox
            | Browser::Librewolf
            | Browser::Waterfox
            | Browser::Zen
            | Browser::Tor
            | Browser::DuckDuckGo
            | Browser::Falkon
            | Browser::Midori
            | Browser::SeaMonkey => BrowserStore::MozillaAddons,
            Browser::Safari | Browser::PaleMoon => BrowserStore::Bundled,
        }
    }

    /// Resolve a focused-process executable name to a known browser.
    ///
    /// Matching is case-insensitive on Windows and byte-exact elsewhere; see
    /// `process_name_matches` in the crate root.
    pub fn from_process_name(process_name: &str) -> Option<Self> {
        if process_name.is_empty() {
            return None;
        }
        Browser::ALL
            .iter()
            .copied()
            .find(|b| process_name_matches(b.process_name(), process_name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn every_browser_round_trips_through_process_name() {
        for browser in Browser::ALL {
            assert_eq!(
                Browser::from_process_name(browser.process_name()),
                Some(*browser),
                "round-trip failed for {browser:?}"
            );
        }
    }

    #[test]
    fn process_names_are_unique() {
        let mut seen = HashSet::new();
        for browser in Browser::ALL {
            let name = browser.process_name();
            assert!(
                seen.insert(name),
                "duplicate process name {name:?} for {browser:?}"
            );
        }
    }

    #[test]
    fn store_classification_is_stable() {
        assert_eq!(Browser::Chrome.store(), BrowserStore::ChromeWebStore);
        assert_eq!(Browser::Brave.store(), BrowserStore::ChromeWebStore);
        assert_eq!(Browser::Edge.store(), BrowserStore::EdgeAddons);
        assert_eq!(Browser::Firefox.store(), BrowserStore::MozillaAddons);
        assert_eq!(Browser::Librewolf.store(), BrowserStore::MozillaAddons);
        assert_eq!(Browser::Safari.store(), BrowserStore::Bundled);
        assert_eq!(Browser::PaleMoon.store(), BrowserStore::Bundled);
    }

    #[test]
    fn unknown_process_does_not_resolve() {
        assert_eq!(Browser::from_process_name(""), None);
        assert_eq!(Browser::from_process_name("not-a-browser"), None);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_lookup_is_case_insensitive() {
        assert_eq!(
            Browser::from_process_name("CHROME.EXE"),
            Some(Browser::Chrome)
        );
        assert_eq!(
            Browser::from_process_name("Firefox.Exe"),
            Some(Browser::Firefox)
        );
    }

    #[test]
    fn linkable_stores_expose_extension_url() {
        assert!(BrowserStore::ChromeWebStore.extension_url().is_some());
        assert!(BrowserStore::MozillaAddons.extension_url().is_some());
        assert!(BrowserStore::EdgeAddons.extension_url().is_some());
    }

    #[test]
    fn bundled_store_has_no_extension_url() {
        assert!(BrowserStore::Bundled.extension_url().is_none());
    }
}
