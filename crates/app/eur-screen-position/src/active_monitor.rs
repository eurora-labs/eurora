use tauri::PhysicalSize;

use crate::{MonitorInfo, platform::ImplActiveMonitor};

/// ActiveMonitor represents the currently active monitor (based on cursor position)
/// and provides generalized positioning methods for that monitor
pub struct ActiveMonitor {
    impl_active_monitor: ImplActiveMonitor,
}

impl ActiveMonitor {
    /// Create a new ActiveMonitor for a specific monitor
    pub fn new() -> Self {
        Self {
            impl_active_monitor: ImplActiveMonitor::default(),
        }
    }

    pub fn calculate_position_for_percentage(
        &self,
        size: PhysicalSize<u32>,
        x_percentage: f64,
        y_percentage: f64,
    ) -> (i32, i32) {
        self.impl_active_monitor
            .calculate_position_for_percentage(size, x_percentage, y_percentage)
    }

    pub fn get_info(&self) -> &MonitorInfo {
        self.impl_active_monitor.get_info()
    }

    pub fn convert_absolute_position_to_relative(&self, x: i32, y: i32) -> (i32, i32) {
        self.impl_active_monitor
            .convert_absolute_position_to_relative(x, y)
    }
}

impl Default for ActiveMonitor {
    /// Create an ActiveMonitor for the monitor containing the cursor
    /// Falls back to primary monitor if cursor position cannot be determined
    fn default() -> Self {
        Self::new()
    }
}
