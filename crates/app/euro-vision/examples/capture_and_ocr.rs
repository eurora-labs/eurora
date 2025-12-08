use std::{fs, path::Path, time::Instant};

use anyhow::Result;
use euro_ocr::{self, OcrStrategy};
use euro_vision::capture_all_monitors;
use tracing::debug;

fn main() -> Result<()> {
    // Create screenshots directory if it doesn't exist
    let screenshot_dir = Path::new("examples/screenshots");
    if !screenshot_dir.exists() {
        fs::create_dir_all(screenshot_dir)?;
    }
    debug!("Running multi-monitor capture method...");
    let start = Instant::now();

    // Capture all monitors
    let images = capture_all_monitors()?;
    let tess = euro_ocr::TesseractOcr {};

    for image in &images {
        // TODO: remove this code
        let result_text = tess.recognize(&image::DynamicImage::ImageRgba8(image.clone()));
        debug!("Recognized text: {}", result_text);
    }

    let duration = start.elapsed();
    debug!("Multi-monitor capture completed in: {:?}", duration);
    debug!("Number of monitors captured: {}", images.len());

    // Save each captured image
    for (i, image) in images.iter().enumerate() {
        let filename = screenshot_dir.join(format!("monitor_{}.png", i));
        image.save(&filename)?;
        debug!("Monitor {} image saved to: {}", i, filename.display());
    }

    debug!("All capture methods completed successfully!");
    Ok(())
}
