use super::{BrowserFunctionality, BrowserStore};
use crate::{ProcessFunctionality, os_pick};

#[derive(Debug, Clone)]
pub struct Chrome;

impl ProcessFunctionality for Chrome {
    fn get_name(&self) -> &str {
        os_pick("chrome.exe", "chrome", "Google Chrome")
    }
}

impl BrowserFunctionality for Chrome {
    fn get_store(&self) -> BrowserStore {
        BrowserStore::CWS
    }
}

#[derive(Debug, Clone)]
pub struct Opera;

impl ProcessFunctionality for Opera {
    fn get_name(&self) -> &str {
        os_pick("opera.exe", "opera", "Opera")
    }
}

impl BrowserFunctionality for Opera {
    fn get_store(&self) -> BrowserStore {
        // TODO: Switch to Opera's store once published there
        BrowserStore::CWS
    }
}

#[derive(Debug, Clone)]
pub struct Brave;

impl ProcessFunctionality for Brave {
    fn get_name(&self) -> &str {
        os_pick("brave.exe", "brave", "Brave Browser")
    }
}

impl BrowserFunctionality for Brave {
    fn get_store(&self) -> BrowserStore {
        BrowserStore::CWS
    }
}

#[derive(Debug, Clone)]
pub struct Edge;

impl ProcessFunctionality for Edge {
    fn get_name(&self) -> &str {
        os_pick("msedge.exe", "msedge", "Microsoft Edge")
    }
}

impl BrowserFunctionality for Edge {
    fn get_store(&self) -> BrowserStore {
        BrowserStore::Edge
    }
}

#[derive(Debug, Clone)]
pub struct Vivaldi;

impl ProcessFunctionality for Vivaldi {
    fn get_name(&self) -> &str {
        os_pick("vivaldi.exe", "vivaldi-bin", "Vivaldi")
    }
}

impl BrowserFunctionality for Vivaldi {
    fn get_store(&self) -> BrowserStore {
        BrowserStore::CWS
    }
}

#[derive(Debug, Clone)]
pub struct ArcBrowser;

impl ProcessFunctionality for ArcBrowser {
    fn get_name(&self) -> &str {
        os_pick("Arc.exe", "arc", "Arc")
    }
}

impl BrowserFunctionality for ArcBrowser {
    fn get_store(&self) -> BrowserStore {
        BrowserStore::CWS
    }
}

#[derive(Debug, Clone)]
pub struct Chromium;

impl ProcessFunctionality for Chromium {
    fn get_name(&self) -> &str {
        os_pick("chromium.exe", "chromium", "Chromium")
    }
}

impl BrowserFunctionality for Chromium {
    fn get_store(&self) -> BrowserStore {
        BrowserStore::CWS
    }
}
