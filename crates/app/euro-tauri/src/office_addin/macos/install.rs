//! macOS catalog backend. Drops the rendered Word manifest into Word's
//! sandboxed `wef/` directory under
//! `~/Library/Containers/com.microsoft.Word/Data/Documents`.

use std::fs;
use std::path::{Path, PathBuf};

use tauri::{AppHandle, Manager, Runtime};

use crate::office_addin::install::{MANIFEST_FILE, remove_if_exists};
use crate::office_addin::manifest;
use crate::office_addin::{Error, InstallOutcome, Result};

pub fn install_for_app<R: Runtime>(app: &AppHandle<R>) -> Result<InstallOutcome> {
    let xml = manifest::render_manifest_for_app(app)?;
    let home = app.path().home_dir().map_err(|source| Error::Path {
        kind: "home_dir",
        source,
    })?;
    install_at(&home, &xml)
}

pub fn uninstall_for_app<R: Runtime>(app: &AppHandle<R>) -> Result<()> {
    let home = app.path().home_dir().map_err(|source| Error::Path {
        kind: "home_dir",
        source,
    })?;
    uninstall_at(&home)
}

/// Resolve the manifest path via `dirs::home_dir` (no Tauri runtime
/// needed) and tear down whatever [`install_for_app`] would have
/// written. Returns the manifest path the operation targeted, even if
/// the file was already absent.
pub fn uninstall_standalone() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or(Error::DirsLookup { kind: "home_dir" })?;
    let manifest_path = documents_dir(&home).join("wef").join(MANIFEST_FILE);
    uninstall_at(&home)?;
    Ok(manifest_path)
}

fn documents_dir(home: &Path) -> PathBuf {
    home.join("Library/Containers/com.microsoft.Word/Data/Documents")
}

fn install_at(home: &Path, xml: &str) -> Result<InstallOutcome> {
    let documents = documents_dir(home);
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

fn uninstall_at(home: &Path) -> Result<()> {
    let manifest_path = documents_dir(home).join("wef").join(MANIFEST_FILE);
    remove_if_exists(&manifest_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn documents_dir_matches_documented_path() {
        assert_eq!(
            documents_dir(Path::new("/Users/alice")),
            PathBuf::from("/Users/alice/Library/Containers/com.microsoft.Word/Data/Documents")
        );
    }

    #[test]
    fn install_writes_manifest_when_documents_exists() {
        let tmp = tempdir().unwrap();
        let home = tmp.path();
        let documents = documents_dir(home);
        fs::create_dir_all(&documents).unwrap();

        let outcome = install_at(home, "<manifest>1</manifest>").unwrap();
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
        let outcome = install_at(tmp.path(), "<manifest/>").unwrap();
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
        fs::create_dir_all(documents_dir(home)).unwrap();

        let first = install_at(home, "<v1/>").unwrap();
        let second = install_at(home, "<v2/>").unwrap();

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
        fs::create_dir_all(documents_dir(home)).unwrap();
        install_at(home, "<v1/>").unwrap();

        let manifest_path = documents_dir(home).join("wef").join("com.eurora.word.xml");
        assert!(manifest_path.exists());

        uninstall_at(home).unwrap();
        assert!(!manifest_path.exists());
    }

    #[test]
    fn uninstall_is_idempotent_when_missing() {
        let tmp = tempdir().unwrap();
        uninstall_at(tmp.path()).unwrap();
        uninstall_at(tmp.path()).unwrap();
    }
}
