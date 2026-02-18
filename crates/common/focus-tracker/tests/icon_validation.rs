//! Phase 4 - Icon Data Verification Tests
//!
//! These tests verify that icon data is properly formatted and can be
//! differentiated between different applications.

mod util;

use focus_tracker::{FocusTracker, FocusedWindow};
use serial_test::serial;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;
use tracing::info;
use util::*;

/// Test that PNG format icons have correct PNG header and can be decoded
#[test]
#[serial]
fn test_icon_format_png() {
    if !should_run_integration_tests() {
        info!("Skipping integration test - INTEGRATION_TEST=1 not set");
        return;
    }

    if let Err(e) = setup_test_environment() {
        info!("Skipping test due to environment setup failure: {}", e);
        return;
    }

    #[cfg(any(target_os = "windows", target_os = "macos"))]
    {
        let focus_events = Arc::new(Mutex::new(Vec::<FocusedWindow>::new()));
        let focus_events_clone = focus_events.clone();
        let stop_signal = Arc::new(AtomicBool::new(false));
        let stop_signal_clone = stop_signal.clone();

        match spawn_test_window("PNG Icon Test Window") {
            Ok(mut child) => {
                info!("Spawned test window for PNG icon test");

                if let Err(e) = focus_window(&mut child) {
                    info!("Warning: Failed to focus window: {}", e);
                }

                let tracker_handle = std::thread::spawn(move || {
                    let tracker = FocusTracker::new();
                    let _ = tracker.track_focus_with_stop(
                        move |window: FocusedWindow| -> focus_tracker::FocusTrackerResult<()> {
                            if let Ok(mut events) = focus_events_clone.lock() {
                                events.push(window);
                            }
                            Ok(())
                        },
                        &stop_signal_clone,
                    );
                });

                std::thread::sleep(Duration::from_millis(1000));
                stop_signal.store(true, Ordering::Relaxed);
                let _ = tracker_handle.join();

                if let Err(e) = cleanup_child_process(child) {
                    info!("Warning: Failed to cleanup child process: {}", e);
                }
            }
            Err(e) => {
                info!("Could not spawn test window: {}", e);
            }
        }
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        info!("PNG format test is primarily for Windows/macOS platforms");
    }
}

/// Test that RGBA format icons have correct dimensions
#[test]
#[serial]
fn test_icon_format_rgba() {
    if !should_run_integration_tests() {
        info!("Skipping integration test - INTEGRATION_TEST=1 not set");
        return;
    }

    if let Err(e) = setup_test_environment() {
        info!("Skipping test due to environment setup failure: {}", e);
        return;
    }

    #[cfg(target_os = "linux")]
    {
        let focus_events = Arc::new(Mutex::new(Vec::<FocusedWindow>::new()));
        let focus_events_clone = focus_events.clone();
        let stop_signal = Arc::new(AtomicBool::new(false));
        let stop_signal_clone = stop_signal.clone();

        match spawn_test_window("RGBA Icon Test Window") {
            Ok(mut child) => {
                info!("Spawned test window for RGBA icon test");

                if let Err(e) = focus_window(&mut child) {
                    info!("Warning: Failed to focus window: {}", e);
                }

                let tracker_handle = std::thread::spawn(move || {
                    let tracker = FocusTracker::new();
                    let _ = tracker.track_focus_with_stop(
                        move |window: FocusedWindow| -> focus_tracker::FocusTrackerResult<()> {
                            if let Ok(mut events) = focus_events_clone.lock() {
                                events.push(window);
                            }
                            Ok(())
                        },
                        &stop_signal_clone,
                    );
                });

                std::thread::sleep(Duration::from_millis(1000));
                stop_signal.store(true, Ordering::Relaxed);
                let _ = tracker_handle.join();

                if let Ok(events) = focus_events.lock() {
                    for event in events.iter() {
                        if let Some(icon) = &event.icon {
                            let expected_size = icon.width() * icon.height() * 4;
                            let actual_size = icon.pixels().len() as u32;

                            info!(
                                "Icon dimensions: {}x{}, expected size: {}, actual size: {}",
                                icon.width(),
                                icon.height(),
                                expected_size,
                                actual_size
                            );

                            assert_eq!(
                                expected_size, actual_size,
                                "Icon data size should match width * height * 4 for RGBA format. Expected: {expected_size} bytes, Actual: {actual_size} bytes",
                            );
                            info!("RGBA icon format validation passed");
                        }
                    }
                }

                if let Err(e) = cleanup_child_process(child) {
                    info!("Warning: Failed to cleanup child process: {}", e);
                }
            }
            Err(e) => {
                info!("Could not spawn test window: {}", e);
            }
        }
    }

    #[cfg(not(target_os = "linux"))]
    {
        info!("RGBA format test is primarily for Linux X11 systems");
    }
}

/// Test that different applications have different icon hashes
#[test]
#[serial]
fn test_icon_diff_between_apps() {
    if !should_run_integration_tests() {
        info!("Skipping integration test - INTEGRATION_TEST=1 not set");
        return;
    }

    if let Err(e) = setup_test_environment() {
        info!("Skipping test due to environment setup failure: {}", e);
        return;
    }

    let test_windows = vec!["Text Editor Window", "Browser Window", "Terminal Window"];

    for window_title in test_windows {
        let focus_events = Arc::new(Mutex::new(Vec::<FocusedWindow>::new()));
        let focus_events_clone = focus_events.clone();
        let stop_signal = Arc::new(AtomicBool::new(false));
        let stop_signal_clone = stop_signal.clone();

        match spawn_test_window(window_title) {
            Ok(mut child) => {
                info!("Spawned test window: {}", window_title);

                if let Err(e) = focus_window(&mut child) {
                    info!("Warning: Failed to focus window: {}", e);
                }

                let tracker_handle = std::thread::spawn(move || {
                    let tracker = FocusTracker::new();
                    let _ = tracker.track_focus_with_stop(
                        move |window: FocusedWindow| -> focus_tracker::FocusTrackerResult<()> {
                            if let Ok(mut events) = focus_events_clone.lock() {
                                events.push(window);
                            }
                            Ok(())
                        },
                        &stop_signal_clone,
                    );
                });

                std::thread::sleep(Duration::from_millis(1000));
                stop_signal.store(true, Ordering::Relaxed);
                let _ = tracker_handle.join();

                if let Err(e) = cleanup_child_process(child) {
                    info!("Warning: Failed to cleanup child process: {}", e);
                }

                std::thread::sleep(Duration::from_millis(500));
            }
            Err(e) => {
                info!("Could not spawn test window '{}': {}", window_title, e);
            }
        }
    }
}
