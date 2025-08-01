use tauri::LogicalSize;

use crate::platform::ImplActiveMonitor;

pub struct ActiveMonitor {
    impl_active_monitor: ImplActiveMonitor,
}

impl ActiveMonitor {
    fn get_middle_for_size(size: LogicalSize<u32>) -> (u32, u32) {
        ImplActiveMonitor::get_middle_for_size(size)
    }
}
