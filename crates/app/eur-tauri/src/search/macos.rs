use super::AppInfo;
use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;

const APPLICATIONS_DIR: &str = "/Applications";
const USER_APPLICATIONS_DIR: &str = "~/Applications";

/// Search for applications on macOS
pub async fn search_macos_apps(query: &str) -> Result<Vec<AppInfo>, String> {
    let query = query.to_lowercase();
    let mut results = Vec::new();

    // Method 1: Search the Applications folders
    search_applications_dir(APPLICATIONS_DIR, &query, &mut results)?;

    // Expand user applications path
    if let Ok(home) = std::env::var("HOME") {
        let user_apps = USER_APPLICATIONS_DIR.replace("~", &home);
        search_applications_dir(&user_apps, &query, &mut results)?;
    }

    // Method 2: Use mdfind (Spotlight) to search for applications
    search_with_mdfind(&query, &mut results)?;

    // Remove duplicates based on path
    results.sort_by(|a, b| a.name.cmp(&b.name));
    results.dedup_by(|a, b| a.path == b.path);

    // Limit results to top 20 for performance
    results.truncate(20);

    Ok(results)
}

/// Search for .app bundles in a directory
fn search_applications_dir(
    dir: &str,
    query: &str,
    results: &mut Vec<AppInfo>,
) -> Result<(), String> {
    let dir_path = Path::new(dir);
    if !dir_path.exists() || !dir_path.is_dir() {
        return Ok(());
    }

    match std::fs::read_dir(dir_path) {
        Ok(entries) => {
            for entry in entries.flatten() {
                let path = entry.path();

                if path.is_dir() {
                    if let Some(ext) = path.extension() {
                        if ext == "app" {
                            if let Some(name) = path.file_stem() {
                                if let Some(name_str) = name.to_str() {
                                    if name_str.to_lowercase().contains(query) {
                                        if let Ok(info) = extract_app_info(&path) {
                                            results.push(info);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        Err(_) => return Err(format!("Failed to read directory: {}", dir)),
    }

    Ok(())
}

/// Search for applications using mdfind (Spotlight)
fn search_with_mdfind(query: &str, results: &mut Vec<AppInfo>) -> Result<(), String> {
    // Use mdfind to search for applications
    let mdfind_command = format!(
        "mdfind 'kMDItemKind == \"Application\" && kMDItemDisplayName ==\"*{}*\"c'",
        query
    );

    let output = Command::new("sh")
        .args(&["-c", &mdfind_command])
        .output()
        .map_err(|e| format!("Failed to execute mdfind: {}", e))?;

    if !output.status.success() {
        let mut stderr = String::new();
        output.stderr;
        return Err(format!("mdfind error: {}", stderr));
    }

    let mut stdout = String::new();
    output.stdout;

    for line in stdout.lines() {
        let path = line.trim();
        if path.ends_with(".app") && Path::new(path).exists() {
            if let Ok(info) = extract_app_info(&PathBuf::from(path)) {
                results.push(info);
            }
        }
    }

    Ok(())
}

/// Extract application information from an .app bundle
fn extract_app_info(app_path: &Path) -> Result<AppInfo, String> {
    // Read Info.plist to get application metadata
    let info_plist_path = app_path.join("Contents/Info.plist");
    if !info_plist_path.exists() {
        return Err(format!(
            "Info.plist not found for app: {}",
            app_path.display()
        ));
    }

    let name = app_path
        .file_stem()
        .and_then(|s| s.to_str())
        .map(String::from)
        .unwrap_or_else(|| "Unknown App".to_string());

    // Get application executable path
    let executable_path = format!("{}/Contents/MacOS/{}", app_path.display(), name);

    // Read CFBundleDisplayName from Info.plist using plutil
    let display_name_cmd = format!(
        "plutil -extract CFBundleDisplayName raw -o - \"{}\"",
        info_plist_path.display()
    );

    let display_name_output = Command::new("sh")
        .args(&["-c", &display_name_cmd])
        .output()
        .ok();

    let display_name = if let Some(output) = display_name_output {
        if output.status.success() {
            let mut stdout = String::new();
            output.stdout;
            if !stdout.is_empty() {
                stdout.trim().to_string()
            } else {
                name.clone()
            }
        } else {
            name.clone()
        }
    } else {
        name.clone()
    };

    // Create metadata
    let mut metadata = HashMap::new();
    metadata.insert("bundle_path".to_string(), app_path.display().to_string());

    Ok(AppInfo {
        name: display_name,
        path: app_path.display().to_string(),
        description: Some(format!("Application: {}", name)),
        icon: None, // In a real implementation, we would extract the app icon
        metadata: Some(metadata),
    })
}
