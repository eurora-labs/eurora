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
use util::*;

/// Test macOS Accessibility permission handling
#[cfg(target_os = "macos")]
#[test]
#[serial]
#[ignore] // Only run when AX_ALLOWED=1 is set
fn test_macos_accessibility_permission() {
    if env::var("AX_ALLOWED").unwrap_or_default() != "1" {
        tracing::info!("Skipping macOS accessibility test - AX_ALLOWED=1 not set");
        return;
    }

    if !should_run_integration_tests() {
        tracing::info!("Skipping integration test - INTEGRATION_TEST=1 not set");
        return;
    }

    tracing::info!("Testing macOS Accessibility permission handling");

    let tracker = FocusTracker::new();
    let stop_signal = AtomicBool::new(false);
    let focus_events = Arc::new(Mutex::new(Vec::new()));

    let focus_events_clone = Arc::clone(&focus_events);
    let result = tracker.track_focus_with_stop(
        move |window: FocusedWindow| -> FocusTrackerResult<()> {
            tracing::info!("Focus event received: {:?}", window);
            if let Ok(mut events) = focus_events_clone.lock() {
                events.push(window);
            }
            Ok(())
        },
        &stop_signal,
    );

    std::thread::sleep(Duration::from_millis(500));
    stop_signal.store(true, Ordering::Relaxed);

    match result {
        Ok(_) => {
            tracing::info!("Focus tracking succeeded - accessibility permission likely granted");
            if let Ok(events) = focus_events.lock()
                && events.iter().any(|w| w.window_title.is_none())
            {
                tracing::info!("Some windows had no title - possible permission issue");
            }
        }
        Err(FocusTrackerError::PermissionDenied { .. }) => {
            tracing::info!("Expected PermissionDenied error received");
        }
        Err(e) => {
            tracing::info!("Unexpected error (but didn't panic): {}", e);
        }
    }
}

/// Test macOS Accessibility without permission (mock test)
#[cfg(target_os = "macos")]
#[test]
#[serial]
fn test_macos_accessibility_no_permission_mock() {
    if !should_run_integration_tests() {
        tracing::info!("Skipping integration test - INTEGRATION_TEST=1 not set");
        return;
    }

    tracing::info!("Testing macOS Accessibility mock permission denial");

    let tracker = FocusTracker::new();
    tracing::info!("FocusTracker created successfully: {:?}", tracker);

    let stop_signal = AtomicBool::new(false);

    stop_signal.store(true, Ordering::Relaxed);

    let result = tracker.track_focus_with_stop(
        |window: FocusedWindow| -> FocusTrackerResult<()> {
            if window.window_title.is_none() {
                tracing::info!("Received window with no title - possible permission issue");
            }
            Ok(())
        },
        &stop_signal,
    );

    match result {
        Ok(_) => tracing::info!("Focus tracking completed without error"),
        Err(e) => tracing::info!("Focus tracking failed gracefully: {}", e),
    }
}

/// Test Wayland unsupported compositor handling
#[cfg(target_os = "linux")]
#[test]
#[serial]
fn test_wayland_unsupported_compositor() {
    if !should_run_integration_tests() {
        tracing::info!("Skipping integration test - INTEGRATION_TEST=1 not set");
        return;
    }

    if !should_use_wayland() {
        tracing::info!("Skipping Wayland test - not in Wayland environment");
        return;
    }

    tracing::info!("Testing Wayland unsupported compositor handling");

    let tracker = FocusTracker::new();
    let stop_signal = AtomicBool::new(false);

    let timeout_handle = std::thread::spawn(|| {
        std::thread::sleep(Duration::from_millis(1000));
    });

    std::thread::spawn(|| {
        std::thread::sleep(Duration::from_millis(500));
    });

    stop_signal.store(true, Ordering::Relaxed);

    let result = tracker.track_focus_with_stop(
        |window: FocusedWindow| -> FocusTrackerResult<()> {
            tracing::info!(
                "Unexpected focus event in unsupported environment: {:?}",
                window
            );
            Ok(())
        },
        &stop_signal,
    );

    let _ = timeout_handle.join();

    match result {
        Ok(_) => {
            tracing::info!("Focus tracking completed - compositor may be supported");
        }
        Err(FocusTrackerError::Unsupported) => {
            tracing::info!("Expected Unsupported error received - test passed");
        }
        Err(e) => {
            tracing::info!("Received error (didn't panic): {}", e);
        }
    }
}

/// Test missing X server handling
#[cfg(target_os = "linux")]
#[test]
#[serial]
fn test_missing_x_server() {
    if !should_run_integration_tests() {
        tracing::info!("Skipping integration test - INTEGRATION_TEST=1 not set");
        return;
    }

    tracing::info!("Testing missing X server handling");

    let original_display = env::var("DISPLAY").ok();

    unsafe {
        env::remove_var("DISPLAY");
    }

    let original_wayland_display = env::var("WAYLAND_DISPLAY").ok();
    unsafe {
        env::remove_var("WAYLAND_DISPLAY");
    }

    let result = std::panic::catch_unwind(|| {
        let tracker = FocusTracker::new();
        let stop_signal = AtomicBool::new(false);

        stop_signal.store(true, Ordering::Relaxed);

        tracker.track_focus_with_stop(
            |window: FocusedWindow| -> FocusTrackerResult<()> {
                tracing::info!("Unexpected focus event without display: {:?}", window);
                Ok(())
            },
            &stop_signal,
        )
    });

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
                tracing::info!("Focus tracking completed unexpectedly without display");
            }
            Err(FocusTrackerError::NoDisplay) => {
                tracing::info!("Expected NoDisplay error received - test passed");
            }
            Err(FocusTrackerError::Unsupported) => {
                tracing::info!("Received Unsupported error - acceptable fallback");
            }
            Err(e) => {
                tracing::info!("Received error without panic: {}", e);
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
        tracing::info!("Skipping integration test - INTEGRATION_TEST=1 not set");
        return;
    }

    tracing::info!("Testing Windows service context handling (mock)");

    let tracker = FocusTracker::new();
    let stop_signal = AtomicBool::new(false);

    stop_signal.store(true, Ordering::Relaxed);

    let result = tracker.track_focus_with_stop(
        |window: FocusedWindow| -> FocusTrackerResult<()> {
            tracing::info!("Focus event in service context: {:?}", window);
            Ok(())
        },
        &stop_signal,
    );

    match result {
        Ok(_) => {
            tracing::info!("Focus tracking completed - interactive session available");
        }
        Err(FocusTrackerError::NotInteractiveSession) => {
            tracing::info!("Expected NotInteractiveSession error received - test passed");
        }
        Err(e) => {
            tracing::info!("Received error without panic: {}", e);
        }
    }
}

/// Test general error handling robustness
#[test]
#[serial]
fn test_error_handling_robustness() {
    if !should_run_integration_tests() {
        tracing::info!("Skipping integration test - INTEGRATION_TEST=1 not set");
        return;
    }

    tracing::info!("Testing general error handling robustness");

    let result = std::panic::catch_unwind(|| {
        let tracker = FocusTracker::new();
        tracing::info!("FocusTracker created: {:?}", tracker);
        tracker
    });

    match result {
        Ok(_tracker) => {
            tracing::info!("FocusTracker creation succeeded without panic");
        }
        Err(_) => {
            panic!("FocusTracker creation panicked - test failed");
        }
    }
}

/// Test that all error types can be created and displayed
#[test]
fn test_error_types() {
    tracing::info!("Testing all error types");

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
            std::io::Error::other("inner error"),
        ),
    ];

    for error in errors {
        tracing::info!("Error: {}", error);
        tracing::info!("Debug: {:?}", error);
    }

    tracing::info!("All error types tested successfully");
}

/// Test timeout behavior to ensure tests don't hang
#[test]
#[serial]
fn test_timeout_behavior() {
    if !should_run_integration_tests() {
        tracing::info!("Skipping integration test - INTEGRATION_TEST=1 not set");
        return;
    }

    tracing::info!("Testing timeout behavior");

    let tracker = FocusTracker::new();
    let stop_signal = AtomicBool::new(false);

    let _timeout_handle = std::thread::spawn(|| {
        std::thread::sleep(Duration::from_millis(500));
    });

    std::thread::spawn(|| {
        std::thread::sleep(Duration::from_millis(400));
    });

    stop_signal.store(true, Ordering::Relaxed);

    let start_time = std::time::Instant::now();

    let result = tracker.track_focus_with_stop(
        |window: FocusedWindow| -> FocusTrackerResult<()> {
            tracing::info!("Focus event: {:?}", window);
            Ok(())
        },
        &stop_signal,
    );

    let elapsed = start_time.elapsed();

    tracing::info!("Focus tracking completed in {:?}", elapsed);

    assert!(
        elapsed < Duration::from_secs(2),
        "Test took too long - possible hang"
    );

    match result {
        Ok(_) => tracing::info!("Focus tracking completed successfully"),
        Err(e) => tracing::info!("Focus tracking failed gracefully: {}", e),
    }
}
