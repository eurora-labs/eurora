use tauri::{Manager, Runtime};
use tauri_plugin_global_shortcut::GlobalShortcutExt;
use tracing::{error, info};

use crate::util::*;

#[taurpc::procedures(path = "settings")]
pub trait SettingsApi {}

#[derive(Clone)]
pub struct SettingsApiImpl;

#[taurpc::resolvers]
impl SettingsApi for SettingsApiImpl {}
