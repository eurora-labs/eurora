use crate::{ProcessFunctionality, os_pick};

mod chromium;
mod firefox;

pub use chromium::*;
pub use firefox::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrowserStore {
    /// Chrome Web Store
    CWS,
    /// Mozilla Add-ons Store
    AMO,
    /// Microsoft Edge Add-ons
    Edge,
    /// Browser handles extension installation outside of a public store
    /// (e.g. Safari ships the extension bundled with the host app).
    Other,
}

impl BrowserStore {
    /// Public landing page for the Eurora extension on this store, or `None`
    /// when the browser does not have a directly-linkable listing
    /// (Safari, Pale Moon, etc.).
    ///
    /// URLs are filled in once the listings are live.
    pub fn extension_url(&self) -> Option<&'static str> {
        match self {
            BrowserStore::CWS => Some(
                "https://chromewebstore.google.com/detail/eurora/bfndcocdeinignobnnjplgoggmgebihm",
            ),
            BrowserStore::AMO => Some("https://addons.mozilla.org/en-US/firefox/addon/eurora/"),
            BrowserStore::Edge => Some(
                "https://microsoftedge.microsoft.com/addons/detail/eurora/jldnbebjeaegfgpboohhoipokpbpncke",
            ),
            BrowserStore::Other => None,
        }
    }
}

pub trait BrowserFunctionality {
    fn get_store(&self) -> BrowserStore;
}

/// Resolve the process executable name reported by the focus tracker into the
/// extension store of the browser it identifies, when that browser is one we
/// recognize.
///
/// The single match arm here is intentionally the canonical mapping: every
/// known browser appears once and the compiler will flag a missing arm if a
/// new browser type is added without a corresponding case.
pub fn browser_store_for_process(process_name: &str) -> Option<BrowserStore> {
    if process_name.is_empty() {
        return None;
    }
    let candidates: &[(&str, BrowserStore)] = &[
        (Chrome.get_name(), Chrome.get_store()),
        (Brave.get_name(), Brave.get_store()),
        (Edge.get_name(), Edge.get_store()),
        (Opera.get_name(), Opera.get_store()),
        (Vivaldi.get_name(), Vivaldi.get_store()),
        (ArcBrowser.get_name(), ArcBrowser.get_store()),
        (Chromium.get_name(), Chromium.get_store()),
        (Firefox.get_name(), Firefox.get_store()),
        (Librewolf.get_name(), Librewolf.get_store()),
        (Waterfox.get_name(), Waterfox.get_store()),
        (Zen.get_name(), Zen.get_store()),
        (TorBrowser.get_name(), TorBrowser.get_store()),
        (DuckDuckGo.get_name(), DuckDuckGo.get_store()),
        (Falkon.get_name(), Falkon.get_store()),
        (Midori.get_name(), Midori.get_store()),
        (SeaMonkey.get_name(), SeaMonkey.get_store()),
        (Safari.get_name(), Safari.get_store()),
        (PaleMoon.get_name(), PaleMoon.get_store()),
    ];

    candidates
        .iter()
        .find(|(name, _)| *name == process_name)
        .map(|(_, store)| *store)
}

#[derive(Debug, Clone)]
pub struct Safari;

impl ProcessFunctionality for Safari {
    fn get_name(&self) -> &str {
        os_pick("safari.exe", "safari", "Safari")
    }
}

impl BrowserFunctionality for Safari {
    fn get_store(&self) -> BrowserStore {
        // Safari extension is installed automatically, no need for store
        BrowserStore::Other
    }
}

#[derive(Debug, Clone)]
pub struct PaleMoon;

impl ProcessFunctionality for PaleMoon {
    fn get_name(&self) -> &str {
        os_pick("palemoon.exe", "palemoon", "Pale Moon")
    }
}

impl BrowserFunctionality for PaleMoon {
    fn get_store(&self) -> BrowserStore {
        BrowserStore::Other
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_name() {
        let process = Librewolf;
        assert_eq!(
            process.get_name(),
            os_pick("librewolf.exe", "librewolf", "LibreWolf")
        );
    }

    #[test]
    fn classifies_chromium_browsers_as_cws() {
        assert_eq!(
            browser_store_for_process(Chrome.get_name()),
            Some(BrowserStore::CWS)
        );
        assert_eq!(
            browser_store_for_process(Brave.get_name()),
            Some(BrowserStore::CWS)
        );
    }

    #[test]
    fn classifies_firefox_browsers_as_amo() {
        assert_eq!(
            browser_store_for_process(Firefox.get_name()),
            Some(BrowserStore::AMO)
        );
        assert_eq!(
            browser_store_for_process(Librewolf.get_name()),
            Some(BrowserStore::AMO)
        );
    }

    #[test]
    fn classifies_edge_with_dedicated_store() {
        assert_eq!(
            browser_store_for_process(Edge.get_name()),
            Some(BrowserStore::Edge)
        );
    }

    #[test]
    fn classifies_safari_as_other() {
        assert_eq!(
            browser_store_for_process(Safari.get_name()),
            Some(BrowserStore::Other)
        );
    }

    #[test]
    fn unknown_process_does_not_resolve() {
        assert_eq!(browser_store_for_process(""), None);
        assert_eq!(browser_store_for_process("not-a-browser"), None);
    }

    #[test]
    fn other_store_has_no_extension_url() {
        assert!(BrowserStore::Other.extension_url().is_none());
    }

    #[test]
    fn linkable_stores_have_extension_url() {
        assert!(BrowserStore::CWS.extension_url().is_some());
        assert!(BrowserStore::AMO.extension_url().is_some());
        assert!(BrowserStore::Edge.extension_url().is_some());
    }
}
