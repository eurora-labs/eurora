use anyhow::Result;
use eur_ocr::{self, OcrStrategy};
use eur_vision::capture_all_monitors;
use std::{fs, path::Path, time::Instant};
use tracing::info;
fn main() -> Result<()> {
    // Create screenshots directory if it doesn't exist
    let screenshot_dir = Path::new("examples/screenshots");
    if !screenshot_dir.exists() {
        fs::create_dir_all(screenshot_dir)?;
    }
    info!("Running multi-monitor capture method...");
    let start = Instant::now();

    // Capture all monitors
    let images = capture_all_monitors()?;
    let tess = eur_ocr::TesseractOcr {};

    for image in &images {
        // TODO: remove this code
        let result_text = tess.recognize(&image::DynamicImage::ImageRgba8(image.clone()));
        info!("Recognized text: {}", result_text);
    }

    let duration = start.elapsed();
    info!("Multi-monitor capture completed in: {:?}", duration);
    info!("Number of monitors captured: {}", images.len());

    // Save each captured image
    for (i, image) in images.iter().enumerate() {
        let filename = screenshot_dir.join(format!("monitor_{}.png", i));
        image.save(&filename)?;
        info!("Monitor {} image saved to: {}", i, filename.display());
    }

    info!("All capture methods completed successfully!");
    Ok(())
}
