use xcap::Monitor;

use crate::{CursorMonitorResult, MonitorInfo};

pub fn find_cursor_monitor(
    cursor_position: tauri::PhysicalPosition<f64>,
) -> Option<CursorMonitorResult> {
    let monitors = Monitor::all().ok()?;

    for monitor in monitors {
        let monitor_info = MonitorInfo::from(&monitor);

        if cursor_position.x >= monitor_info.x as f64
            && cursor_position.x <= (monitor_info.x + monitor_info.width as i32) as f64
            && cursor_position.y >= monitor_info.y as f64
            && cursor_position.y <= (monitor_info.y + monitor_info.height as i32) as f64
        {
            return Some(CursorMonitorResult {
                monitor: monitor_info,
                cursor_x: cursor_position.x,

                cursor_y: cursor_position.y,
            });
        }
    }
    None
}
