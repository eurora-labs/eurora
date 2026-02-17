use std::{fs, path::Path, time::Instant};

use anyhow::{Result, anyhow};
use euro_vision::capture_monitor_region;
use tracing::debug;
use xcap::Monitor;

fn main() -> Result<()> {
    let screenshot_dir = Path::new("examples/screenshots");
    if !screenshot_dir.exists() {
        fs::create_dir_all(screenshot_dir)?;
    }

    debug!("Running region capture...");
    let start = Instant::now();

    let monitor = Monitor::all()?
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("No monitors found"))?;

    let width = monitor.width().unwrap() as i32;
    let height = monitor.height().unwrap() as i32;
    let start_x = width / 4;

    let image = capture_monitor_region(monitor, start_x as u32, 0, width as u32, height as u32)?;
    let duration = start.elapsed();

    debug!("Region capture completed in: {:?}", duration);
    debug!(
        "Captured image dimensions: {}x{}",
        image.width(),
        image.height()
    );

    let filename = screenshot_dir.join("region_capture.png");
    image.save(&filename)?;
    debug!("Region image saved to: {}", filename.display());

    debug!("-----------------------------------");
    debug!("Region capture completed successfully!");

    Ok(())
}
