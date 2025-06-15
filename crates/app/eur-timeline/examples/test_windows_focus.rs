//! Test program for Windows focus tracking
//!
//! Run with: cargo run --example test_windows_focus

use ferrous_focus::{FerrousFocusResult, FocusTracker, FocusedWindow};
fn main() -> anyhow::Result<()> {
    println!("Starting Windows focus tracker test...");
    println!("Switch between different applications to see focus events.");
    println!("Press Ctrl+C to exit.");

    let tracker = FocusTracker::new();

    tracker.track_focus(|event: FocusedWindow| -> FerrousFocusResult<()> {
        println!("Focus changed:");
        println!("  Process: {}", event.process_name.unwrap());
        println!("  Title: {}", event.window_title.unwrap());
        println!(
            "  Icon: {}",
            if event.icon.is_some() {
                "Icon available"
            } else {
                "No icon"
            }
        );
        println!("  ---");
        Ok(())
    })?;

    Ok(())
}
