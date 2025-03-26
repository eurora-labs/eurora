use anyhow::{Context, Result};
use image::DynamicImage;
use std::path::{Path, PathBuf};
use tracing::debug;
use xcap::Monitor;

/// Represents a captured screenshot
pub struct Screenshot {
    pub image: DynamicImage,
    pub width: u32,
    pub height: u32,
    pub monitor_name: String,
}

impl Screenshot {
    /// Save the screenshot to a file
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        self.image.save(&path)?;
        Ok(())
    }
}

/// Capture the primary monitor
pub fn capture_primary_monitor() -> Result<Screenshot> {
    let monitor = Monitor::all()?
        .into_iter()
        .find(|m| m.is_primary())
        .context("Could not find primary monitor")?;

    capture_monitor(&monitor)
}

/// Capture all available monitors
pub fn capture_all_monitors() -> Result<Vec<Screenshot>> {
    let monitors = Monitor::all()?;

    monitors.iter().map(capture_monitor).collect()
}

/// Capture a specific monitor by index (0-based)
pub fn capture_monitor_by_index(index: usize) -> Result<Screenshot> {
    let monitors = Monitor::all()?;

    if index >= monitors.len() {
        anyhow::bail!(
            "Monitor index out of bounds: {}, max index is {}",
            index,
            monitors.len() - 1
        );
    }

    capture_monitor(&monitors[index])
}

/// Capture a specific monitor
fn capture_monitor(monitor: &Monitor) -> Result<Screenshot> {
    debug!("Capturing monitor: {}", monitor.name());

    let img = monitor.capture_image()?;

    Ok(Screenshot {
        image: img.into(),
        width: monitor.width(),
        height: monitor.height(),
        monitor_name: monitor.name().to_string(),
    })
}

/// List all available monitors
pub fn list_monitors() -> Result<Vec<MonitorInfo>> {
    let monitors = Monitor::all()?;

    Ok(monitors
        .iter()
        .enumerate()
        .map(|(idx, m)| MonitorInfo {
            index: idx,
            name: m.name().to_string(),
            width: m.width(),
            height: m.height(),
            is_primary: m.is_primary(),
        })
        .collect())
}

/// Information about a monitor
#[derive(Debug, Clone)]
pub struct MonitorInfo {
    pub index: usize,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub is_primary: bool,
}

/// Generate a filename for a screenshot
pub fn generate_filename(prefix: &str, monitor_name: &str) -> PathBuf {
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
    let sanitized_name = monitor_name.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_");

    PathBuf::from(format!("{prefix}_{sanitized_name}_{timestamp}.png"))
}
