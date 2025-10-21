use cfg_if::cfg_if;
use enum_dispatch::enum_dispatch;

#[inline(always)]
fn os_pick<'a>(_windows: &'a str, _linux: &'a str, _macos: &'a str) -> &'a str {
    cfg_if! {
        if #[cfg(target_os = "windows")] { _windows }
        else if #[cfg(target_os = "linux")] { _linux }
        else if #[cfg(target_os = "macos")] { _macos }
        else { compile_error!("Unsupported target OS"); }
    }
}

#[enum_dispatch(ProcessFunctionality)]
pub enum SupportedProcesses {
    ProcessLibrewolf,
}

#[enum_dispatch]
pub trait ProcessFunctionality {
    fn get_name(&self) -> &str;
}

pub struct ProcessLibrewolf;

impl ProcessFunctionality for ProcessLibrewolf {
    fn get_name(&self) -> &str {
        os_pick("librewolf.exe", "librewolf", "LibreWolf")
    }
}

// Implement a test
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_name() {
        let process = ProcessLibrewolf;
        assert_eq!(
            process.get_name(),
            os_pick("librewolf.exe", "librewolf", "LibreWolf")
        );
    }
}
