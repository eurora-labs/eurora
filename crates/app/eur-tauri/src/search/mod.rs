mod linux;
mod macos;
mod windows;

use crate::search::linux::search_linux_apps as linux_search;
use crate::search::macos::search_macos_apps as macos_search;
use crate::search::windows::search_windows_apps as windows_search;

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct AppInfo {
    name: String,
    path: String,
    description: Option<String>,
    icon: Option<String>,
    metadata: Option<std::collections::HashMap<String, String>>,
}

#[tauri::command]
pub async fn search_windows_apps(query: String) -> Result<Vec<AppInfo>, String> {
    windows_search(&query).await
}

#[tauri::command]
pub async fn search_macos_apps(query: String) -> Result<Vec<AppInfo>, String> {
    macos_search(&query).await
}

#[tauri::command]
pub async fn launch_application(path: String) -> Result<(), String> {
    match open::that(&path) {
        Ok(_) => Ok(()),
        Err(err) => Err(format!("Failed to launch application: {}", err)),
    }
}
