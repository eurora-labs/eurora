use std::{fs, path::Path, time::Instant};

use anyhow::Result;
use euro_vision::{capture_all_monitors, capture_monitor};

enum CaptureMethod {
    Basic,
    AllMonitors,
}

fn main() -> Result<()> {
    let screenshot_dir = Path::new("examples/screenshots");
    if !screenshot_dir.exists() {
        fs::create_dir_all(screenshot_dir)?;
    }

    let methods = [CaptureMethod::Basic, CaptureMethod::AllMonitors];

    for method in methods {
        match method {
            CaptureMethod::Basic => {
                tracing::debug!("Running basic capture method...");
                let start = Instant::now();

                let image = capture_monitor()?;

                let duration = start.elapsed();
                tracing::debug!("Basic capture completed in: {:?}", duration);

                let filename = screenshot_dir.join("basic_capture.png");
                image.save(&filename)?;
                tracing::debug!("Image saved to: {}", filename.display());
            }

            CaptureMethod::AllMonitors => {
                tracing::debug!("Running multi-monitor capture method...");
                let start = Instant::now();

                let images = capture_all_monitors()?;

                let duration = start.elapsed();
                tracing::debug!("Multi-monitor capture completed in: {:?}", duration);
                tracing::debug!("Number of monitors captured: {}", images.len());

                for (i, image) in images.iter().enumerate() {
                    let filename = screenshot_dir.join(format!("monitor_{}.png", i));
                    image.save(&filename)?;
                    tracing::debug!("Monitor {} image saved to: {}", i, filename.display());
                }
            }
        }

        tracing::debug!("-----------------------------------");
    }

    tracing::debug!("All capture methods completed successfully!");
    Ok(())
}
