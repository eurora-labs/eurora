use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::Instant,
};

use crate::procedures::window_procedures::{LauncherInfo, TauRpcWindowApiEventTrigger};
use eur_screen_position::ActiveMonitor;
use eur_vision::{
    capture_focused_region_rgba, capture_monitor, capture_monitor_by_id, get_all_monitors,
    image_to_base64,
};
use tauri::{Emitter, Manager};
use tracing::{error, info};

// Shared state to track if launcher is visible
static LAUNCHER_VISIBLE: AtomicBool = AtomicBool::new(false);

/// Monitor information for window positioning
#[derive(Debug, Clone)]
pub struct MonitorInfo {
    pub id: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub scale_factor: f64,
}

/// Result of finding the monitor containing the cursor
#[derive(Debug)]
pub struct CursorMonitorResult {
    pub monitor: MonitorInfo,
    pub cursor_x: f64,
    pub cursor_y: f64,
}

/// Find the monitor that contains the cursor position for WebviewWindow
pub fn find_cursor_monitor(window: &tauri::WebviewWindow) -> Option<CursorMonitorResult> {
    let cursor_position = window.cursor_position().ok()?;
    find_cursor_monitor_impl(cursor_position)
}

/// Find the monitor that contains the cursor position for Window
pub fn find_cursor_monitor_window<R: tauri::Runtime>(
    window: &tauri::Window<R>,
) -> Option<CursorMonitorResult> {
    let cursor_position = window.cursor_position().ok()?;
    find_cursor_monitor_impl(cursor_position)
}

/// Internal implementation for finding cursor monitor
fn find_cursor_monitor_impl(
    cursor_position: tauri::PhysicalPosition<f64>,
) -> Option<CursorMonitorResult> {
    let monitors = get_all_monitors().ok()?;

    for monitor in monitors {
        let monitor_id = monitor.id().unwrap_or_default().to_string();
        let scale_factor = monitor.scale_factor().unwrap_or(1.0) as f64;
        let monitor_width = (monitor.width().unwrap_or(1920) as f64 * scale_factor) as u32;
        let monitor_height = (monitor.height().unwrap_or(1080) as f64 * scale_factor) as u32;
        let monitor_x = (monitor.x().unwrap_or(0) as f64 * scale_factor) as i32;
        let monitor_y = (monitor.y().unwrap_or(0) as f64 * scale_factor) as i32;

        // Check if cursor is on this monitor
        if cursor_position.x >= monitor_x as f64
            && cursor_position.x <= (monitor_x + monitor_width as i32) as f64
            && cursor_position.y >= monitor_y as f64
            && cursor_position.y <= (monitor_y + monitor_height as i32) as f64
        {
            return Some(CursorMonitorResult {
                monitor: MonitorInfo {
                    id: monitor_id,
                    x: monitor_x,
                    y: monitor_y,
                    width: monitor_width,
                    height: monitor_height,
                    scale_factor,
                },
                cursor_x: cursor_position.x,
                cursor_y: cursor_position.y,
            });
        }
    }
    None
}

/// Calculate launcher position (centered horizontally, 1/4 from top)
pub fn calculate_launcher_position(
    monitor: &MonitorInfo,
    window_size: tauri::PhysicalSize<u32>,
) -> (i32, i32) {
    let launcher_x = monitor.x + (monitor.width as i32 - window_size.width as i32) / 2;
    let launcher_y = monitor.y + (monitor.height as i32 - window_size.height as i32) / 4;
    (launcher_x, launcher_y)
}

/// Calculate hover window position (right side, 3/4 down)
pub fn calculate_hover_position(
    monitor: &MonitorInfo,
    window_size: tauri::PhysicalSize<u32>,
) -> (i32, i32) {
    let hover_x = monitor.x + monitor.width as i32 - window_size.width as i32 - 10; // 10px margin from edge
    let hover_y =
        monitor.y + (monitor.height as f64 * 0.75) as i32 - (window_size.height as i32 / 2);
    (hover_x, hover_y)
}

pub fn toggle_launcher_window<R: tauri::Runtime>(
    launcher: &tauri::Window<R>,
) -> Result<(), String> {
    let is_visible = is_launcher_visible();

    if is_visible {
        // Hide the launcher window and emit the closed event
        launcher
            .hide()
            .map_err(|e| format!("Failed to hide launcher window: {e}"))?;

        let app_clone = launcher.app_handle().clone();
        TauRpcWindowApiEventTrigger::new(app_clone)
            .launcher_closed()
            .map_err(|e| format!("Failed to emit launcher_closed event: {e}"))?;

        // Update the shared state to indicate launcher is hidden
        set_launcher_visible(false);
    } else {
        // Open and position the launcher window
        open_launcher_window(launcher)?;
    }

    Ok(())
}

/// Open and position the launcher window with background capture
pub fn open_launcher_window<R: tauri::Runtime>(launcher: &tauri::Window<R>) -> Result<(), String> {
    // Update the shared state to indicate launcher is visible
    LAUNCHER_VISIBLE.store(true, Ordering::SeqCst);

    let active_monitor = ActiveMonitor::default();

    #[cfg(target_os = "linux")]
    let mut window_size = launcher.inner_size().unwrap();

    #[cfg(target_os = "windows")]
    let mut window_size = launcher.outer_size().unwrap();

    #[cfg(target_os = "macos")]
    let mut window_size = launcher.inner_size().unwrap();

    window_size.width /= 2;
    window_size.height /= 2;

    let (launcher_x, launcher_y) = active_monitor.calculate_position_for_percentage(
        tauri::PhysicalSize {
            width: window_size.width,
            height: window_size.height,
        },
        0.5,
        0.25,
    );
    let monitor = active_monitor.get_info();
    launcher
        .set_position(tauri::Position::Physical(tauri::PhysicalPosition {
            x: launcher_x,
            y: launcher_y,
        }))
        .map_err(|e| format!("Failed to set launcher position: {}", e))?;

    // Calculate relative position for screen capture
    // let capture_x =
    //     ((monitor.width as i32 as f64) / 2.0) as i32 - (window_size.width as f64) as i32;
    // let capture_y =
    //     ((monitor.height as i32 as f64) / 4.0) as i32 - (window_size.height as f64) as i32;

    info!("launcher opened at: ({}, {})", launcher_x, launcher_y);
    info!("monitor_id: {}", monitor.id.clone());
    // Convert absolute launcher position across all screens to relative position on monitor
    let (capture_x, capture_y) =
        active_monitor.convert_absolute_position_to_relative(launcher_x, launcher_y);

    let monitor_image_app = launcher.app_handle().clone();
    let monitor_id = monitor.id.clone();
    tauri::async_runtime::spawn(async move {
        match capture_monitor_by_id(&monitor_id)
            .and_then(|m| Ok(image::DynamicImage::ImageRgba8(m).to_rgb8()))
            .and_then(image_to_base64)
        {
            Ok(base64) => {
                if let Err(e) = TauRpcWindowApiEventTrigger::new(monitor_image_app)
                    .background_image_changed(base64)
                    .map_err(|e| e.to_string())
                {
                    error!("Failed to emit background_image_changed: {}", e);
                }
            }
            Err(e) => error!(
                "Background capture failed for monitor {}: {}",
                monitor_id, e
            ),
        }
    });

    // Capture the screen region behind the launcher
    let background_image = match capture_focused_region_rgba(
        monitor.id.clone(),
        capture_x as u32,
        capture_y as u32,
        window_size.width * 2,
        window_size.height * 2,
    ) {
        Ok(img) => {
            let img = image::DynamicImage::ImageRgba8(img.clone()).to_rgb8();

            info!("Captured image size: {:?}", img.dimensions());

            // Convert the image to base64
            if let Ok(base64_image) = image_to_base64(img) {
                // Send the base64 image to the frontend
                // launcher
                //     .emit("background_image", base64_image)
                //     .map_err(|e| format!("Failed to emit background_image event: {}", e))?;
                Some(base64_image)
            } else {
                error!("Failed to convert image to base64");
                None
            }
        }
        Err(e) => {
            error!("Failed to capture screen region: {}", e);
            None
        }
    };

    // Only show the launcher if it was previously hidden
    launcher
        .show()
        .map_err(|e| format!("Failed to show launcher window: {}", e))?;

    // Emit an event to notify that the launcher has been opened
    // Include positioning information for proper background alignment

    let launcher_info = LauncherInfo {
        background_image,
        monitor_id: monitor.id.clone(),
        launcher_x,
        launcher_y,
        launcher_width: window_size.width,
        launcher_height: window_size.height,
        monitor_width: monitor.width,
        monitor_height: monitor.height,
        monitor_x: monitor.x,
        monitor_y: monitor.y,
        capture_x,
        capture_y,
    };

    // Measure time
    let app_clone = launcher.app_handle().clone();

    TauRpcWindowApiEventTrigger::new(app_clone)
        .launcher_opened(launcher_info)
        .map_err(|e| e.to_string())?;

    launcher
        .set_focus()
        .map_err(|e| format!("Failed to focus launcher window: {}", e))?;

    Ok(())
}

/// Position the hover window to the right side, around 3/4 to the bottom of the screen
pub fn position_hover_window(hover_window: &tauri::WebviewWindow) {
    if let Some(cursor_monitor) = find_cursor_monitor(hover_window) {
        let monitor = &cursor_monitor.monitor;
        let window_size = hover_window.inner_size().unwrap_or(tauri::PhysicalSize {
            width: 50,
            height: 50,
        });

        // Calculate hover position using consolidated function
        let (hover_x, hover_y) = calculate_hover_position(monitor, window_size);

        info!(
            "Positioning hover window at: ({}, {}) on monitor {}x{}",
            hover_x, hover_y, monitor.width, monitor.height
        );

        if let Err(e) =
            hover_window.set_position(tauri::Position::Physical(tauri::PhysicalPosition {
                x: hover_x,
                y: hover_y,
            }))
        {
            error!("Failed to set hover window position: {}", e);
        }
    }
}

/// Monitor cursor position and reposition hover window when cursor moves to different screen
pub async fn monitor_cursor_for_hover(hover_window: tauri::WebviewWindow) {
    let last_monitor_id = String::new();
    let last_cursor_x = 0.0;
    let last_cursor_y = 0.0;

    loop {
        // Very fast polling for maximum responsiveness - check every 16ms (~60fps)
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let active_monitor = ActiveMonitor::default();
        let (hover_x, hover_y) = active_monitor.calculate_position_for_percentage(
            tauri::PhysicalSize::new(50, 50),
            1.0,
            0.75,
        );

        let _ = hover_window.set_position(tauri::Position::Physical(tauri::PhysicalPosition {
            x: hover_x,
            y: hover_y,
        }));

        // if let Ok(cursor_position) = hover_window.cursor_position() {
        //     // Only proceed if cursor actually moved (avoid unnecessary work)
        //     if (cursor_position.x - last_cursor_x).abs() < 1.0
        //         && (cursor_position.y - last_cursor_y).abs() < 1.0
        //     {
        //         continue;
        //     }

        //     last_cursor_x = cursor_position.x;
        //     last_cursor_y = cursor_position.y;

        //     // Use consolidated monitor detection function
        //     if let Some(cursor_monitor) = find_cursor_monitor(&hover_window) {
        //         let monitor = &cursor_monitor.monitor;

        //         // If cursor moved to a different monitor, reposition hover window immediately
        //         if monitor.id != last_monitor_id {
        //             info!(
        //                 "Cursor moved to monitor: {} (immediate repositioning)",
        //                 monitor.id
        //             );
        //             last_monitor_id = monitor.id.clone();

        //             // Position hover window on the new monitor
        //             let window_size = hover_window.inner_size().unwrap_or(tauri::PhysicalSize {
        //                 width: 50,
        //                 height: 50,
        //             });

        //             let active_monitor = ActiveMonitor::default();
        //             let (hover_x, hover_y) = active_monitor.calculate_position_for_percentage(
        //                 tauri::PhysicalSize::new(50, 50),
        //                 1.0,
        //                 0.75,
        //             );

        //             // Calculate hover position using consolidated function
        //             // let (hover_x, hover_y) =
        //             //     active_monitor.calculate_position_for_percentage(window_size, 0.75, 0.75);

        //             info!(
        //                 "Repositioning hover window to: ({}, {}) on monitor {}x{}",
        //                 hover_x, hover_y, monitor.width, monitor.height
        //             );

        //             if let Err(e) = hover_window.set_position(tauri::Position::Physical(
        //                 tauri::PhysicalPosition {
        //                     x: hover_x,
        //                     y: hover_y,
        //                 },
        //             )) {
        //                 error!("Failed to reposition hover window: {}", e);
        //             }
        //         }
        //     }
        // }
    }
}

/// Get the current launcher visibility state
pub fn is_launcher_visible() -> bool {
    LAUNCHER_VISIBLE.load(Ordering::SeqCst)
}

/// Set the launcher visibility state
pub fn set_launcher_visible(visible: bool) {
    LAUNCHER_VISIBLE.store(visible, Ordering::SeqCst);
}
