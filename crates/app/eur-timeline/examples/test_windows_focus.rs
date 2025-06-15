//! Test program for Windows focus tracking
//!
//! Run with: cargo run --example test_windows_focus

use ferrous_focus::{FerrousFocusResult, FocusTracker, FocusedWindow};
fn main() -> anyhow::Result<()> {
    info!("Starting Windows focus tracker test...");
    info!("Switch between different applications to see focus events.");
    info!("Press Ctrl+C to exit.");

    let tracker = FocusTracker::new();

    tracker.track_focus(|event: FocusedWindow| -> FerrousFocusResult<()> {
        info!("Focus changed:");
        info!("  Process: {}", event.process_name.unwrap());
        info!("  Title: {}", event.window_title.unwrap());
        info!(
            "  Icon: {}",
            if event.icon.is_some() {
                "Icon available"
            } else {
                "No icon"
            }
        );
        info!("  ---");
        Ok(())
    })?;

    Ok(())
}
