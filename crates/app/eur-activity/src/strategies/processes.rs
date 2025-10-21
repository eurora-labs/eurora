use crate::utils::os_pick;
use enum_dispatch::enum_dispatch;
use serde::{Deserialize, Serialize};

#[enum_dispatch(ProcessFunctionality)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SupportedProcesses {
    Librewolf,
    Chrome,
    Firefox,
}

#[enum_dispatch]
pub trait ProcessFunctionality {
    fn get_name(&self) -> &str;
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
