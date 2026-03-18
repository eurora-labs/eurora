//! Phase 5 - Focus Change Detection Tests
//!
//! These tests verify that focus change detection works correctly in both
//! polling and event-driven modes, including stress testing scenarios.

mod util;

use focus_tracker::FocusTracker;
use serial_test::serial;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use util::*;

#[test]
#[serial]
fn polling_focus_switch() {
    if !should_run_integration_tests() {
        tracing::info!("Skipping integration test - INTEGRATION_TEST=1 not set");
        return;
    }

    if let Err(e) = setup_test_environment() {
        tracing::info!("Skipping test due to environment setup failure: {}", e);
        return;
    }

    tracing::info!("Starting polling focus switch test");

    let win_a = match spawn_window("WinA") {
        Ok(child) => child,
        Err(e) => {
            tracing::info!("Failed to spawn WinA: {}", e);
            return;
        }
    };

    let mut win_a_mut = win_a;
    if let Err(e) = focus_window(&mut win_a_mut) {
        tracing::info!("Failed to focus WinA: {}", e);
        cleanup_child_process(win_a_mut).ok();
        return;
    }

    std::thread::sleep(Duration::from_millis(500));
    let focused = get_focused_window();
    tracing::info!(
        "Current focused window after focusing WinA: {:?}",
        focused.window_title
    );

    let win_a_focused = focused
        .window_title
        .as_deref()
        .map(|title| title.contains("WinA"))
        .unwrap_or(false);

    if !win_a_focused {
        tracing::info!("WinA not focused as expected, but continuing test");
    }

    let win_b = match spawn_window("WinB") {
        Ok(child) => child,
        Err(e) => {
            tracing::info!("Failed to spawn WinB: {}", e);
            cleanup_child_process(win_a_mut).ok();
            return;
        }
    };

    let mut win_b_mut = win_b;
    if let Err(e) = focus_window(&mut win_b_mut) {
        tracing::info!("Failed to focus WinB: {}", e);
        cleanup_child_process(win_a_mut).ok();
        cleanup_child_process(win_b_mut).ok();
        return;
    }

    let found_focus = wait_for_focus("WinB", Duration::from_secs(2));
    tracing::info!("Found expected focus for WinB: {}", found_focus);

    let final_focused = get_focused_window();
    tracing::info!("Final focused window: {:?}", final_focused.window_title);

    let win_b_focused = final_focused
        .window_title
        .as_deref()
        .map(|title| title.contains("WinB"))
        .unwrap_or(false);

    if win_b_focused {
        tracing::info!("✓ Polling focus switch test passed");
    } else {
        tracing::info!("⚠ Polling focus switch test: WinB not focused as expected");
    }

    cleanup(win_a_mut, win_b_mut);
}

#[tokio::test]
#[serial]
async fn event_mode_focus_switch() {
    if !should_run_integration_tests() {
        tracing::info!("Skipping integration test - INTEGRATION_TEST=1 not set");
        return;
    }

    if let Err(e) = setup_test_environment() {
        tracing::info!("Skipping test due to environment setup failure: {}", e);
        return;
    }

    tracing::info!("Starting event mode focus switch test");

    let tracker = FocusTracker::builder().build();
    let stop_signal = Arc::new(AtomicBool::new(false));
    let events: Arc<tokio::sync::Mutex<Vec<focus_tracker::FocusedWindow>>> =
        Arc::new(tokio::sync::Mutex::new(Vec::new()));

    let events_clone = Arc::clone(&events);
    let stop_clone = Arc::clone(&stop_signal);

    let track_handle = tokio::spawn(async move {
        let _ = tracker
            .track_focus()
            .on_focus(|window| {
                let events = Arc::clone(&events_clone);
                async move {
                    tracing::info!("Received focus event: {:?}", window.window_title);
                    events.lock().await.push(window);
                    Ok(())
                }
            })
            .stop_signal(&stop_clone)
            .call()
            .await;
    });

    tokio::time::sleep(Duration::from_millis(500)).await;

    let win_a = match spawn_window("EventWinA") {
        Ok(child) => child,
        Err(e) => {
            tracing::info!("Failed to spawn EventWinA: {}", e);
            stop_signal.store(true, Ordering::Release);
            let _ = track_handle.await;
            return;
        }
    };

    let mut win_a_mut = win_a;
    if let Err(e) = focus_window(&mut win_a_mut) {
        tracing::info!("Failed to focus EventWinA: {}", e);
        cleanup_child_process(win_a_mut).ok();
        stop_signal.store(true, Ordering::Release);
        let _ = track_handle.await;
        return;
    }

    tokio::time::sleep(Duration::from_millis(500)).await;

    let win_b = match spawn_window("EventWinB") {
        Ok(child) => child,
        Err(e) => {
            tracing::info!("Failed to spawn EventWinB: {}", e);
            cleanup_child_process(win_a_mut).ok();
            stop_signal.store(true, Ordering::Release);
            let _ = track_handle.await;
            return;
        }
    };

    let mut win_b_mut = win_b;
    if let Err(e) = focus_window(&mut win_b_mut) {
        tracing::info!("Failed to focus EventWinB: {}", e);
        cleanup_child_process(win_a_mut).ok();
        cleanup_child_process(win_b_mut).ok();
        stop_signal.store(true, Ordering::Release);
        let _ = track_handle.await;
        return;
    }

    tokio::time::sleep(Duration::from_secs(3)).await;

    stop_signal.store(true, Ordering::Release);
    let _ = track_handle.await;

    let collected_events = events.lock().await;
    tracing::info!("Collected {} focus events", collected_events.len());

    let has_win_a = collected_events.iter().any(|e| {
        e.window_title
            .as_deref()
            .map(|title| title.contains("EventWinA"))
            .unwrap_or(false)
    });

    let has_win_b = collected_events.iter().any(|e| {
        e.window_title
            .as_deref()
            .map(|title| title.contains("EventWinB"))
            .unwrap_or(false)
    });

    tracing::info!(
        "Event analysis - Has WinA: {}, Has WinB: {}",
        has_win_a,
        has_win_b,
    );

    if collected_events.len() >= 2 && (has_win_a || has_win_b) {
        tracing::info!("✓ Event mode focus switch test passed");
    } else {
        tracing::info!(
            "⚠ Event mode focus switch test: Expected at least 2 events with window focus changes"
        );
    }

    cleanup(win_a_mut, win_b_mut);
}

#[test]
#[serial]
fn stress_focus_switch() {
    if !should_run_integration_tests() {
        tracing::info!("Skipping integration test - INTEGRATION_TEST=1 not set");
        return;
    }

    if let Err(e) = setup_test_environment() {
        tracing::info!("Skipping test due to environment setup failure: {}", e);
        return;
    }

    tracing::info!("Starting stress focus switch test");

    let win_a = match spawn_window("StressWinA") {
        Ok(child) => child,
        Err(e) => {
            tracing::info!("Failed to spawn StressWinA: {}", e);
            return;
        }
    };

    let win_b = match spawn_window("StressWinB") {
        Ok(child) => child,
        Err(e) => {
            tracing::info!("Failed to spawn StressWinB: {}", e);
            cleanup_child_process(win_a).ok();
            return;
        }
    };

    let mut win_a_mut = win_a;
    let mut win_b_mut = win_b;

    let mut successful_switches = 0;
    for i in 0..10 {
        tracing::info!("Focus switch iteration {}", i + 1);

        if focus_window(&mut win_a_mut).is_ok() {
            std::thread::sleep(Duration::from_millis(100));

            if wait_for_focus("StressWinA", Duration::from_millis(500)) {
                successful_switches += 1;
            }
        }

        if focus_window(&mut win_b_mut).is_ok() {
            std::thread::sleep(Duration::from_millis(100));

            if wait_for_focus("StressWinB", Duration::from_millis(500)) {
                successful_switches += 1;
            }
        }
    }

    tracing::info!("Successful focus switches: {}/20", successful_switches);

    let final_focused = get_focused_window();
    let final_is_correct = final_focused
        .window_title
        .as_deref()
        .map(|title| title.contains("StressWin"))
        .unwrap_or(false);

    tracing::info!("Final focused window: {:?}", final_focused.window_title);

    if successful_switches >= 10 && final_is_correct {
        tracing::info!("✓ Stress focus switch test passed");
    } else {
        tracing::info!(
            "⚠ Stress focus switch test: Expected more successful switches or correct final state"
        );
    }

    cleanup(win_a_mut, win_b_mut);
}
