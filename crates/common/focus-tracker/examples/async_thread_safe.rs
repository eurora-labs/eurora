//! Example demonstrating that the async focus tracker is now Send-safe
//! and can be used in tokio::spawn, which was previously failing due to
//! non-Send HWND (*mut c_void) types in the Windows implementation.

use focus_tracker::{FocusTracker, FocusTrackerConfig, IconConfig};
use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for logging
    tracing_subscriber::fmt::init();

    println!("Starting async thread-safe focus tracker example...");
    println!("This demonstrates that the tracker can be spawned in tokio::spawn");
    println!("Press Ctrl+C to exit\n");

    let config = FocusTrackerConfig::builder()
        .icon(IconConfig::builder().size(64)?.build())
        .poll_interval(std::time::Duration::from_millis(500))?
        .build();

    let tracker = FocusTracker::with_config(config);

    // Use Arc<Mutex<>> to share state between tasks
    let focus_count = Arc::new(Mutex::new(0u32));
    let focus_count_clone = Arc::clone(&focus_count);

    // This is the critical test: spawning the tracker in tokio::spawn
    // This would previously fail with: `*mut c_void` cannot be sent between threads safely
    let handle = tokio::spawn(async move {
        tracker
            .track_focus_async(move |window| {
                let focus_count = Arc::clone(&focus_count_clone);
                async move {
                    let mut count = focus_count.lock().await;
                    *count += 1;

                    println!("--- Focus Change #{} ---", *count);
                    println!("Process: {}", window.process_name);
                    if let Some(title) = &window.window_title {
                        println!("Title: {}", title);
                    }

                    println!("PID: {}", window.process_id);

                    if let Some(icon) = &window.icon {
                        println!("Icon: {}x{}", icon.width(), icon.height());
                    }
                    println!();

                    Ok(())
                }
            })
            .await
    });

    // Wait for a bit to show it's working
    tokio::time::sleep(std::time::Duration::from_secs(10)).await;

    // In a real application, you'd have a graceful shutdown mechanism
    println!("Example completed successfully!");
    println!("The focus tracker is now Send-safe and works in tokio::spawn!");

    // Abort the handle since we're done
    handle.abort();

    Ok(())
}
