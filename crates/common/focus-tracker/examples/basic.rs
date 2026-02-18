//! Basic example showing the simplest setup to get focus-tracker running
//!
//! This example demonstrates the minimal code needed to track focus changes.
//! It uses the default configuration and the convenient subscribe_focus_changes API.
//!
//! Usage: cargo run --example basic

use focus_tracker::subscribe_focus_changes;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    println!("🔍 Starting basic focus tracking example...");
    println!("   Switch between different applications to see focus changes.");
    println!("   Press Ctrl+C to exit.");
    println!();

    let subscription = subscribe_focus_changes()?;

    let running = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        println!("\n👋 Received Ctrl+C, shutting down...");
        r.store(false, std::sync::atomic::Ordering::SeqCst);
    })?;

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

                let icon_status = if focused_window.icon.is_some() {
                    "✅ Has icon"
                } else {
                    "❌ No icon"
                };
                println!("   Icon: {}", icon_status);
                println!();
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
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
