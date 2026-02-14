//! Basic example showing the simplest setup to get focus-tracker running
//!
//! This example demonstrates the minimal code needed to track focus changes.
//! It uses the default configuration and the convenient subscribe_focus_changes API.
//!
//! Usage: cargo run --example basic

use focus_tracker::subscribe_focus_changes;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging to see what's happening
    tracing_subscriber::fmt::init();

    println!("🔍 Starting basic focus tracking example...");
    println!("   Switch between different applications to see focus changes.");
    println!("   Press Ctrl+C to exit.");
    println!();

    // This is the simplest way to get focus changes - just one function call!
    let subscription = subscribe_focus_changes()?;

    // Set up Ctrl+C handler for graceful shutdown
    let running = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        println!("\n👋 Received Ctrl+C, shutting down...");
        r.store(false, std::sync::atomic::Ordering::SeqCst);
    })?;

    // Listen for focus changes in a simple loop
    let mut event_count = 0;
    while running.load(std::sync::atomic::Ordering::SeqCst) {
        match subscription
            .receiver()
            .recv_timeout(Duration::from_millis(100))
        {
            Ok(focused_window) => {
                event_count += 1;
                println!(
                    "📱 Focus Event #{}: {}",
                    event_count,
                    focused_window.window_title.as_deref().unwrap_or("Unknown")
                );

                println!("   Process: {}", &focused_window.process_name);

                // Check if icon is available
                let icon_status = if focused_window.icon.is_some() {
                    "✅ Has icon"
                } else {
                    "❌ No icon"
                };
                println!("   Icon: {}", icon_status);
                println!();
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                // Continue waiting - this is normal
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                println!("📡 Focus tracking channel disconnected");
                break;
            }
        }
    }

    println!("📊 Total focus events captured: {}", event_count);
    println!("✨ Basic example completed!");

    Ok(())
}
