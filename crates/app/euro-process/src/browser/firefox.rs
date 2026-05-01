use super::{BrowserFunctionality, BrowserStore};
use crate::{ProcessFunctionality, os_pick};

#[derive(Debug, Clone)]
pub struct Librewolf;

impl ProcessFunctionality for Librewolf {
    fn get_name(&self) -> &str {
        os_pick("librewolf.exe", "librewolf", "LibreWolf")
    }
}

impl BrowserFunctionality for Librewolf {
    fn get_store(&self) -> BrowserStore {
        BrowserStore::AMO
    }
}

#[derive(Debug, Clone)]
pub struct Firefox;

impl ProcessFunctionality for Firefox {
    fn get_name(&self) -> &str {
        os_pick("firefox.exe", "firefox", "Firefox")
    }
}

impl BrowserFunctionality for Firefox {
    fn get_store(&self) -> BrowserStore {
        BrowserStore::AMO
    }
}

#[derive(Debug, Clone)]
pub struct TorBrowser;

impl ProcessFunctionality for TorBrowser {
    fn get_name(&self) -> &str {
        os_pick("tor.exe", "tor-browser", "Tor Browser")
    }
}

impl BrowserFunctionality for TorBrowser {
    fn get_store(&self) -> BrowserStore {
        BrowserStore::AMO
    }
}

#[derive(Debug, Clone)]
pub struct Waterfox;

impl ProcessFunctionality for Waterfox {
    fn get_name(&self) -> &str {
        os_pick("waterfox.exe", "waterfox", "Waterfox")
    }
}

impl BrowserFunctionality for Waterfox {
    fn get_store(&self) -> BrowserStore {
        BrowserStore::AMO
    }
}

#[derive(Debug, Clone)]
pub struct Zen;

impl ProcessFunctionality for Zen {
    fn get_name(&self) -> &str {
        os_pick("zen.exe", "zen", "Zen Browser")
    }
}

impl BrowserFunctionality for Zen {
    fn get_store(&self) -> BrowserStore {
        BrowserStore::AMO
    }
}

#[derive(Debug, Clone)]
pub struct DuckDuckGo;

impl ProcessFunctionality for DuckDuckGo {
    fn get_name(&self) -> &str {
        os_pick("DuckDuckGo.exe", "duckduckgo", "DuckDuckGo")
    }
}

impl BrowserFunctionality for DuckDuckGo {
    fn get_store(&self) -> BrowserStore {
        BrowserStore::AMO
    }
}

#[derive(Debug, Clone)]
pub struct Falkon;

impl ProcessFunctionality for Falkon {
    fn get_name(&self) -> &str {
        os_pick("falkon.exe", "falkon", "Falkon")
    }
}

impl BrowserFunctionality for Falkon {
    fn get_store(&self) -> BrowserStore {
        BrowserStore::AMO
    }
}

#[derive(Debug, Clone)]
pub struct Midori;

impl ProcessFunctionality for Midori {
    fn get_name(&self) -> &str {
        os_pick("midori.exe", "midori", "Midori")
    }
}

impl BrowserFunctionality for Midori {
    fn get_store(&self) -> BrowserStore {
        BrowserStore::AMO
    }
}

#[derive(Debug, Clone)]
pub struct SeaMonkey;

impl ProcessFunctionality for SeaMonkey {
    fn get_name(&self) -> &str {
        os_pick("seamonkey.exe", "seamonkey", "SeaMonkey")
    }
}

impl BrowserFunctionality for SeaMonkey {
    fn get_store(&self) -> BrowserStore {
        BrowserStore::AMO
    }
}
