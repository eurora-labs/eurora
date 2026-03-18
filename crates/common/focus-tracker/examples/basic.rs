//! Basic example showing the simplest setup to get focus-tracker running
//!
//! This example demonstrates the minimal code needed to track focus changes.
//!
//! Usage: cargo run --example basic

use focus_tracker::FocusTracker;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    println!("🔍 Starting basic focus tracking example...");
    println!("   Switch between different applications to see focus changes.");
    println!("   Press Ctrl+C to exit.");
    println!();

    let tracker = FocusTracker::new();
    let stop_signal = Arc::new(AtomicBool::new(false));

    let r = Arc::clone(&stop_signal);
    ctrlc::set_handler(move || {
        println!("\n👋 Received Ctrl+C, shutting down...");
        r.store(true, Ordering::Release);
    })?;

    let mut event_count = 0u64;
    tracker
        .track_focus_with_stop(
            |focused_window| {
                event_count += 1;
                let count = event_count;
                async move {
                    println!(
                        "📱 Focus Event #{}: {}",
                        count,
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
                    Ok(())
                }
            },
            &stop_signal,
        )
        .await?;

    println!("📊 Total focus events captured: {}", event_count);
    println!("✨ Basic example completed!");

    Ok(())
}
