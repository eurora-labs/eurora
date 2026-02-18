//! Async Focus Tracker Example
//!
//! This example demonstrates how to use the async focus tracking capabilities
//! to perform async operations when focus changes occur.
//!
//! To run this example:
//! ```bash
//! cargo run --example async --features async
//! ```

#[cfg(feature = "async")]
use focus_tracker::{FocusTracker, FocusTrackerResult};
#[cfg(feature = "async")]
use std::sync::Arc;
#[cfg(feature = "async")]
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(feature = "async")]
use std::time::Duration;

#[cfg(feature = "async")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    println!("🚀 Starting async focus tracker example with stop signal...");
    println!("This example demonstrates awaiting async operations in focus callbacks.");
    println!("It will automatically stop after 10 seconds.");
    println!("Switch between different applications to see focus changes.\n");

    let tracker = FocusTracker::new();

    let stop_signal = Arc::new(AtomicBool::new(false));

    let stop_signal_timeout = Arc::clone(&stop_signal);
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(10)).await;
        println!("\n⏰ 10 second timeout reached, stopping gracefully...");
        stop_signal_timeout.store(true, Ordering::Release);
    });

    tracker
        .track_focus_async_with_stop(
            |window| async move {
                println!(
                    "🔍 Focus changed to: {}",
                    window.window_title.as_deref().unwrap_or("Unknown")
                );

                println!("   📱 Process: {}", window.process_name);

                let icon_status = if window.icon.is_some() {
                    "✅ Has icon"
                } else {
                    "❌ No icon"
                };
                println!("   Icon: {}", icon_status);

                println!("   ⏳ Performing async processing...");
                simulate_async_processing(&window).await?;

                println!("   🔢 Processing window data asynchronously...");
                process_window_data(&window).await?;

                println!("   ✨ All async operations complete!\n");

                Ok(())
            },
            &stop_signal,
        )
        .await?;

    println!("\n👋 Async focus tracking completed gracefully!");
    Ok(())
}

/// Simulate async processing that might involve network or computation
#[cfg(feature = "async")]
async fn simulate_async_processing(
    window: &focus_tracker::FocusedWindow,
) -> FocusTrackerResult<()> {
    tokio::time::sleep(Duration::from_millis(50)).await;

    println!(
        "   🔄 [ASYNC] Processed focus event for: {}",
        window.process_name
    );

    Ok(())
}

/// Simulate async data processing
#[cfg(feature = "async")]
async fn process_window_data(window: &focus_tracker::FocusedWindow) -> FocusTrackerResult<()> {
    tokio::time::sleep(Duration::from_millis(30)).await;

    let title_length = window.window_title.as_ref().map(|t| t.len()).unwrap_or(0);
    let process_length = window.process_name.len();

    println!(
        "   📊 [DATA] Title length: {}, Process length: {}, Has icon: {}",
        title_length,
        process_length,
        window.icon.is_some()
    );

    Ok(())
}

#[cfg(not(feature = "async"))]
fn main() {
    println!("This example requires the 'async' feature to be enabled.");
    println!("Run with: cargo run --example async --features async");
}
