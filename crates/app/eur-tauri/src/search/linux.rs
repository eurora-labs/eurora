use super::AppInfo;
use gtk::gio::{AppInfo as GtkAppInfo, DesktopAppInfo};
use gtk::prelude::*;
use regex::Regex;
use std::collections::HashMap;
use std::path::PathBuf;
use tracing::{debug, error, info};

/// Search for applications on Linux using GTK's AppInfo API
pub async fn search_linux_apps(query: &str) -> Result<Vec<AppInfo>, String> {
    info!("Searching for Linux apps with query: {}", query);
    let query = query.to_lowercase();
    let mut results = Vec::new();

    // Create a regex to clean up exec commands
    let re = match Regex::new(r"%[uUfFdDnNickvm]") {
        Ok(re) => re,
        Err(e) => {
            error!("Failed to create regex: {}", e);
            return Err(format!("Failed to create regex: {}", e));
        }
    };

    // Get all applications from GTK
    debug!("Reading applications through GTK AppInfo API");
    for app in GtkAppInfo::all() {
        // Skip apps that shouldn't be shown
        if !app.should_show() {
            continue;
        }

        // Get app name and convert to lowercase for search
        let name = app.display_name().to_string().to_lowercase();
        let description = match app.description() {
            Some(desc) => desc.to_string().to_lowercase(),
            None => String::new(),
        };

        // Skip if neither name nor description match the query
        if query.is_empty() || name.contains(&query) || description.contains(&query) {
            // Get command line
            if let Some(exec) = app.commandline() {
                // Get app id (desktop file name)
                if let Some(desktop_file) = app.id() {
                    // Clean up the exec command
                    let exec_clean = re.replace_all(&exec.display().to_string(), "").to_string();

                    // Parse command into arguments
                    let executable = match shell_words::split(&exec_clean) {
                        Ok(args) if !args.is_empty() => args[0].clone(),
                        _ => exec_clean.clone(),
                    };

                    // Get icon
                    let icon = if app.icon().is_none() {
                        None
                    } else {
                        match gtk::prelude::IconExt::to_string(&app.icon().unwrap()) {
                            Some(icon_name) => Some(icon_name.to_string()),
                            None => None,
                        }
                    };

                    // Check if app should run in terminal
                    let mut metadata = HashMap::new();
                    metadata.insert("desktop_file".to_string(), desktop_file.to_string());

                    // Get terminal property from DesktopAppInfo
                    if let Some(desktop_app) = DesktopAppInfo::new(&desktop_file) {
                        if desktop_app.boolean("Terminal") {
                            metadata.insert("terminal".to_string(), "true".to_string());
                        }

                        // Get categories if available
                        if let Some(categories) = desktop_app.categories() {
                            metadata.insert("categories".to_string(), categories.to_string());
                        }

                        // Get keywords if available (for better search)
                        if let keywords = desktop_app.keywords() {
                            let keywords_str: String = keywords
                                .iter()
                                .map(|s| s.to_string().to_lowercase())
                                .collect();
                            if !keywords_str.is_empty() && keywords_str.contains(&query) {
                                // If we're here only because of keywords match, we already checked query above
                            }
                        }
                    }

                    // Create and add AppInfo
                    results.push(AppInfo {
                        name: app.display_name().to_string(),
                        path: executable,
                        description: app.description().map(|d| d.to_string()),
                        icon,
                        metadata: Some(metadata),
                    });
                }
            }
        }
    }

    // Sort by name and remove duplicates
    results.sort_by(|a, b| a.name.cmp(&b.name));
    results.dedup_by(|a, b| a.name == b.name);

    info!("Found {} matching Linux applications", results.len());

    // Limit results to top 20 for performance
    results.truncate(20);

    Ok(results)
}
