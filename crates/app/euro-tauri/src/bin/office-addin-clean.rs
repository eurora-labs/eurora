//! Standalone CLI to remove every artifact `install_for_app` /
//! `bridge_certs::ensure` could have written:
//!
//! - the rendered Word manifest under
//!   `%APPDATA%\Eurora\OfficeAddins\` (Windows) or
//!   `~/Library/Containers/com.microsoft.Word/Data/Documents/wef/` (macOS),
//! - the `HKCU\…\TrustedCatalogs` subkey on Windows,
//! - the bridge TLS material under `<data_dir>/Eurora/bridge/`,
//! - the `Eurora Local Bridge CA` from the per-user OS root store.
//!
//! Idempotent: safe to run when nothing is installed. Useful when a
//! stale dev install (e.g. left over from before
//! `EURORA_OFFICE_ADDIN_DEV_SIDELOAD` existed) is shadowing the
//! Vite-served add-in inside Word, or when the trust chain needs to be
//! reset.

use std::process::ExitCode;

use euro_tauri::office_addin::{UninstallOutcome, bridge_certs, uninstall_standalone};

fn main() -> ExitCode {
    match uninstall_standalone() {
        Ok(UninstallOutcome::Cleaned {
            manifest_path,
            ca_trust,
        }) => {
            println!(
                "Removed Office add-in manifest at {} (no-op if it didn't exist)",
                manifest_path.display()
            );
            #[cfg(target_os = "windows")]
            println!("Removed HKCU\\Software\\Microsoft\\Office\\16.0\\WEF\\TrustedCatalogs entry");
            match ca_trust {
                bridge_certs::TrustOutcome::Installed => {
                    println!("Removed Eurora bridge CA from per-user root store")
                }
                bridge_certs::TrustOutcome::AlreadyTrusted => {
                    println!("Eurora bridge CA already absent from per-user root store")
                }
                bridge_certs::TrustOutcome::Skipped => {
                    println!("CA trust removal not applicable on this OS")
                }
                bridge_certs::TrustOutcome::Failed(reason) => {
                    eprintln!(
                        "Warning: failed to remove Eurora bridge CA from root store: {reason}"
                    )
                }
            }
            println!("Removed bridge TLS material under <data_dir>/Eurora/bridge/");
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
