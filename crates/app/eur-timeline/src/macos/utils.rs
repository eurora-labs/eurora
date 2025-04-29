use std::process::Command;

/// Get the name of the frontmost application using AppleScript
pub fn get_frontmost_app_name() -> Option<String> {
    let output = Command::new("osascript")
        .arg("-e")
        .arg("tell application \"System Events\" to get name of first application process whose frontmost is true")
        .output()
        .ok()?;

    if output.status.success() {
        let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !name.is_empty() {
            return Some(name);
        }
    }

    None
}

/// Get the title of the frontmost window using AppleScript
pub fn get_frontmost_window_title() -> Option<String> {
    let script = r#"
    tell application "System Events"
        set frontApp to first application process whose frontmost is true
        set frontAppName to name of frontApp
        
        tell process frontAppName
            try
                set windowTitle to name of first window
                return windowTitle
            on error
                return ""
            end try
        end tell
    end tell
    "#;

    let output = Command::new("osascript")
        .arg("-e")
        .arg(script)
        .output()
        .ok()?;

    if output.status.success() {
        let title = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !title.is_empty() {
            return Some(title);
        }
    }

    None
}

/// Check if the application is running in a sandboxed environment
pub fn is_app_sandboxed() -> bool {
    // Check for the App Sandbox environment variable
    std::env::var("APP_SANDBOX_CONTAINER_ID").is_ok()
}

/// Get the bundle identifier for a given application name
pub fn get_bundle_id_for_app(app_name: &str) -> Option<String> {
    let script = format!(
        r#"
        try
            tell application "Finder"
                set appPath to POSIX path of (application file "{}" as alias)
            end tell
            
            do shell script "mdls -name kMDItemCFBundleIdentifier -raw " & quoted form of appPath
        on error
            return ""
        end try
        "#,
        app_name
    );

    let output = Command::new("osascript")
        .arg("-e")
        .arg(&script)
        .output()
        .ok()?;

    if output.status.success() {
        let bundle_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !bundle_id.is_empty() {
            return Some(bundle_id);
        }
    }

    None
}
