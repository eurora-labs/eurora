//! Standalone CLI to remove every artifact `install_for_app` could have
//! written: the rendered manifest under `%APPDATA%\Eurora\OfficeAddins\`
//! (Windows) or `~/Library/Containers/com.microsoft.Word/Data/Documents/wef/`
//! (macOS), plus the `HKCU\Software\Microsoft\Office\16.0\WEF\TrustedCatalogs`
//! subkey on Windows.
//!
//! Idempotent: safe to run when nothing is installed. Useful when a stale
//! dev install (e.g. left over from before `EURORA_OFFICE_ADDIN_DEV_SIDELOAD`
//! existed) is shadowing the Vite-served add-in inside Word.

use std::process::ExitCode;

use euro_tauri::office_addin::{UninstallOutcome, uninstall_standalone};

fn main() -> ExitCode {
    match uninstall_standalone() {
        Ok(UninstallOutcome::Cleaned { manifest_path }) => {
            println!(
                "Removed Office add-in manifest at {} (no-op if it didn't exist)",
                manifest_path.display()
            );
            #[cfg(target_os = "windows")]
            println!("Removed HKCU\\Software\\Microsoft\\Office\\16.0\\WEF\\TrustedCatalogs entry");
            println!("Restart Word for the change to take effect.");
            ExitCode::SUCCESS
        }
        Ok(UninstallOutcome::SkippedUnsupportedOs) => {
            println!("Nothing to clean: Microsoft Word does not run natively on this OS");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("office-addin-clean: {e}");
            ExitCode::FAILURE
        }
    }
}
