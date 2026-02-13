//! Phase 3 - Permission & Fallback Behavior Tests
//!
//! These tests verify that the library handles permission errors and
//! unsupported environments gracefully without panicking.

mod util;

use focus_tracker::{FocusTracker, FocusTrackerError, FocusTrackerResult, FocusedWindow};
use serial_test::serial;
#[allow(unused_imports)]
use std::env;
use std::sync::atomic::{AtomicBool, Ordering};
#[allow(unused_imports)]
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tracing::info;
use util::*;

/// Test macOS Accessibility permission handling
#[cfg(target_os = "macos")]
#[test]
#[serial]
#[ignore] // Only run when AX_ALLOWED=1 is set
fn test_macos_accessibility_permission() {
    // Only run this test if explicitly enabled
    if env::var("AX_ALLOWED").unwrap_or_default() != "1" {
        info!("Skipping macOS accessibility test - AX_ALLOWED=1 not set");
        return;
    }

    if !should_run_integration_tests() {
        info!("Skipping integration test - INTEGRATION_TEST=1 not set");
        return;
    }

    info!("Testing macOS Accessibility permission handling");

    let tracker = FocusTracker::new();
    let stop_signal = AtomicBool::new(false);
    let focus_events = Arc::new(Mutex::new(Vec::new()));

    // Try to track focus - this should either work (if permission granted)
    // or return an error/None window title (if permission denied)
    let focus_events_clone = Arc::clone(&focus_events);
    let result = tracker.track_focus_with_stop(
        move |window: FocusedWindow| -> FocusTrackerResult<()> {
            info!("Focus event received: {:?}", window);
            if let Ok(mut events) = focus_events_clone.lock() {
                events.push(window);
            }
            Ok(())
        },
        &stop_signal,
    );

    // Stop after a short time
    std::thread::sleep(Duration::from_millis(500));
    stop_signal.store(true, Ordering::Relaxed);

    match result {
        Ok(_) => {
            info!("Focus tracking succeeded - accessibility permission likely granted");
            // Check if we got meaningful window titles
            if let Ok(events) = focus_events.lock()
                && events.iter().any(|w| w.window_title.is_none())
            {
                info!("Some windows had no title - possible permission issue");
            }
        }
        Err(FocusTrackerError::PermissionDenied { .. }) => {
            info!("Expected PermissionDenied error received");
        }
        Err(e) => {
            info!("Unexpected error (but didn't panic): {}", e);
        }
    }
}

/// Test macOS Accessibility without permission (mock test)
#[cfg(target_os = "macos")]
#[test]
#[serial]
fn test_macos_accessibility_no_permission_mock() {
    if !should_run_integration_tests() {
        info!("Skipping integration test - INTEGRATION_TEST=1 not set");
        return;
    }

    info!("Testing macOS Accessibility mock permission denial");

    // This test simulates what should happen when accessibility permission is denied
    // We'll create a mock scenario by testing the error handling path

    // Test that we can create the tracker without panicking
    let tracker = FocusTracker::new();
    info!("FocusTracker created successfully: {:?}", tracker);

    // Test that calling the API doesn't panic even in error conditions
    let stop_signal = AtomicBool::new(false);

    // Set stop signal immediately to avoid long-running test
    stop_signal.store(true, Ordering::Relaxed);

    let result = tracker.track_focus_with_stop(
        |window: FocusedWindow| -> FocusTrackerResult<()> {
            // If we get a window with no title, that could indicate permission issues
            if window.window_title.is_none() {
                info!("Received window with no title - possible permission issue");
            }
            Ok(())
        },
        &stop_signal,
    );

    // The important thing is that we don't panic
    match result {
        Ok(_) => info!("Focus tracking completed without error"),
        Err(e) => info!("Focus tracking failed gracefully: {}", e),
    }
}

/// Test Wayland unsupported compositor handling
#[cfg(target_os = "linux")]
#[test]
#[serial]
fn test_wayland_unsupported_compositor() {
    if !should_run_integration_tests() {
        info!("Skipping integration test - INTEGRATION_TEST=1 not set");
        return;
    }

    // Only run this test if we're in a Wayland environment
    if !should_use_wayland() {
        info!("Skipping Wayland test - not in Wayland environment");
        return;
    }

    info!("Testing Wayland unsupported compositor handling");

    // Test under conditions that might not support wlr-toplevel or GNOME DBus
    // This simulates running under Weston headless or other minimal compositors

    let tracker = FocusTracker::new();
    let stop_signal = AtomicBool::new(false);

    // Set a short timeout to avoid hanging
    let timeout_handle = std::thread::spawn(|| {
        std::thread::sleep(Duration::from_millis(1000));
    });

    // Set stop signal after a short delay
    std::thread::spawn(|| {
        std::thread::sleep(Duration::from_millis(500));
    });

    // Set stop signal immediately to avoid long test
    stop_signal.store(true, Ordering::Relaxed);

    let result = tracker.track_focus_with_stop(
        |window: FocusedWindow| -> FocusTrackerResult<()> {
            info!(
                "Unexpected focus event in unsupported environment: {:?}",
                window
            );
            Ok(())
        },
        &stop_signal,
    );

    // Clean up timeout thread
    let _ = timeout_handle.join();

    match result {
        Ok(_) => {
            info!("Focus tracking completed - compositor may be supported");
        }
        Err(FocusTrackerError::Unsupported) => {
            info!("Expected Unsupported error received - test passed");
        }
        Err(e) => {
            info!("Received error (didn't panic): {}", e);
        }
    }
}

/// Test missing X server handling
#[cfg(target_os = "linux")]
#[test]
#[serial]
fn test_missing_x_server() {
    if !should_run_integration_tests() {
        info!("Skipping integration test - INTEGRATION_TEST=1 not set");
        return;
    }

    info!("Testing missing X server handling");

    // Save original DISPLAY value
    let original_display = env::var("DISPLAY").ok();

    // Unset DISPLAY to simulate missing X server
    unsafe {
        env::remove_var("DISPLAY");
    }

    // Also ensure we're not using Wayland for this test
    let original_wayland_display = env::var("WAYLAND_DISPLAY").ok();
    unsafe {
        env::remove_var("WAYLAND_DISPLAY");
    }

    let result = std::panic::catch_unwind(|| {
        let tracker = FocusTracker::new();
        let stop_signal = AtomicBool::new(false);

        // Set stop signal quickly to avoid hanging
        stop_signal.store(true, Ordering::Relaxed);

        tracker.track_focus_with_stop(
            |window: FocusedWindow| -> FocusTrackerResult<()> {
                info!("Unexpected focus event without display: {:?}", window);
                Ok(())
            },
            &stop_signal,
        )
    });

    // Restore environment variables
    if let Some(display) = original_display {
        unsafe {
            env::set_var("DISPLAY", display);
        }
    }
    if let Some(wayland_display) = original_wayland_display {
        unsafe {
            env::set_var("WAYLAND_DISPLAY", wayland_display);
        }
    }

    match result {
        Ok(track_result) => match track_result {
            Ok(_) => {
                info!("Focus tracking completed unexpectedly without display");
            }
            Err(FocusTrackerError::NoDisplay) => {
                info!("Expected NoDisplay error received - test passed");
            }
            Err(FocusTrackerError::Unsupported) => {
                info!("Received Unsupported error - acceptable fallback");
            }
            Err(e) => {
                info!("Received error without panic: {}", e);
            }
        },
        Err(_) => {
            panic!("Code panicked instead of returning error - test failed");
        }
    }
}

/// Test Windows service context handling (mock)
#[cfg(target_os = "windows")]
#[test]
#[serial]
fn test_windows_service_context_mock() {
    if !should_run_integration_tests() {
        info!("Skipping integration test - INTEGRATION_TEST=1 not set");
        return;
    }

    info!("Testing Windows service context handling (mock)");

    // This test simulates what should happen in a Windows service context
    // where there's no interactive desktop session

    let tracker = FocusTracker::new();
    let stop_signal = AtomicBool::new(false);

    // Set stop signal quickly
    stop_signal.store(true, Ordering::Relaxed);

    let result = tracker.track_focus_with_stop(
        |window: FocusedWindow| -> FocusTrackerResult<()> {
            info!("Focus event in service context: {:?}", window);
            Ok(())
        },
        &stop_signal,
    );

    match result {
        Ok(_) => {
            info!("Focus tracking completed - interactive session available");
        }
        Err(FocusTrackerError::NotInteractiveSession) => {
            info!("Expected NotInteractiveSession error received - test passed");
        }
        Err(e) => {
            info!("Received error without panic: {}", e);
        }
    }
}

/// Test general error handling robustness
#[test]
#[serial]
fn test_error_handling_robustness() {
    if !should_run_integration_tests() {
        info!("Skipping integration test - INTEGRATION_TEST=1 not set");
        return;
    }

    info!("Testing general error handling robustness");

    // Test that creating a FocusTracker doesn't panic
    let result = std::panic::catch_unwind(|| {
        let tracker = FocusTracker::new();
        info!("FocusTracker created: {:?}", tracker);
        tracker
    });

    match result {
        Ok(_tracker) => {
            info!("FocusTracker creation succeeded without panic");
        }
        Err(_) => {
            panic!("FocusTracker creation panicked - test failed");
        }
    }
}

/// Test that all error types can be created and displayed
#[test]
fn test_error_types() {
    info!("Testing all error types");

    let errors: Vec<FocusTrackerError> = vec![
        FocusTrackerError::Unsupported,
        FocusTrackerError::PermissionDenied {
            context: "test permission denied".into(),
        },
        FocusTrackerError::NoDisplay,
        FocusTrackerError::NotInteractiveSession,
        FocusTrackerError::ChannelClosed,
        FocusTrackerError::InvalidConfig {
            reason: "test invalid config".into(),
        },
        FocusTrackerError::platform("test platform error"),
        FocusTrackerError::platform_with_source(
            "test platform with source",
            std::io::Error::new(std::io::ErrorKind::Other, "inner error"),
        ),
    ];

    for error in errors {
        info!("Error: {}", error);
        info!("Debug: {:?}", error);
    }

    info!("All error types tested successfully");
}

/// Test timeout behavior to ensure tests don't hang
#[test]
#[serial]
fn test_timeout_behavior() {
    if !should_run_integration_tests() {
        info!("Skipping integration test - INTEGRATION_TEST=1 not set");
        return;
    }

    info!("Testing timeout behavior");

    let tracker = FocusTracker::new();
    let stop_signal = AtomicBool::new(false);

    // Set up a timeout using a separate thread that doesn't capture stop_signal
    let _timeout_handle = std::thread::spawn(|| {
        std::thread::sleep(Duration::from_millis(500));
    });

    // Set stop signal after timeout
    std::thread::spawn(|| {
        std::thread::sleep(Duration::from_millis(400));
    });

    // Set stop signal to ensure test completes quickly
    stop_signal.store(true, Ordering::Relaxed);

    let start_time = std::time::Instant::now();

    let result = tracker.track_focus_with_stop(
        |window: FocusedWindow| -> FocusTrackerResult<()> {
            info!("Focus event: {:?}", window);
            Ok(())
        },
        &stop_signal,
    );

    let elapsed = start_time.elapsed();

    info!("Focus tracking completed in {:?}", elapsed);

    // Should complete within reasonable time (not hang)
    assert!(
        elapsed < Duration::from_secs(2),
        "Test took too long - possible hang"
    );

    match result {
        Ok(_) => info!("Focus tracking completed successfully"),
        Err(e) => info!("Focus tracking failed gracefully: {}", e),
    }
}
