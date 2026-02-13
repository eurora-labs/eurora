//! Advanced example showing all configuration options and features
//!
//! This example demonstrates:
//! - Custom FocusTrackerConfig with all available options
//! - Using track_focus_with_stop for manual control
//! - Icon extraction and saving to files
//! - Custom polling intervals and icon sizes
//! - Proper signal handling and graceful shutdown
//!
//! Usage: cargo run --example advanced

use focus_tracker::{
    FocusTracker, FocusTrackerConfig, FocusTrackerResult, FocusedWindow, IconConfig,
};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

fn save_icon_to_file(
    icon_data: &image::RgbaImage,
    filename: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Save as PNG
    icon_data.save(filename)?;
    println!("💾 Icon saved: {}", filename);
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize detailed logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    println!("🚀 Starting ADVANCED focus tracking example!");
    println!("   This demo showcases all configuration options and features.");
    println!("   Icons will be saved to examples/recorded_icons/");
    println!("   Press Ctrl+C for graceful shutdown.");
    println!();

    let config = FocusTrackerConfig::builder()
        .poll_interval(std::time::Duration::from_millis(50))?
        .icon(IconConfig::builder().size(64)?.build())
        .build();

    println!("⚙️  Configuration:");
    println!("   Poll interval: {:?}", config.poll_interval);
    println!(
        "   Icon size: {}x{}",
        config.icon.get_size_or_default(),
        config.icon.get_size_or_default()
    );
    println!();

    // Create focus tracker with custom config
    let tracker = FocusTracker::with_config(config);

    // Create stop signal for controlled shutdown
    let stop_signal = Arc::new(AtomicBool::new(false));
    let stop_signal_clone = stop_signal.clone();

    // Set up Ctrl+C handler
    ctrlc::set_handler(move || {
        println!("\n🛑 Interrupt signal received, initiating graceful shutdown...");
        stop_signal_clone.store(true, Ordering::SeqCst);
    })?;

    // Statistics tracking
    let mut event_count = 0;
    let mut icons_saved = 0;
    let mut unique_processes = std::collections::HashSet::new();
    let start_time = std::time::Instant::now();

    println!("🎯 Focus tracking active! Switch between applications...");
    println!();

    // Start advanced focus tracking with full control
    let result = tracker.track_focus_with_stop(
        |window: FocusedWindow| -> FocusTrackerResult<()> {
            event_count += 1;

            // Extract window information
            let window_title = window.window_title.as_deref().unwrap_or("Unknown");
            let process_name = window.process_name;

            // Track unique processes
            unique_processes.insert(process_name.to_string());

            println!("🔄 Focus Event #{}", event_count);
            println!("   📋 Title: {}", window_title);
            println!(
                "   ⚙️  Process: {} (PID: {:?})",
                process_name, window.process_id
            );

            // Advanced icon handling
            if let Some(icon) = window.icon {
                let (width, height) = (icon.width(), icon.height());
                println!("   🖼️  Icon: {}x{} pixels", width, height);

                // Save icon with detailed naming
                icons_saved += 1;
                let filename = format!(
                    "examples/recorded_icons/advanced_{:03}_{}.png",
                    icons_saved,
                    process_name.replace("/", "_").replace(" ", "_")
                );

                match save_icon_to_file(&icon, &filename) {
                    Ok(_) => println!("   ✅ Icon saved successfully"),
                    Err(e) => println!("   ❌ Failed to save icon: {}", e),
                }
            } else {
                println!("   🚫 No icon available");
            }

            println!("   ⏱️  Uptime: {:?}", start_time.elapsed());
            println!();

            Ok(())
        },
        &stop_signal,
    );

    // Handle results and show statistics
    match result {
        Ok(_) => println!("✅ Focus tracking completed successfully"),
        Err(e) => println!("❌ Focus tracking error: {}", e),
    }

    println!();
    println!("📊 SESSION STATISTICS:");
    println!("   Total events: {}", event_count);
    println!("   Icons saved: {}", icons_saved);
    println!("   Unique processes: {}", unique_processes.len());
    println!("   Session duration: {:?}", start_time.elapsed());
    println!(
        "   Average events/min: {:.1}",
        event_count as f64 / start_time.elapsed().as_secs_f64() * 60.0
    );

    if !unique_processes.is_empty() {
        println!("   Processes seen:");
        for (i, process) in unique_processes.iter().enumerate() {
            println!("     {}. {}", i + 1, process);
        }
    }

    println!();
    println!("🎉 Advanced example completed!");
    println!("   Check examples/recorded_icons/ for saved icons");

    Ok(())
}
