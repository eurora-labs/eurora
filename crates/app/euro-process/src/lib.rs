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
pub struct Safari;

impl ProcessFunctionality for Safari {
    fn get_name(&self) -> &str {
        os_pick("safari.exe", "safari", "Safari")
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
