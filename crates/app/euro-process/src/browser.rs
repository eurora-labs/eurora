use crate::{ProcessFunctionality, os_pick};

mod chromium;
mod firefox;

pub use chromium::*;
pub use firefox::*;

pub enum BrowserStore {
    /// Chrome Web Store
    CWS,
    /// Mozilla Add-ons Store
    AMO,
    Edge,
    Other,
}

pub trait BrowserFunctionality {
    fn get_store(&self) -> BrowserStore;
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
}
