use crate::{CursorMonitorResult, MonitorInfo};
use xcap::Monitor;

#[derive(Debug, Clone)]
pub struct ImplActiveMonitor;

impl ImplActiveMonitor {
    pub fn get_middle_for_size(size: tauri::LogicalSize<u32>) -> (u32, u32) {
        let monitor_result =
            Self::find_cursor_monitor(tauri::PhysicalPosition::new(0.0, 0.0)).unwrap();
        let monitor = monitor_result.monitor;
        let window_x = monitor.x + (monitor.width as i32 - size.width as i32) / 2;
        let window_y = monitor.y + (monitor.height as i32 - size.height as i32) / 4;
        (window_x as u32, window_y as u32)
    }

    pub fn find_cursor_monitor(
        cursor_position: tauri::PhysicalPosition<f64>,
    ) -> Option<CursorMonitorResult> {
        let monitors = Monitor::all().ok()?;

        for monitor in monitors {
            let monitor_id = monitor.id().unwrap_or_default().to_string();
            let scale_factor = monitor.scale_factor().unwrap_or(1.0) as f64;
            let monitor_width = (monitor.width().unwrap_or(1920) as f64 * scale_factor) as u32;
            let monitor_height = (monitor.height().unwrap_or(1080) as f64 * scale_factor) as u32;
            let monitor_x = (monitor.x().unwrap_or(0) as f64 * scale_factor) as i32;
            let monitor_y = (monitor.y().unwrap_or(0) as f64 * scale_factor) as i32;

            // Check if cursor is on this monitor
            if cursor_position.x >= monitor_x as f64
                && cursor_position.x <= (monitor_x + monitor_width as i32) as f64
                && cursor_position.y >= monitor_y as f64
                && cursor_position.y <= (monitor_y + monitor_height as i32) as f64
            {
                return Some(CursorMonitorResult {
                    monitor: MonitorInfo {
                        id: monitor_id,
                        x: monitor_x,
                        y: monitor_y,
                        width: monitor_width,
                        height: monitor_height,
                        scale_factor,
                    },
                    cursor_x: cursor_position.x,
                    cursor_y: cursor_position.y,
                });
            }
        }
        None
    }
}
