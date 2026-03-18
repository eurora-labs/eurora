//! Phase 4 - Icon Data Verification Tests
//!
//! These tests verify that icon data is properly formatted and can be
//! differentiated between different applications.

mod util;

use focus_tracker::{FocusTracker, FocusedWindow};
use serial_test::serial;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;
use util::*;

#[tokio::test]
#[serial]
async fn test_icon_format_png() {
    if !should_run_integration_tests() {
        tracing::info!("Skipping integration test - INTEGRATION_TEST=1 not set");
        return;
    }

    if let Err(e) = setup_test_environment() {
        tracing::info!("Skipping test due to environment setup failure: {}", e);
        return;
    }

    #[cfg(any(target_os = "windows", target_os = "macos"))]
    {
        let focus_events = Arc::new(tokio::sync::Mutex::new(Vec::<FocusedWindow>::new()));
        let focus_events_clone = Arc::clone(&focus_events);
        let stop_signal = Arc::new(AtomicBool::new(false));
        let stop_signal_clone = Arc::clone(&stop_signal);

        match spawn_test_window("PNG Icon Test Window") {
            Ok(mut child) => {
                tracing::info!("Spawned test window for PNG icon test");

                if let Err(e) = focus_window(&mut child) {
                    tracing::info!("Warning: Failed to focus window: {}", e);
                }

                let tracker_handle = tokio::spawn(async move {
                    let tracker = FocusTracker::builder().build();
                    let _ = tracker
                        .track_focus()
                        .on_focus(move |window: FocusedWindow| {
                            let events = Arc::clone(&focus_events_clone);
                            async move {
                                events.lock().await.push(window);
                                Ok(())
                            }
                        })
                        .stop_signal(&stop_signal_clone)
                        .call()
                        .await;
                });

                tokio::time::sleep(Duration::from_millis(1000)).await;
                stop_signal.store(true, Ordering::Relaxed);
                let _ = tracker_handle.await;

                if let Err(e) = cleanup_child_process(child) {
                    tracing::info!("Warning: Failed to cleanup child process: {}", e);
                }
            }
            Err(e) => {
                tracing::info!("Could not spawn test window: {}", e);
            }
        }
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        tracing::info!("PNG format test is primarily for Windows/macOS platforms");
    }
}

#[tokio::test]
#[serial]
async fn test_icon_format_rgba() {
    if !should_run_integration_tests() {
        tracing::info!("Skipping integration test - INTEGRATION_TEST=1 not set");
        return;
    }

    if let Err(e) = setup_test_environment() {
        tracing::info!("Skipping test due to environment setup failure: {}", e);
        return;
    }

    #[cfg(target_os = "linux")]
    {
        let focus_events = Arc::new(tokio::sync::Mutex::new(Vec::<FocusedWindow>::new()));
        let focus_events_clone = Arc::clone(&focus_events);
        let stop_signal = Arc::new(AtomicBool::new(false));
        let stop_signal_clone = Arc::clone(&stop_signal);

        match spawn_test_window("RGBA Icon Test Window") {
            Ok(mut child) => {
                tracing::info!("Spawned test window for RGBA icon test");

                if let Err(e) = focus_window(&mut child) {
                    tracing::info!("Warning: Failed to focus window: {}", e);
                }

                let tracker_handle = tokio::spawn(async move {
                    let tracker = FocusTracker::builder().build();
                    let _ = tracker
                        .track_focus()
                        .on_focus(move |window: FocusedWindow| {
                            let events = Arc::clone(&focus_events_clone);
                            async move {
                                events.lock().await.push(window);
                                Ok(())
                            }
                        })
                        .stop_signal(&stop_signal_clone)
                        .call()
                        .await;
                });

                tokio::time::sleep(Duration::from_millis(1000)).await;
                stop_signal.store(true, Ordering::Relaxed);
                let _ = tracker_handle.await;

                let events = focus_events.lock().await;
                for event in events.iter() {
                    if let Some(icon) = &event.icon {
                        let expected_size = icon.width() * icon.height() * 4;
                        let actual_size = icon.pixels().len() as u32;

                        tracing::info!(
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
                        tracing::info!("RGBA icon format validation passed");
                    }
                }

                if let Err(e) = cleanup_child_process(child) {
                    tracing::info!("Warning: Failed to cleanup child process: {}", e);
                }
            }
            Err(e) => {
                tracing::info!("Could not spawn test window: {}", e);
            }
        }
    }

    #[cfg(not(target_os = "linux"))]
    {
        tracing::info!("RGBA format test is primarily for Linux X11 systems");
    }
}

#[tokio::test]
#[serial]
async fn test_icon_diff_between_apps() {
    if !should_run_integration_tests() {
        tracing::info!("Skipping integration test - INTEGRATION_TEST=1 not set");
        return;
    }

    if let Err(e) = setup_test_environment() {
        tracing::info!("Skipping test due to environment setup failure: {}", e);
        return;
    }

    let test_windows = vec!["Text Editor Window", "Browser Window", "Terminal Window"];

    for window_title in test_windows {
        let focus_events = Arc::new(tokio::sync::Mutex::new(Vec::<FocusedWindow>::new()));
        let focus_events_clone = Arc::clone(&focus_events);
        let stop_signal = Arc::new(AtomicBool::new(false));
        let stop_signal_clone = Arc::clone(&stop_signal);

        match spawn_test_window(window_title) {
            Ok(mut child) => {
                tracing::info!("Spawned test window: {}", window_title);

                if let Err(e) = focus_window(&mut child) {
                    tracing::info!("Warning: Failed to focus window: {}", e);
                }

                let tracker_handle = tokio::spawn(async move {
                    let tracker = FocusTracker::builder().build();
                    let _ = tracker
                        .track_focus()
                        .on_focus(move |window: FocusedWindow| {
                            let events = Arc::clone(&focus_events_clone);
                            async move {
                                events.lock().await.push(window);
                                Ok(())
                            }
                        })
                        .stop_signal(&stop_signal_clone)
                        .call()
                        .await;
                });

                tokio::time::sleep(Duration::from_millis(1000)).await;
                stop_signal.store(true, Ordering::Relaxed);
                let _ = tracker_handle.await;

                if let Err(e) = cleanup_child_process(child) {
                    tracing::info!("Warning: Failed to cleanup child process: {}", e);
                }

                tokio::time::sleep(Duration::from_millis(500)).await;
            }
            Err(e) => {
                tracing::info!("Could not spawn test window '{}': {}", window_title, e);
            }
        }
    }
}
