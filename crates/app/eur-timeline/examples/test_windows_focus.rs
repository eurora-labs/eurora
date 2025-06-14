//! Test program for Windows focus tracking
//! 
//! Run with: cargo run --example test_windows_focus

use eur_timeline::{FocusEvent, focus_tracker::FocusTracker};
use eur_timeline::platform::impl_focus_tracker::ImplFocusTracker;

fn main() -> anyhow::Result<()> {
    println!("Starting Windows focus tracker test...");
    println!("Switch between different applications to see focus events.");
    println!("Press Ctrl+C to exit.");

    let tracker = FocusTracker::new(ImplFocusTracker::new());
    
    tracker.track_focus(|event: FocusEvent| {
        println!("Focus changed:");
        println!("  Process: {}", event.process);
        println!("  Title: {}", event.title);
        println!("  Icon: {}", if event.icon_base64.is_empty() { 
            "No icon" 
        } else { 
            "Icon available" 
        });
        println!("  ---");
        Ok(())
    })?;

    Ok(())
}