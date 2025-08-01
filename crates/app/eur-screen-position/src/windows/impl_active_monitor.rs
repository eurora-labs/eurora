use super::util::find_cursor_monitor;
use crate::{CursorMonitorResult, MonitorInfo};
use tauri::LogicalSize;
use xcap::Monitor;

#[derive(Debug, Clone)]
pub struct ImplActiveMonitor {
    info: MonitorInfo,
}

impl ImplActiveMonitor {
    pub fn new(info: MonitorInfo) -> Self {
        Self { info }
    }

    pub fn calculate_position_for_percentage(
        &self,
        size: LogicalSize<u32>,
        x_percentage: f64,
        y_percentage: f64,
    ) -> (i32, i32) {
        let monitor_width = self.info.width as f64 * x_percentage;
        let monitor_height = self.info.height as f64 * y_percentage;

        let x = self.info.x + monitor_width as i32 - size.width as i32;
        let y = self.info.y + monitor_height as i32 - size.height as i32;
        (x, y)
    }

    /// Get all available monitors
    pub fn get_all_monitors() -> Result<Vec<MonitorInfo>, String> {
        let monitors = Monitor::all().map_err(|e| format!("Failed to get monitors: {}", e))?;

        let monitor_infos = monitors
            .into_iter()
            .map(|monitor| {
                let monitor_id = monitor.id().unwrap_or_default().to_string();
                let scale_factor = monitor.scale_factor().unwrap_or(1.0) as f64;
                let monitor_width = (monitor.width().unwrap_or(1920) as f64 * scale_factor) as u32;
                let monitor_height =
                    (monitor.height().unwrap_or(1080) as f64 * scale_factor) as u32;
                let monitor_x = (monitor.x().unwrap_or(0) as f64 * scale_factor) as i32;
                let monitor_y = (monitor.y().unwrap_or(0) as f64 * scale_factor) as i32;

                MonitorInfo {
                    id: monitor_id,
                    x: monitor_x,
                    y: monitor_y,
                    width: monitor_width,
                    height: monitor_height,
                    scale_factor,
                }
            })
            .collect();

        Ok(monitor_infos)
    }

    /// Find the primary monitor
    pub fn get_primary_monitor() -> Option<MonitorInfo> {
        let monitors = Monitor::all().ok()?;

        // Try to find the primary monitor (usually the one at 0,0)
        for monitor in &monitors {
            if monitor.x().unwrap_or(0) == 0 && monitor.y().unwrap_or(0) == 0 {
                let monitor_id = monitor.id().unwrap_or_default().to_string();
                let scale_factor = monitor.scale_factor().unwrap_or(1.0) as f64;
                let monitor_width = (monitor.width().unwrap_or(1920) as f64 * scale_factor) as u32;
                let monitor_height =
                    (monitor.height().unwrap_or(1080) as f64 * scale_factor) as u32;
                let monitor_x = (monitor.x().unwrap_or(0) as f64 * scale_factor) as i32;
                let monitor_y = (monitor.y().unwrap_or(0) as f64 * scale_factor) as i32;

                return Some(MonitorInfo {
                    id: monitor_id,
                    x: monitor_x,
                    y: monitor_y,
                    width: monitor_width,
                    height: monitor_height,
                    scale_factor,
                });
            }
        }

        // If no monitor at 0,0, return the first one
        monitors.first().map(|monitor| {
            let monitor_id = monitor.id().unwrap_or_default().to_string();
            let scale_factor = monitor.scale_factor().unwrap_or(1.0) as f64;
            let monitor_width = (monitor.width().unwrap_or(1920) as f64 * scale_factor) as u32;
            let monitor_height = (monitor.height().unwrap_or(1080) as f64 * scale_factor) as u32;
            let monitor_x = (monitor.x().unwrap_or(0) as f64 * scale_factor) as i32;
            let monitor_y = (monitor.y().unwrap_or(0) as f64 * scale_factor) as i32;

            MonitorInfo {
                id: monitor_id,
                x: monitor_x,
                y: monitor_y,
                width: monitor_width,
                height: monitor_height,
                scale_factor,
            }
        })
    }
}

impl Default for ImplActiveMonitor {
    /// Create an ImplActiveMonitor for the monitor containing the cursor
    /// Falls back to primary monitor if cursor position cannot be determined
    fn default() -> Self {
        // Try to get cursor position from a dummy position (0,0) to find current monitor
        if let Some(cursor_monitor) = find_cursor_monitor(tauri::PhysicalPosition::new(0.0, 0.0)) {
            Self::new(cursor_monitor.monitor)
        } else if let Some(primary_monitor) = ImplActiveMonitor::get_primary_monitor() {
            Self::new(primary_monitor)
        } else {
            // Fallback to a default monitor if nothing else works
            Self::new(MonitorInfo {
                id: "default".to_string(),
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
                scale_factor: 1.0,
            })
        }
    }
}
