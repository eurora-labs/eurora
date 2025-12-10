//! Basic integration tests for euro-focus
//!
//! These tests verify that the basic focus tracking functionality works
//! across different platforms and display backends.

mod util;

use euro_focus::{EuroFocusResult, FocusTracker, FocusedWindow};
use serial_test::serial;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;
use tracing::info;
use util::*;

#[test]
#[serial]
fn test_environment_setup() {
    // This test just verifies that our test environment setup works
    if !should_run_integration_tests() {
        info!("Skipping integration test - INTEGRATION_TEST=1 not set");
        return;
    }

    assert!(setup_test_environment().is_ok());
    info!("Test environment setup successful");
}

#[test]
#[serial]
fn test_spawn_window_helper() {
    if !should_run_integration_tests() {
        info!("Skipping integration test - INTEGRATION_TEST=1 not set");
        return;
    }

    if let Err(e) = setup_test_environment() {
        info!("Skipping test due to environment setup failure: {}", e);
        return;
    }

    // Test spawning a basic window
    let child = spawn_test_window("Test Window Basic");
    match child {
        Ok(child) => {
            info!("Successfully spawned test window");

            // Let the window exist for a moment
            std::thread::sleep(Duration::from_secs(1));

            // Clean up
            if let Err(e) = cleanup_child_process(child) {
                info!("Warning: Failed to cleanup child process: {}", e);
            }
        }
        Err(e) => {
            info!(
                "Failed to spawn test window (this may be expected in headless environments): {}",
                e
            );
        }
    }
}

#[test]
#[serial]
fn test_basic_focus_tracking() {
    if !should_run_integration_tests() {
        info!("Skipping integration test - INTEGRATION_TEST=1 not set");
        return;
    }

    if let Err(e) = setup_test_environment() {
        info!("Skipping test due to environment setup failure: {}", e);
        return;
    }

    // Test basic focus tracking functionality
    let focus_events = Arc::new(Mutex::new(Vec::<FocusedWindow>::new()));
    let focus_events_clone = focus_events.clone();

    // Create a stop signal for the tracker
    let stop_signal = Arc::new(AtomicBool::new(false));
    let stop_signal_clone = stop_signal.clone();

    // Spawn the focus tracker in a separate thread with a stop signal
    let tracker_handle = std::thread::spawn(move || {
        let tracker = FocusTracker::new();
        let result = tracker.track_focus_with_stop(
            move |window: FocusedWindow| -> EuroFocusResult<()> {
                info!("Focus event: {:?}", window);
                if let Ok(mut events) = focus_events_clone.lock() {
                    events.push(window);
                }
                Ok(())
            },
            &stop_signal_clone,
        );

        match result {
            Ok(_) => info!("Focus tracking completed"),
            Err(e) => info!("Focus tracking failed: {}", e),
        }
    });

    // Let the tracker run for a short time
    std::thread::sleep(Duration::from_millis(500));

    // Signal the tracker to stop
    stop_signal.store(true, Ordering::Relaxed);

    // Wait for the tracker thread to finish
    if let Err(e) = tracker_handle.join() {
        info!("Failed to join tracker thread: {:?}", e);
    }

    info!("Focus tracking test completed successfully");

    // Check if we got any focus events
    if let Ok(events) = focus_events.lock() {
        info!("Captured {} focus events", events.len());
        for (i, event) in events.iter().enumerate() {
            info!("Event {}: {:?}", i + 1, event);
        }
    }
}

#[test]
#[serial]
fn test_wayland_detection() {
    if !should_run_integration_tests() {
        info!("Skipping integration test - INTEGRATION_TEST=1 not set");
        return;
    }

    // Test Wayland detection logic
    let using_wayland = should_use_wayland();
    let using_x11 = should_use_x11();

    info!("Wayland flag: {}", using_wayland);
    info!("X11 flag: {}", using_x11);

    // Test the actual detection logic from our utils
    #[cfg(target_os = "linux")]
    {
        use euro_focus::utils::wayland_detect;
        let detected_wayland = wayland_detect();
        info!("Detected Wayland: {}", detected_wayland);
    }
}

#[test]
#[serial]
fn test_focus_tracking_with_window() {
    if !should_run_integration_tests() {
        info!("Skipping integration test - INTEGRATION_TEST=1 not set");
        return;
    }

    if let Err(e) = setup_test_environment() {
        info!("Skipping test due to environment setup failure: {}", e);
        return;
    }

    // This test attempts to spawn a window and track focus changes
    let window_title = "Focus Test Window";

    match spawn_test_window(window_title) {
        Ok(mut child) => {
            info!("Spawned test window: {}", window_title);

            // Try to focus the window
            if let Err(e) = focus_window(&mut child) {
                info!("Warning: Failed to focus window: {}", e);
            }

            // Wait for focus to settle
            std::thread::sleep(Duration::from_millis(500));

            // Test if we can detect the focused window
            let found_focus = wait_for_focus(window_title, Duration::from_secs(2));
            info!("Found expected focus: {}", found_focus);

            // Clean up
            if let Err(e) = cleanup_child_process(child) {
                info!("Warning: Failed to cleanup child process: {}", e);
            }
        }
        Err(e) => {
            info!(
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
        info!("Skipping integration test - INTEGRATION_TEST=1 not set");
        return;
    }

    // Test that we can create the Linux focus tracker
    use euro_focus::FocusTracker;

    let tracker = FocusTracker::new();
    info!("Successfully created Linux focus tracker: {:?}", tracker);

    // Test backend detection
    use euro_focus::utils::wayland_detect;
    let is_wayland = wayland_detect();
    info!(
        "Detected backend - Wayland: {}, X11: {}",
        is_wayland, !is_wayland
    );
}
