//! Per-OS deployment of the rendered add-in manifest into the Office catalog.
//!
//! Idempotent on every launch: a clean reinstall and a no-op reinstall converge
//! on the same final state. Failures never crash the desktop; the caller logs
//! and continues.

use std::path::PathBuf;
#[cfg(any(target_os = "macos", target_os = "windows"))]
use std::{fs, path::Path};

#[cfg(any(target_os = "macos", target_os = "windows"))]
use tauri::Manager;
use tauri::{AppHandle, Runtime};

use super::Result;
#[cfg(any(target_os = "macos", target_os = "windows"))]
use super::{Error, manifest};

/// File name used for the deployed Word manifest inside the Office catalog.
#[cfg(any(target_os = "macos", target_os = "windows"))]
const MANIFEST_FILE: &str = "com.eurora.word.xml";

/// Stable catalog GUID written to
/// `HKCU\Software\Microsoft\Office\16.0\WEF\TrustedCatalogs\{GUID}`. Hardcoded
/// so reinstalls reuse the same registry subkey and uninstall can clean it up
/// without persisting extra state.
#[cfg(target_os = "windows")]
const TRUSTED_CATALOG_GUID: &str = "{8a4e6c52-3d0f-4a7b-9e2d-1f5c7b8e0a91}";

/// Outcome of an install attempt. Distinguishes "did not apply on this OS" from
/// "Word's per-user state isn't ready yet" so the caller can log the right
/// severity.
#[derive(Debug, Clone)]
pub enum InstallOutcome {
    /// Manifest was written; Word will pick it up on its next launch.
    Installed { manifest_path: PathBuf },
    /// macOS only: Word's sandboxed container has not been created yet
    /// (Word has never been launched on this user account). The desktop
    /// will retry on its next launch.
    SkippedHostNotPresent,
    /// Linux/other: Word does not run natively here.
    SkippedUnsupportedOs,
}

#[cfg(target_os = "macos")]
pub fn install_for_app<R: Runtime>(app: &AppHandle<R>) -> Result<InstallOutcome> {
    let xml = manifest::render_manifest_for_app(app)?;
    let home = app.path().home_dir().map_err(|source| Error::Path {
        kind: "home_dir",
        source,
    })?;
    install_macos_at(&home, &xml)
}

#[cfg(target_os = "macos")]
pub fn uninstall_for_app<R: Runtime>(app: &AppHandle<R>) -> Result<()> {
    let home = app.path().home_dir().map_err(|source| Error::Path {
        kind: "home_dir",
        source,
    })?;
    uninstall_macos_at(&home)
}

#[cfg(target_os = "windows")]
pub fn install_for_app<R: Runtime>(app: &AppHandle<R>) -> Result<InstallOutcome> {
    let xml = manifest::render_manifest_for_app(app)?;
    let appdata = app.path().data_dir().map_err(|source| Error::Path {
        kind: "data_dir",
        source,
    })?;
    let outcome = install_windows_files(&appdata, &xml)?;
    if let InstallOutcome::Installed { manifest_path } = &outcome {
        let dir = manifest_path
            .parent()
            .expect("manifest path constructed with a parent dir");
        let url = url::Url::from_directory_path(dir)
            .map_err(|()| Error::UrlEncode(dir.to_path_buf()))?
            .to_string();
        register_trusted_catalog(TRUSTED_CATALOG_GUID, &url)?;
    }
    Ok(outcome)
}

#[cfg(target_os = "windows")]
pub fn uninstall_for_app<R: Runtime>(app: &AppHandle<R>) -> Result<()> {
    let appdata = app.path().data_dir().map_err(|source| Error::Path {
        kind: "data_dir",
        source,
    })?;
    uninstall_windows_files(&appdata)?;
    deregister_trusted_catalog(TRUSTED_CATALOG_GUID)
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub fn install_for_app<R: Runtime>(_app: &AppHandle<R>) -> Result<InstallOutcome> {
    Ok(InstallOutcome::SkippedUnsupportedOs)
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub fn uninstall_for_app<R: Runtime>(_app: &AppHandle<R>) -> Result<()> {
    Ok(())
}

// ---------------------------------------------------------------------------
// macOS — `~/Library/Containers/com.microsoft.Word/Data/Documents/wef/…`
// ---------------------------------------------------------------------------

#[cfg(target_os = "macos")]
fn macos_documents_dir(home: &Path) -> PathBuf {
    home.join("Library/Containers/com.microsoft.Word/Data/Documents")
}

#[cfg(target_os = "macos")]
fn install_macos_at(home: &Path, xml: &str) -> Result<InstallOutcome> {
    let documents = macos_documents_dir(home);
    if !documents.exists() {
        // Word's container is created the first time Word launches under
        // this account. Auto-creating the container path here would race
        // Office's own provisioning, so defer instead.
        return Ok(InstallOutcome::SkippedHostNotPresent);
    }
    let wef = documents.join("wef");
    fs::create_dir_all(&wef).map_err(|source| Error::Io {
        action: "creating",
        path: wef.clone(),
        source,
    })?;
    let manifest_path = wef.join(MANIFEST_FILE);
    fs::write(&manifest_path, xml).map_err(|source| Error::Io {
        action: "writing",
        path: manifest_path.clone(),
        source,
    })?;
    Ok(InstallOutcome::Installed { manifest_path })
}

#[cfg(target_os = "macos")]
fn uninstall_macos_at(home: &Path) -> Result<()> {
    let manifest_path = macos_documents_dir(home).join("wef").join(MANIFEST_FILE);
    remove_if_exists(&manifest_path)
}

// ---------------------------------------------------------------------------
// Windows — `%APPDATA%\Eurora\OfficeAddins\…` plus a trusted-catalog entry
// under `HKCU\Software\Microsoft\Office\16.0\WEF\TrustedCatalogs\{GUID}`.
// ---------------------------------------------------------------------------

#[cfg(target_os = "windows")]
fn windows_manifest_dir(appdata: &Path) -> PathBuf {
    appdata.join("Eurora").join("OfficeAddins")
}

#[cfg(target_os = "windows")]
fn install_windows_files(appdata: &Path, xml: &str) -> Result<InstallOutcome> {
    let dir = windows_manifest_dir(appdata);
    fs::create_dir_all(&dir).map_err(|source| Error::Io {
        action: "creating",
        path: dir.clone(),
        source,
    })?;
    let manifest_path = dir.join(MANIFEST_FILE);
    fs::write(&manifest_path, xml).map_err(|source| Error::Io {
        action: "writing",
        path: manifest_path.clone(),
        source,
    })?;
    Ok(InstallOutcome::Installed { manifest_path })
}

#[cfg(target_os = "windows")]
fn uninstall_windows_files(appdata: &Path) -> Result<()> {
    let manifest_path = windows_manifest_dir(appdata).join(MANIFEST_FILE);
    remove_if_exists(&manifest_path)
}

#[cfg(target_os = "windows")]
fn register_trusted_catalog(guid: &str, url: &str) -> Result<()> {
    use winreg::RegKey;
    use winreg::enums::HKEY_CURRENT_USER;

    let key_path = format!(r"Software\Microsoft\Office\16.0\WEF\TrustedCatalogs\{guid}");
    let registry_err = |source: std::io::Error| Error::Registry {
        path: key_path.clone(),
        source,
    };

    let (key, _) = RegKey::predef(HKEY_CURRENT_USER)
        .create_subkey(&key_path)
        .map_err(&registry_err)?;
    key.set_value("Url", &url.to_owned())
        .map_err(&registry_err)?;
    key.set_value("Id", &guid.to_owned())
        .map_err(&registry_err)?;
    key.set_value("Flags", &1u32).map_err(&registry_err)?;
    key.set_value("ShowInMenu", &1u32).map_err(&registry_err)?;
    Ok(())
}

#[cfg(target_os = "windows")]
fn deregister_trusted_catalog(guid: &str) -> Result<()> {
    use winreg::RegKey;
    use winreg::enums::HKEY_CURRENT_USER;

    let key_path = format!(r"Software\Microsoft\Office\16.0\WEF\TrustedCatalogs\{guid}");
    match RegKey::predef(HKEY_CURRENT_USER).delete_subkey_all(&key_path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(Error::Registry {
            path: key_path,
            source,
        }),
    }
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

#[cfg(any(target_os = "macos", target_os = "windows"))]
fn remove_if_exists(path: &Path) -> Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(Error::Io {
            action: "removing",
            path: path.to_path_buf(),
            source,
        }),
    }
}

#[cfg(test)]
mod tests {
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    use super::*;

    #[cfg(target_os = "macos")]
    mod macos {
        use super::*;
        use std::path::Path;
        use tempfile::tempdir;

        #[test]
        fn documents_dir_matches_documented_path() {
            assert_eq!(
                macos_documents_dir(Path::new("/Users/alice")),
                PathBuf::from("/Users/alice/Library/Containers/com.microsoft.Word/Data/Documents")
            );
        }

        #[test]
        fn install_writes_manifest_when_documents_exists() {
            let tmp = tempdir().unwrap();
            let home = tmp.path();
            let documents = macos_documents_dir(home);
            fs::create_dir_all(&documents).unwrap();

            let outcome = install_macos_at(home, "<manifest>1</manifest>").unwrap();
            let manifest_path = match outcome {
                InstallOutcome::Installed { manifest_path } => manifest_path,
                other => panic!("expected Installed, got {other:?}"),
            };

            assert_eq!(
                manifest_path,
                documents.join("wef").join("com.eurora.word.xml")
            );
            assert_eq!(
                fs::read_to_string(&manifest_path).unwrap(),
                "<manifest>1</manifest>"
            );
        }

        #[test]
        fn install_skips_when_documents_missing() {
            let tmp = tempdir().unwrap();
            let outcome = install_macos_at(tmp.path(), "<manifest/>").unwrap();
            assert!(matches!(outcome, InstallOutcome::SkippedHostNotPresent));

            // Verify nothing was created under the fake home — we must not
            // race Office's own container provisioning.
            let entries: Vec<_> = fs::read_dir(tmp.path()).unwrap().collect();
            assert!(
                entries.is_empty(),
                "expected nothing created in fake home, found {} entries",
                entries.len()
            );
        }

        #[test]
        fn install_is_idempotent_and_overwrites_content() {
            let tmp = tempdir().unwrap();
            let home = tmp.path();
            fs::create_dir_all(macos_documents_dir(home)).unwrap();

            let first = install_macos_at(home, "<v1/>").unwrap();
            let second = install_macos_at(home, "<v2/>").unwrap();

            let (first_path, second_path) = match (first, second) {
                (
                    InstallOutcome::Installed {
                        manifest_path: first_path,
                    },
                    InstallOutcome::Installed {
                        manifest_path: second_path,
                    },
                ) => (first_path, second_path),
                other => panic!("expected two Installed outcomes, got {other:?}"),
            };

            assert_eq!(first_path, second_path);
            assert_eq!(fs::read_to_string(&second_path).unwrap(), "<v2/>");
        }

        #[test]
        fn uninstall_removes_existing_manifest() {
            let tmp = tempdir().unwrap();
            let home = tmp.path();
            fs::create_dir_all(macos_documents_dir(home)).unwrap();
            install_macos_at(home, "<v1/>").unwrap();

            let manifest_path = macos_documents_dir(home)
                .join("wef")
                .join("com.eurora.word.xml");
            assert!(manifest_path.exists());

            uninstall_macos_at(home).unwrap();
            assert!(!manifest_path.exists());
        }

        #[test]
        fn uninstall_is_idempotent_when_missing() {
            let tmp = tempdir().unwrap();
            uninstall_macos_at(tmp.path()).unwrap();
            uninstall_macos_at(tmp.path()).unwrap();
        }
    }

    #[cfg(target_os = "windows")]
    mod windows {
        use super::*;
        use std::path::Path;
        use tempfile::tempdir;

        #[test]
        fn manifest_dir_matches_documented_path() {
            assert_eq!(
                windows_manifest_dir(Path::new(r"C:\Users\alice\AppData\Roaming")),
                PathBuf::from(r"C:\Users\alice\AppData\Roaming\Eurora\OfficeAddins")
            );
        }

        #[test]
        fn install_files_writes_manifest() {
            let tmp = tempdir().unwrap();
            let outcome = install_windows_files(tmp.path(), "<v1/>").unwrap();
            let manifest_path = match outcome {
                InstallOutcome::Installed { manifest_path } => manifest_path,
                other => panic!("expected Installed, got {other:?}"),
            };

            assert_eq!(
                manifest_path,
                tmp.path()
                    .join("Eurora")
                    .join("OfficeAddins")
                    .join("com.eurora.word.xml")
            );
            assert_eq!(fs::read_to_string(&manifest_path).unwrap(), "<v1/>");
        }

        #[test]
        fn install_files_is_idempotent_and_overwrites_content() {
            let tmp = tempdir().unwrap();
            install_windows_files(tmp.path(), "<v1/>").unwrap();
            let outcome = install_windows_files(tmp.path(), "<v2/>").unwrap();
            let manifest_path = match outcome {
                InstallOutcome::Installed { manifest_path } => manifest_path,
                other => panic!("expected Installed, got {other:?}"),
            };
            assert_eq!(fs::read_to_string(&manifest_path).unwrap(), "<v2/>");
        }

        #[test]
        fn uninstall_files_removes_existing_manifest() {
            let tmp = tempdir().unwrap();
            install_windows_files(tmp.path(), "<v1/>").unwrap();
            let manifest_path = windows_manifest_dir(tmp.path()).join("com.eurora.word.xml");
            assert!(manifest_path.exists());

            uninstall_windows_files(tmp.path()).unwrap();
            assert!(!manifest_path.exists());
        }

        #[test]
        fn uninstall_files_is_idempotent_when_missing() {
            let tmp = tempdir().unwrap();
            uninstall_windows_files(tmp.path()).unwrap();
            uninstall_windows_files(tmp.path()).unwrap();
        }
    }
}
