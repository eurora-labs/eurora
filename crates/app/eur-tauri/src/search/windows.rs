use super::AppInfo;
use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;

// Standard Start Menu locations
const START_MENU_LOCATIONS: [&str; 2] = [
    "C:\\ProgramData\\Microsoft\\Windows\\Start Menu\\Programs",
    "%APPDATA%\\Microsoft\\Windows\\Start Menu\\Programs",
];

/// Search for applications on Windows
pub async fn search_windows_apps(query: &str) -> Result<Vec<AppInfo>, String> {
    let query = query.to_lowercase();
    let mut results = Vec::new();

    // Search Start Menu for .lnk files
    for location in &START_MENU_LOCATIONS {
        let expanded_path = expand_path(location)?;
        search_directory_for_lnk_files(&expanded_path, &query, &mut results)?;
    }

    // Limit results to top 20 for performance
    results.truncate(20);

    Ok(results)
}

/// Expand environment variables in a path
fn expand_path(path: &str) -> Result<PathBuf, String> {
    let expanded = path.replace("%APPDATA%", &std::env::var("APPDATA").unwrap_or_default());
    Ok(PathBuf::from(expanded))
}

/// Recursively search a directory for .lnk files matching the query
fn search_directory_for_lnk_files(
    dir: &Path,
    query: &str,
    results: &mut Vec<AppInfo>,
) -> Result<(), String> {
    if !dir.exists() || !dir.is_dir() {
        return Ok(());
    }

    match std::fs::read_dir(dir) {
        Ok(entries) => {
            for entry in entries.flatten() {
                let path = entry.path();

                if path.is_dir() {
                    let _ = search_directory_for_lnk_files(&path, query, results);
                } else if let Some(ext) = path.extension() {
                    if ext == "lnk" {
                        if let Some(file_stem) = path.file_stem() {
                            if let Some(name) = file_stem.to_str() {
                                if name.to_lowercase().contains(query) {
                                    if let Ok(info) = extract_lnk_info(&path) {
                                        results.push(info);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        Err(_) => return Err(format!("Failed to read directory: {}", dir.display())),
    }

    Ok(())
}

/// Extract application information from a .lnk (shortcut) file
fn extract_lnk_info(lnk_path: &Path) -> Result<AppInfo, String> {
    // Use PowerShell to extract shortcut information
    let ps_script = format!(
        "
        $shell = New-Object -ComObject WScript.Shell;
        $shortcut = $shell.CreateShortcut('{}');
        
        @{{
            Path = $shortcut.TargetPath;
            Description = $shortcut.Description;
            Name = [System.IO.Path]::GetFileNameWithoutExtension('{}');
        }} | ConvertTo-Json
        ",
        lnk_path.display(),
        lnk_path.display()
    );

    let output = Command::new("powershell")
        .args(&["-Command", &ps_script])
        .output()
        .map_err(|e| format!("Failed to execute PowerShell: {}", e))?;

    if !output.status.success() {
        let mut stderr = String::new();
        output.stderr;
        return Err(format!("PowerShell error: {}", stderr));
    }

    let mut stdout = String::new();
    output.stdout;

    let data: serde_json::Value = serde_json::from_str(&stdout)
        .map_err(|e| format!("Failed to parse PowerShell output: {}", e))?;

    let name = data["Name"].as_str().unwrap_or_default().to_string();
    let path = data["Path"].as_str().unwrap_or_default().to_string();
    let description = data["Description"].as_str().map(String::from);

    // For simplicity, we're not extracting the icon now, but in a real implementation,
    // we would extract the icon and convert it to base64
    let icon = None;

    let mut metadata = HashMap::new();
    metadata.insert("shortcut_path".to_string(), lnk_path.display().to_string());

    Ok(AppInfo {
        name,
        path,
        description,
        icon,
        metadata: Some(metadata),
    })
}
