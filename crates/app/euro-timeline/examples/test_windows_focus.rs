//! Test program for Windows focus tracking
//!
//! Run with: cargo run --example test_windows_focus

use euro_focus::{FerrousFocusResult, FocusTracker, FocusedWindow};
use tracing::debug;

fn main() -> anyhow::Result<()> {
    debug!("Starting Windows focus tracker test...");
    debug!("Switch between different applications to see focus events.");
    debug!("Press Ctrl+C to exit.");

    let tracker = FocusTracker::new();

    tracker.track_focus(|event: FocusedWindow| -> FerrousFocusResult<()> {
        debug!("Focus changed:");
        debug!("  Process: {}", event.process_name.unwrap());
        debug!("  Title: {}", event.window_title.unwrap());
        debug!(
            "  Icon: {}",
            if event.icon.is_some() {
                "Icon available"
            } else {
                "No icon"
            }
        );
        debug!("  ---");
        Ok(())
    })?;

    Ok(())
}
