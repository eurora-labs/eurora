//! Demonstrates per-platform ignore rules.
//!
//! The tracker exposes a separate rule set for each platform. The current
//! platform's rules filter focus events before they reach `on_focus`; rule
//! sets for other platforms are accepted by the builder but silently unused,
//! so the same code compiles and runs on every supported target.
//!
//! Each [`IgnoreRule`] combines a process-name predicate **and** a
//! window-title predicate. A focus event is suppressed when **any** rule
//! matches; a rule itself matches when **both** its predicates do.
//!
//! Matching is byte-exact and case-sensitive — `"firefox"` does not match
//! `"firefox.exe"`. Use the title predicates to distinguish e.g. a splash
//! window (no title) from the real application window (has a title).
//!
//! Usage: cargo run --example ignore_list
//!
//! [`IgnoreRule`]: focus_tracker::IgnoreRule

use focus_tracker::{FocusTracker, FocusTrackerConfig, IgnoreRule, WindowTitleMatch};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let config = FocusTrackerConfig::builder()
        .linux_ignore_rules([
            IgnoreRule::builder().process_name("firefox").build(),
            IgnoreRule::builder().process_name("chrome").build(),
            // Ignore "whatever" only when it has no title; show it when titled.
            IgnoreRule::builder()
                .process_name("whatever")
                .window_title(WindowTitleMatch::Missing)
                .build(),
        ])
        .macos_ignore_rules([
            IgnoreRule::builder().process_name("Firefox").build(),
            IgnoreRule::builder().process_name("Google Chrome").build(),
            IgnoreRule::builder()
                .process_name("whatever")
                .window_title(WindowTitleMatch::Missing)
                .build(),
        ])
        .windows_ignore_rules([
            IgnoreRule::builder().process_name("firefox.exe").build(),
            IgnoreRule::builder().process_name("chrome.exe").build(),
            IgnoreRule::builder()
                .process_name("whatever")
                .window_title(WindowTitleMatch::Missing)
                .build(),
        ])
        .build();

    println!("🔍 Tracking focus with per-platform ignore rules.");
    println!(
        "   Active rules on this platform: {} rule(s)",
        config.ignore_rules_for_current_platform().len()
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
                window.window_title.as_deref().unwrap_or("<no title>")
            );
            Ok(())
        })
        .stop_signal(&stop_signal)
        .call()
        .await?;

    Ok(())
}
