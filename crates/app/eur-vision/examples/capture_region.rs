use anyhow::Result;
use eur_vision::capture_region;
use std::time::Instant;
use std::{fs, path::Path};

fn main() -> Result<()> {
    // Create screenshots directory if it doesn't exist
    let screenshot_dir = Path::new("examples/screenshots");
    if !screenshot_dir.exists() {
        fs::create_dir_all(screenshot_dir)?;
    }

    println!("Running region capture...");

    let start = Instant::now();
    let image = capture_region()?;
    let duration = start.elapsed();

    println!("Region capture completed in: {:?}", duration);
    println!(
        "Captured image dimensions: {}x{}",
        image.width(),
        image.height()
    );

    // Save the captured image
    let filename = screenshot_dir.join("region_capture.png");
    image.save(&filename)?;
    println!("Region image saved to: {}", filename.display());

    println!("-----------------------------------");
    println!("Region capture completed successfully!");

    Ok(())
}
