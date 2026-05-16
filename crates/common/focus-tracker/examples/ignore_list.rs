//! Demonstrates per-platform process ignore lists.
//!
//! The tracker exposes a separate ignore list for each platform. The current
//! platform's list filters focus events before they reach `on_focus`; lists
//! for other platforms are accepted by the builder but silently unused, so
//! the same code compiles and runs on every supported target.
//!
//! Names are matched **byte-exactly** against `FocusedWindow::process_name`
//! as emitted by the platform — `"firefox"` does not match `"firefox.exe"`.
//!
//! Usage: cargo run --example ignore_list

use focus_tracker::{FocusTracker, FocusTrackerConfig};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let config = FocusTrackerConfig::builder()
        .linux_ignored_processes(["firefox", "chrome"])
        .macos_ignored_processes(["Firefox", "Google Chrome"])
        .windows_ignored_processes(["firefox.exe", "chrome.exe"])
        .build();

    println!("🔍 Tracking focus with per-platform ignore lists.");
    println!(
        "   Active list on this platform: {:?}",
        config
            .ignored_processes_for_current_platform()
            .iter()
            .collect::<Vec<_>>()
    );
    println!("   Press Ctrl+C to exit.");
    println!();

    let tracker = FocusTracker::builder().config(config).build();
    let stop_signal = Arc::new(AtomicBool::new(false));

    let r = Arc::clone(&stop_signal);
    ctrlc::set_handler(move || {
        println!("\n👋 Received Ctrl+C, shutting down...");
        r.store(true, Ordering::Release);
    })?;

    tracker
        .track_focus()
        .on_focus(|window| async move {
            println!(
                "📱 {} — {}",
                window.process_name,
                window.window_title.as_deref().unwrap_or("Unknown")
            );
            Ok(())
        })
        .stop_signal(&stop_signal)
        .call()
        .await?;

    Ok(())
}
