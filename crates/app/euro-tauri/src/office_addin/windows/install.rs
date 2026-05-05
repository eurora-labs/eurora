//! Windows catalog backend. Writes the rendered manifest under
//! `%APPDATA%\Eurora\OfficeAddins\` and registers the directory as a
//! trusted catalog under
//! `HKCU\Software\Microsoft\Office\16.0\WEF\TrustedCatalogs\{GUID}`.

use std::fs;
use std::path::{Path, PathBuf};

use tauri::{AppHandle, Manager, Runtime};

use crate::office_addin::install::{MANIFEST_FILE, remove_if_exists};
use crate::office_addin::manifest;
use crate::office_addin::{Error, InstallOutcome, Result};

/// Stable catalog GUID written to
/// `HKCU\Software\Microsoft\Office\16.0\WEF\TrustedCatalogs\{GUID}`. Hardcoded
/// so reinstalls reuse the same registry subkey and uninstall can clean it up
/// without persisting extra state.
const TRUSTED_CATALOG_GUID: &str = "{8a4e6c52-3d0f-4a7b-9e2d-1f5c7b8e0a91}";

pub fn install_for_app<R: Runtime>(app: &AppHandle<R>) -> Result<InstallOutcome> {
    let xml = manifest::render_manifest_for_app(app)?;
    let appdata = app.path().data_dir().map_err(|source| Error::Path {
        kind: "data_dir",
        source,
    })?;
    let outcome = install_files(&appdata, &xml)?;
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

pub fn uninstall_for_app<R: Runtime>(app: &AppHandle<R>) -> Result<()> {
    let appdata = app.path().data_dir().map_err(|source| Error::Path {
        kind: "data_dir",
        source,
    })?;
    uninstall_files(&appdata)?;
    deregister_trusted_catalog(TRUSTED_CATALOG_GUID)
}

/// Resolve the manifest path via `dirs::data_dir` (no Tauri runtime
/// needed) and tear down both the manifest file and the
/// `TrustedCatalogs\{GUID}` subkey. Returns the manifest path the
/// operation targeted, even if the file was already absent.
pub fn uninstall_standalone() -> Result<PathBuf> {
    let appdata = dirs::data_dir().ok_or(Error::DirsLookup { kind: "data_dir" })?;
    let manifest_path = manifest_dir(&appdata).join(MANIFEST_FILE);
    uninstall_files(&appdata)?;
    deregister_trusted_catalog(TRUSTED_CATALOG_GUID)?;
    Ok(manifest_path)
}

fn manifest_dir(appdata: &Path) -> PathBuf {
    appdata.join("Eurora").join("OfficeAddins")
}

fn install_files(appdata: &Path, xml: &str) -> Result<InstallOutcome> {
    let dir = manifest_dir(appdata);
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

fn uninstall_files(appdata: &Path) -> Result<()> {
    let manifest_path = manifest_dir(appdata).join(MANIFEST_FILE);
    remove_if_exists(&manifest_path)
}

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
    tracing::info!("Registered Office trusted catalog under HKCU\\{key_path} -> {url}");
    Ok(())
}

fn deregister_trusted_catalog(guid: &str) -> Result<()> {
    use winreg::RegKey;
    use winreg::enums::HKEY_CURRENT_USER;

    let key_path = format!(r"Software\Microsoft\Office\16.0\WEF\TrustedCatalogs\{guid}");
    match RegKey::predef(HKEY_CURRENT_USER).delete_subkey_all(&key_path) {
        Ok(()) => {
            tracing::info!("Removed Office trusted catalog under HKCU\\{key_path}");
            Ok(())
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            tracing::debug!("No Office trusted catalog at HKCU\\{key_path}; nothing to remove");
            Ok(())
        }
        Err(source) => Err(Error::Registry {
            path: key_path,
            source,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn manifest_dir_matches_documented_path() {
        assert_eq!(
            manifest_dir(Path::new(r"C:\Users\alice\AppData\Roaming")),
            PathBuf::from(r"C:\Users\alice\AppData\Roaming\Eurora\OfficeAddins")
        );
    }

    #[test]
    fn install_files_writes_manifest() {
        let tmp = tempdir().unwrap();
        let outcome = install_files(tmp.path(), "<v1/>").unwrap();
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
        install_files(tmp.path(), "<v1/>").unwrap();
        let outcome = install_files(tmp.path(), "<v2/>").unwrap();
        let manifest_path = match outcome {
            InstallOutcome::Installed { manifest_path } => manifest_path,
            other => panic!("expected Installed, got {other:?}"),
        };
        assert_eq!(fs::read_to_string(&manifest_path).unwrap(), "<v2/>");
    }

    #[test]
    fn uninstall_files_removes_existing_manifest() {
        let tmp = tempdir().unwrap();
        install_files(tmp.path(), "<v1/>").unwrap();
        let manifest_path = manifest_dir(tmp.path()).join("com.eurora.word.xml");
        assert!(manifest_path.exists());

        uninstall_files(tmp.path()).unwrap();
        assert!(!manifest_path.exists());
    }

    #[test]
    fn uninstall_files_is_idempotent_when_missing() {
        let tmp = tempdir().unwrap();
        uninstall_files(tmp.path()).unwrap();
        uninstall_files(tmp.path()).unwrap();
    }
}
