use cfg_if::cfg_if;

#[inline(always)]
pub fn os_pick<'a>(_windows: &'a str, _linux: &'a str, _macos: &'a str) -> &'a str {
    cfg_if! {
        if #[cfg(target_os = "windows")] { _windows }
        else if #[cfg(target_os = "linux")] { _linux }
        else if #[cfg(target_os = "macos")] { _macos }
        else { compile_error!("Unsupported target OS"); }
    }
}

pub trait ProcessFunctionality {
    fn get_name(&self) -> &str;
}

#[derive(Debug, Clone)]
pub struct Eurora;

impl ProcessFunctionality for Eurora {
    fn get_name(&self) -> &str {
        match cfg!(debug_assertions) {
            true => os_pick("euro-tauri.exe", "euro-tauri", "euro-tauri"),
            false => os_pick("eurora.exe", "eurora", "Eurora"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct EuroraNightly;

impl ProcessFunctionality for EuroraNightly {
    fn get_name(&self) -> &str {
        os_pick("eurora-nightly.exe", "eurora-nightly", "Eurora Nightly")
    }
}

#[derive(Debug, Clone)]
pub struct Librewolf;

impl ProcessFunctionality for Librewolf {
    fn get_name(&self) -> &str {
        os_pick("librewolf.exe", "librewolf", "LibreWolf")
    }
}

#[derive(Debug, Clone)]
pub struct Firefox;

impl ProcessFunctionality for Firefox {
    fn get_name(&self) -> &str {
        os_pick("firefox.exe", "firefox", "Firefox")
    }
}

#[derive(Debug, Clone)]
pub struct Chrome;

impl ProcessFunctionality for Chrome {
    fn get_name(&self) -> &str {
        os_pick("chrome.exe", "chrome", "Google Chrome")
    }
}

#[derive(Debug, Clone)]
pub struct Opera;

impl ProcessFunctionality for Opera {
    fn get_name(&self) -> &str {
        os_pick("opera.exe", "opera", "Opera")
    }
}

#[derive(Debug, Clone)]
pub struct Safari;

impl ProcessFunctionality for Safari {
    fn get_name(&self) -> &str {
        os_pick("safari.exe", "safari", "Safari")
    }
}

#[derive(Debug, Clone)]
pub struct Brave;

impl ProcessFunctionality for Brave {
    fn get_name(&self) -> &str {
        os_pick("brave.exe", "brave", "Brave Browser")
    }
}

#[derive(Debug, Clone)]
pub struct Edge;

impl ProcessFunctionality for Edge {
    fn get_name(&self) -> &str {
        os_pick("msedge.exe", "msedge", "Microsoft Edge")
    }
}

#[derive(Debug, Clone)]
pub struct Vivaldi;

impl ProcessFunctionality for Vivaldi {
    fn get_name(&self) -> &str {
        os_pick("vivaldi.exe", "vivaldi-bin", "Vivaldi")
    }
}

#[derive(Debug, Clone)]
pub struct ArcBrowser;

impl ProcessFunctionality for ArcBrowser {
    fn get_name(&self) -> &str {
        os_pick("Arc.exe", "arc", "Arc")
    }
}

#[derive(Debug, Clone)]
pub struct TorBrowser;

impl ProcessFunctionality for TorBrowser {
    fn get_name(&self) -> &str {
        os_pick("tor.exe", "tor-browser", "Tor Browser")
    }
}

#[derive(Debug, Clone)]
pub struct Chromium;

impl ProcessFunctionality for Chromium {
    fn get_name(&self) -> &str {
        os_pick("chromium.exe", "chromium", "Chromium")
    }
}

#[derive(Debug, Clone)]
pub struct Waterfox;

impl ProcessFunctionality for Waterfox {
    fn get_name(&self) -> &str {
        os_pick("waterfox.exe", "waterfox", "Waterfox")
    }
}

#[derive(Debug, Clone)]
pub struct PaleMoon;

impl ProcessFunctionality for PaleMoon {
    fn get_name(&self) -> &str {
        os_pick("palemoon.exe", "palemoon", "Pale Moon")
    }
}

#[derive(Debug, Clone)]
pub struct Zen;

impl ProcessFunctionality for Zen {
    fn get_name(&self) -> &str {
        os_pick("zen.exe", "zen", "Zen Browser")
    }
}

#[derive(Debug, Clone)]
pub struct DuckDuckGo;

impl ProcessFunctionality for DuckDuckGo {
    fn get_name(&self) -> &str {
        os_pick("DuckDuckGo.exe", "duckduckgo", "DuckDuckGo")
    }
}

#[derive(Debug, Clone)]
pub struct Min;

impl ProcessFunctionality for Min {
    fn get_name(&self) -> &str {
        os_pick("Min.exe", "min", "Min")
    }
}

#[derive(Debug, Clone)]
pub struct Falkon;

impl ProcessFunctionality for Falkon {
    fn get_name(&self) -> &str {
        os_pick("falkon.exe", "falkon", "Falkon")
    }
}

#[derive(Debug, Clone)]
pub struct Midori;

impl ProcessFunctionality for Midori {
    fn get_name(&self) -> &str {
        os_pick("midori.exe", "midori", "Midori")
    }
}

#[derive(Debug, Clone)]
pub struct SeaMonkey;

impl ProcessFunctionality for SeaMonkey {
    fn get_name(&self) -> &str {
        os_pick("seamonkey.exe", "seamonkey", "SeaMonkey")
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
