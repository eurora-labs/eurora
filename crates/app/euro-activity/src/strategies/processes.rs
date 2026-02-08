use crate::utils::os_pick;
use enum_dispatch::enum_dispatch;
use serde::{Deserialize, Serialize};

// #[enum_dispatch(ProcessFunctionality)]
// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub enum SupportedProcesses {
//     Librewolf,
//     Chrome,
//     Firefox,
// }

#[enum_dispatch]
pub trait ProcessFunctionality {
    fn get_name(&self) -> &str;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Eurora;

impl ProcessFunctionality for Eurora {
    fn get_name(&self) -> &str {
        // This process is used to ignore the application.
        // So different names are used for debug and release builds.
        match cfg!(debug_assertions) {
            true => os_pick("euro-tauri.exe", "euro-tauri", "euro-tauri"),
            false => os_pick("eurora.exe", "eurora", "Eurora"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Librewolf;

impl ProcessFunctionality for Librewolf {
    fn get_name(&self) -> &str {
        os_pick("librewolf.exe", "librewolf", "LibreWolf")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Firefox;

impl ProcessFunctionality for Firefox {
    fn get_name(&self) -> &str {
        os_pick("firefox.exe", "firefox", "Firefox")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chrome;

impl ProcessFunctionality for Chrome {
    fn get_name(&self) -> &str {
        os_pick("chrome.exe", "chrome", "Google Chrome")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Safari;

impl ProcessFunctionality for Safari {
    fn get_name(&self) -> &str {
        os_pick("safari.exe", "safari", "Safari")
    }
}

// Implement a test
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
