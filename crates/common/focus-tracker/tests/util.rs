//! Common test utilities for focus-tracker integration tests

use std::env;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

/// Spawn a test window using the helper binary
///
/// # Arguments
/// * `title` - The window title to set
///
/// # Returns
/// A `Child` process handle for the spawned window
pub fn spawn_test_window(title: &str) -> Result<Child, Box<dyn std::error::Error>> {
    let mut cmd = Command::new("cargo");
    cmd.args(["run", "--example", "spawn_window", "--"])
        .args(["--title", title])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let child = cmd.spawn()?;

    std::thread::sleep(Duration::from_millis(500));

    Ok(child)
}

/// Focus a window (platform-specific implementation)
///
/// # Arguments
/// * `child` - The child process handle of the window to focus
#[allow(dead_code)]
pub fn focus_window(child: &mut Child) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "linux")]
    {
        focus_window_linux(child)
    }

    #[cfg(target_os = "windows")]
    {
        focus_window_windows(child)
    }

    #[cfg(target_os = "macos")]
    {
        focus_window_macos(child)
    }

    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
    compile_error!("focus_window is not implemented for this platform");
}

#[cfg(target_os = "linux")]
#[allow(dead_code)]
fn focus_window_linux(child: &mut Child) -> Result<(), Box<dyn std::error::Error>> {
    use std::process::Command;

    let pid = child.id();

    if Command::new("wmctrl").arg("-l").output().is_ok() {
        let output = Command::new("wmctrl").args(["-l", "-p"]).output()?;

        let output_str = String::from_utf8_lossy(&output.stdout);

        for line in output_str.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3
                && let Ok(window_pid) = parts[2].parse::<u32>()
                && window_pid == pid
            {
                let window_id = parts[0];
                Command::new("wmctrl")
                    .args(["-i", "-a", window_id])
                    .output()?;
                return Ok(());
            }
        }
    }
    if Command::new("xdotool").arg("--version").status()?.success() {
        let ids = Command::new("xdotool")
            .args(["search", "--pid", &pid.to_string()])
            .output()?;
        if !ids.status.success() {
            return Err("xdotool search failed".into());
        }
        if let Some(id) = String::from_utf8_lossy(&ids.stdout).lines().next() {
            let status = Command::new("xdotool")
                .args(["windowactivate", id])
                .status()?;
            if status.success() {
                return Ok(());
            }
        }
    }
    Err("Unable to focus window – neither wmctrl nor xdotool succeeded".into())
}

#[cfg(target_os = "windows")]
fn focus_window_windows(_child: &mut Child) -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}

#[cfg(target_os = "macos")]
fn focus_window_macos(child: &mut Child) -> Result<(), Box<dyn std::error::Error>> {
    let pid = child.id();

    let script = format!(
        "tell application \"System Events\" to set frontmost of the first process whose unix id is {} to true",
        pid
    );

    let output = Command::new("osascript").args(["-e", &script]).output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("osascript failed to focus window (pid {pid}): {stderr}").into());
    }

    std::thread::sleep(Duration::from_millis(200));

    Ok(())
}

/// Wait for a window with the expected title to be focused
///
/// # Arguments
/// * `expected_title` - The title to wait for
/// * `timeout` - Maximum time to wait
///
/// # Returns
/// `true` if the window was focused within the timeout, `false` otherwise
#[allow(dead_code)]
pub fn wait_for_focus(expected_title: &str, timeout: Duration) -> bool {
    let start = Instant::now();

    while start.elapsed() < timeout {
        if let Ok(focused) = get_current_focused_window()
            && let Some(title) = focused.window_title
            && title.contains(expected_title)
        {
            return true;
        }

        std::thread::sleep(Duration::from_millis(100));
    }

    false
}

/// Get the currently focused window (for testing purposes)
fn get_current_focused_window() -> Result<focus_tracker::FocusedWindow, Box<dyn std::error::Error>>
{
    #[cfg(target_os = "linux")]
    {
        get_focused_window_linux()
    }

    #[cfg(target_os = "macos")]
    {
        get_focused_window_macos()
    }

    #[cfg(target_os = "windows")]
    {
        get_focused_window_windows()
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        Ok(focus_tracker::FocusedWindow {
            process_id: 0,
            process_name: "unknown".to_string(),
            window_title: Some("unknown".to_string()),
            icon: None,
        })
    }
}

#[cfg(target_os = "macos")]
fn get_focused_window_macos() -> Result<focus_tracker::FocusedWindow, Box<dyn std::error::Error>> {
    focus_tracker::utils::get_frontmost_window_basic_info()
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
}

#[cfg(target_os = "linux")]
fn get_focused_window_linux() -> Result<focus_tracker::FocusedWindow, Box<dyn std::error::Error>> {
    use std::process::Command;

    if let Ok(output) = Command::new("xdotool")
        .args(["getwindowfocus", "getwindowname"])
        .output()
    {
        let title = String::from_utf8_lossy(&output.stdout).trim().to_string();
        return Ok(focus_tracker::FocusedWindow {
            process_id: 0,
            process_name: "unknown".to_string(),
            window_title: Some(title),
            icon: None,
        });
    }

    Ok(focus_tracker::FocusedWindow {
        process_id: 0,
        process_name: "unknown".to_string(),
        window_title: Some("unknown".to_string()),
        icon: None,
    })
}

#[cfg(target_os = "windows")]
fn get_focused_window_windows() -> Result<focus_tracker::FocusedWindow, Box<dyn std::error::Error>>
{
    Ok(focus_tracker::FocusedWindow {
        process_id: 0,
        process_name: "unknown".to_string(),
        window_title: Some("unknown".to_string()),
        icon: None,
    })
}

/// Check if integration tests should run
///
/// Tests will only run if INTEGRATION_TEST=1 environment variable is set
pub fn should_run_integration_tests() -> bool {
    env::var("INTEGRATION_TEST")
        .map(|v| v == "1")
        .unwrap_or(false)
}

/// Check if we should use Wayland backend
///
/// Returns true if WAYLAND=1 environment variable is set
pub fn should_use_wayland() -> bool {
    env::var("WAYLAND").map(|v| v == "1").unwrap_or(false)
}

/// Check if we should use X11 backend
///
/// Returns true if X11=1 environment variable is set
pub fn should_use_x11() -> bool {
    env::var("X11").map(|v| v == "1").unwrap_or(false)
}

/// Setup test environment based on flags
#[allow(dead_code)]
pub fn setup_test_environment() -> Result<(), Box<dyn std::error::Error>> {
    if !should_run_integration_tests() {
        return Err("Integration tests disabled. Set INTEGRATION_TEST=1 to enable.".into());
    }

    if should_use_wayland() {
        unsafe {
            env::set_var("WAYLAND_DISPLAY", "wayland-test");
        }
        tracing::info!("Using Wayland backend for tests");
    } else if should_use_x11() {
        unsafe {
            env::set_var("DISPLAY", ":99");
        }
        tracing::info!("Using X11 backend for tests");
    }

    Ok(())
}

/// Cleanup function to terminate child processes
#[allow(dead_code)]
pub fn cleanup_child_process(mut child: Child) -> Result<(), Box<dyn std::error::Error>> {
    if child.kill().is_err() {
        // If kill fails, the process might have already exited
    }

    let _ = child.wait();

    Ok(())
}

/// Spawn a window with a specific title (simplified interface for tests)
#[allow(dead_code)]
pub fn spawn_window(title: &str) -> Result<Child, Box<dyn std::error::Error>> {
    spawn_test_window(title)
}

/// Get the currently focused window
#[allow(dead_code)]
pub fn get_focused_window() -> focus_tracker::FocusedWindow {
    get_current_focused_window().unwrap_or_else(|_| focus_tracker::FocusedWindow {
        process_id: 0,
        process_name: "unknown".to_string(),
        window_title: Some("unknown".to_string()),
        icon: None,
    })
}

/// Cleanup multiple child processes
#[allow(dead_code)]
pub fn cleanup(win_a: Child, win_b: Child) {
    let _ = cleanup_child_process(win_a);
    let _ = cleanup_child_process(win_b);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_environment_flags() {
        unsafe {
            env::set_var("INTEGRATION_TEST", "1");
            assert!(should_run_integration_tests());

            env::set_var("WAYLAND", "1");
            assert!(should_use_wayland());

            env::set_var("X11", "1");
            assert!(should_use_x11());

            env::remove_var("INTEGRATION_TEST");
            env::remove_var("WAYLAND");
            env::remove_var("X11");
        }
    }
}
