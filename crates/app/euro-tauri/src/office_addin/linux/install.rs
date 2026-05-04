//! Linux fallback for catalog deployment. Word doesn't run natively
//! here, so install and uninstall are both no-ops.

use tauri::{AppHandle, Runtime};

use crate::office_addin::{InstallOutcome, Result};

pub fn install_for_app<R: Runtime>(_app: &AppHandle<R>) -> Result<InstallOutcome> {
    Ok(InstallOutcome::SkippedUnsupportedOs)
}

pub fn uninstall_for_app<R: Runtime>(_app: &AppHandle<R>) -> Result<()> {
    Ok(())
}
