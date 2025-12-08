/// Monitor information for window positioning
#[derive(Debug, Clone)]
pub struct MonitorInfo {
    pub id: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub scale_factor: f64,
}

/// Result of finding the monitor containing the cursor
#[derive(Debug)]
pub struct CursorMonitorResult {
    pub monitor: MonitorInfo,
    pub cursor_x: f64,
    pub cursor_y: f64,
}
