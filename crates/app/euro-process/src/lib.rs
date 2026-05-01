use cfg_if::cfg_if;

mod browser;

pub use browser::*;

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
