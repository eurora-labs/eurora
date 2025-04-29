use anyhow::Result;
use eur_vision::{capture_all_monitors, capture_monitor};
use std::{fs, path::Path, time::Instant};

// Different capture methods to demonstrate and benchmark
enum CaptureMethod {
    Basic,       // Basic capture using capture_monitor
    AllMonitors, // Capture all monitors
}

fn main() -> Result<()> {
    // Create screenshots directory if it doesn't exist
    let screenshot_dir = Path::new("examples/screenshots");
    if !screenshot_dir.exists() {
        fs::create_dir_all(screenshot_dir)?;
    }

    // Define which capture methods to run
    let methods = [CaptureMethod::Basic, CaptureMethod::AllMonitors];

    for method in methods {
        match method {
            CaptureMethod::Basic => {
                println!("Running basic capture method...");
                let start = Instant::now();

                // Perform basic capture
                let image = capture_monitor()?;

                let duration = start.elapsed();
                println!("Basic capture completed in: {:?}", duration);

                // Save the captured image
                let filename = screenshot_dir.join("basic_capture.png");
                image.save(&filename)?;
                println!("Image saved to: {}", filename.display());
            }

            CaptureMethod::AllMonitors => {
                println!("Running multi-monitor capture method...");
                let start = Instant::now();

                // Capture all monitors
                let images = capture_all_monitors()?;

                let duration = start.elapsed();
                println!("Multi-monitor capture completed in: {:?}", duration);
                println!("Number of monitors captured: {}", images.len());

                // Save each captured image
                for (i, image) in images.iter().enumerate() {
                    let filename = screenshot_dir.join(format!("monitor_{}.png", i));
                    image.save(&filename)?;
                    println!("Monitor {} image saved to: {}", i, filename.display());
                }
            }
        }

        println!("-----------------------------------");
    }

    println!("All capture methods completed successfully!");
    Ok(())
}
