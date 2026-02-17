use enigo::{Enigo, Mouse, Settings};
use tauri::PhysicalSize;
use xcap::Monitor;

use super::util::find_cursor_monitor;
use crate::MonitorInfo;

#[derive(Debug, Clone)]
pub struct ImplActiveMonitor {
    info: MonitorInfo,
}

impl ImplActiveMonitor {
    pub fn new(info: MonitorInfo) -> Self {
        Self { info }
    }

    pub fn get_info(&self) -> &MonitorInfo {
        &self.info
    }

    pub fn convert_absolute_position_to_relative(&self, x: i32, y: i32) -> (i32, i32) {
        (x - self.info.x, y - self.info.y)
    }

    pub fn calculate_position_for_percentage(
        &self,
        size: PhysicalSize<u32>,
        x_percentage: f64,
        y_percentage: f64,
    ) -> (i32, i32) {
        let monitor_width = self.info.width as f64 * x_percentage;
        let monitor_height = self.info.height as f64 * y_percentage;

        let x = self.info.x + monitor_width as i32 - size.width as i32;
        let y = self.info.y + monitor_height as i32 - size.height as i32;
        (x, y)
    }

    pub fn get_primary_monitor() -> Option<MonitorInfo> {
        let monitors = Monitor::all().ok()?;

        for monitor in &monitors {
            if monitor.x().unwrap_or(0) == 0 && monitor.y().unwrap_or(0) == 0 {
                return Some(MonitorInfo::from(monitor));
            }
        }

        None
    }
}

impl Default for ImplActiveMonitor {
    fn default() -> Self {
        let cursor_xy = Enigo::new(&Settings::default())
            .ok()
            .and_then(|e| e.location().ok());

        if let Some((cursor_x, cursor_y)) = cursor_xy
            && let Some(cursor_monitor) = find_cursor_monitor(tauri::PhysicalPosition::new(
                cursor_x as f64,
                cursor_y as f64,
            ))
        {
            return Self::new(cursor_monitor.monitor);
        }

        if let Some(primary_monitor) = ImplActiveMonitor::get_primary_monitor() {
            return Self::new(primary_monitor);
        }

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

impl From<&Monitor> for MonitorInfo {
    fn from(monitor: &Monitor) -> Self {
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
    }
}
