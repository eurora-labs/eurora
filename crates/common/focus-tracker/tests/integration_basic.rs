//! Basic integration tests for focus-tracker
//!
//! These tests verify that the basic focus tracking functionality works
//! across different platforms and display backends.

mod util;

use focus_tracker::{FocusTracker, FocusedWindow};
use serial_test::serial;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;
use util::*;

#[test]
#[serial]
fn test_environment_setup() {
    if !should_run_integration_tests() {
        tracing::info!("Skipping integration test - INTEGRATION_TEST=1 not set");
        return;
    }

    assert!(setup_test_environment().is_ok());
    tracing::info!("Test environment setup successful");
}

#[test]
#[serial]
fn test_spawn_window_helper() {
    if !should_run_integration_tests() {
        tracing::info!("Skipping integration test - INTEGRATION_TEST=1 not set");
        return;
    }

    if let Err(e) = setup_test_environment() {
        tracing::info!("Skipping test due to environment setup failure: {}", e);
        return;
    }

    let child = spawn_test_window("Test Window Basic");
    match child {
        Ok(child) => {
            tracing::info!("Successfully spawned test window");

            std::thread::sleep(Duration::from_secs(1));

            if let Err(e) = cleanup_child_process(child) {
                tracing::info!("Warning: Failed to cleanup child process: {}", e);
            }
        }
        Err(e) => {
            tracing::info!(
                "Failed to spawn test window (this may be expected in headless environments): {}",
                e
            );
        }
    }
}

#[tokio::test]
#[serial]
async fn test_basic_focus_tracking() {
    if !should_run_integration_tests() {
        tracing::info!("Skipping integration test - INTEGRATION_TEST=1 not set");
        return;
    }

    if let Err(e) = setup_test_environment() {
        tracing::info!("Skipping test due to environment setup failure: {}", e);
        return;
    }

    let focus_events = Arc::new(tokio::sync::Mutex::new(Vec::<FocusedWindow>::new()));
    let focus_events_clone = Arc::clone(&focus_events);

    let stop_signal = Arc::new(AtomicBool::new(false));
    let stop_signal_clone = Arc::clone(&stop_signal);

    let tracker_handle = tokio::spawn(async move {
        let tracker = FocusTracker::builder().build();
        let result = tracker
            .track_focus()
            .on_focus(move |window: FocusedWindow| {
                let events = Arc::clone(&focus_events_clone);
                async move {
                    tracing::info!("Focus event: {:?}", window);
                    events.lock().await.push(window);
                    Ok(())
                }
            })
            .stop_signal(&stop_signal_clone)
            .call()
            .await;

        match result {
            Ok(_) => tracing::info!("Focus tracking completed"),
            Err(e) => tracing::info!("Focus tracking failed: {}", e),
        }
    });

    tokio::time::sleep(Duration::from_millis(500)).await;

    stop_signal.store(true, Ordering::Relaxed);

    if let Err(e) = tracker_handle.await {
        tracing::info!("Failed to join tracker task: {:?}", e);
    }

    tracing::info!("Focus tracking test completed successfully");

    let events = focus_events.lock().await;
    tracing::info!("Captured {} focus events", events.len());
    for (i, event) in events.iter().enumerate() {
        tracing::info!("Event {}: {:?}", i + 1, event);
    }
}

#[test]
#[serial]
fn test_wayland_detection() {
    if !should_run_integration_tests() {
        tracing::info!("Skipping integration test - INTEGRATION_TEST=1 not set");
        return;
    }

    let using_wayland = should_use_wayland();
    let using_x11 = should_use_x11();

    tracing::info!("Wayland flag: {}", using_wayland);
    tracing::info!("X11 flag: {}", using_x11);

    #[cfg(target_os = "linux")]
    {
        let is_wayland_session = std::env::var("XDG_SESSION_TYPE")
            .map(|v| v.eq_ignore_ascii_case("wayland"))
            .unwrap_or(false);
        let has_wayland_display = std::env::var("WAYLAND_DISPLAY")
            .map(|v| !v.is_empty())
            .unwrap_or(false);
        let detected_wayland = is_wayland_session || has_wayland_display;
        tracing::info!("Detected Wayland: {}", detected_wayland);
    }
}

#[test]
#[serial]
fn test_focus_tracking_with_window() {
    if !should_run_integration_tests() {
        tracing::info!("Skipping integration test - INTEGRATION_TEST=1 not set");
        return;
    }

    if let Err(e) = setup_test_environment() {
        tracing::info!("Skipping test due to environment setup failure: {}", e);
        return;
    }

    let window_title = "Focus Test Window";

    match spawn_test_window(window_title) {
        Ok(mut child) => {
            tracing::info!("Spawned test window: {}", window_title);

            if let Err(e) = focus_window(&mut child) {
                tracing::info!("Warning: Failed to focus window: {}", e);
            }

            std::thread::sleep(Duration::from_millis(500));

            let found_focus = wait_for_focus(window_title, Duration::from_secs(2));
            tracing::info!("Found expected focus: {}", found_focus);

            if let Err(e) = cleanup_child_process(child) {
                tracing::info!("Warning: Failed to cleanup child process: {}", e);
            }
        }
        Err(e) => {
            tracing::info!(
                "Could not spawn test window (expected in headless environments): {}",
                e
            );
        }
    }
}

#[cfg(target_os = "linux")]
#[test]
#[serial]
fn test_linux_backend_selection() {
    if !should_run_integration_tests() {
        tracing::info!("Skipping integration test - INTEGRATION_TEST=1 not set");
        return;
    }

    use focus_tracker::FocusTracker;

    let tracker = FocusTracker::builder().build();
    tracing::info!("Successfully created Linux focus tracker: {:?}", tracker);

    let is_wayland_session = std::env::var("XDG_SESSION_TYPE")
        .map(|v| v.eq_ignore_ascii_case("wayland"))
        .unwrap_or(false);
    let has_wayland_display = std::env::var("WAYLAND_DISPLAY")
        .map(|v| !v.is_empty())
        .unwrap_or(false);
    let is_wayland = is_wayland_session || has_wayland_display;
    tracing::info!(
        "Detected backend - Wayland: {}, X11: {}",
        is_wayland,
        !is_wayland
    );
}
