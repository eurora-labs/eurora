//! Bundled Office add-in support for the desktop app.
//!
//! Two responsibilities:
//!
//! - **Manifest rendering** ([`render_manifest_for_app`]) — locate the bundled
//!   add-in tree under `resource_dir()`, load or generate the stable per-install
//!   add-in GUID, and render `manifest.template.xml` into a deployable XML
//!   string.
//! - **Catalog deployment** ([`install_for_app`] / [`uninstall_for_app`]) — drop
//!   the rendered manifest into the per-OS Office catalog (macOS WEF directory,
//!   Windows trusted-catalog registry) so Word picks it up on its next launch.

mod install;
mod manifest;

use std::path::PathBuf;

use thiserror::Error;

pub use install::{InstallOutcome, install_for_app, uninstall_for_app};
pub use manifest::render_manifest_for_app;

#[derive(Debug, Error)]
pub enum Error {
    #[error("could not resolve {kind} for office add-in: {source}")]
    Path {
        kind: &'static str,
        source: tauri::Error,
    },

    #[error("office add-in resource not found: {0}")]
    MissingResource(PathBuf),

    #[error("could not encode {0} as a file:// URL")]
    UrlEncode(PathBuf),

    #[error("could not parse desktop version `{value}`: {reason}")]
    Version { value: String, reason: String },

    #[error("manifest template references unknown token `{0}`")]
    UnknownToken(String),

    #[error("io error while {action} {path}: {source}")]
    Io {
        action: &'static str,
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[cfg(target_os = "windows")]
    #[error("registry write failed for HKCU\\{path}: {source}")]
    Registry {
        path: String,
        #[source]
        source: std::io::Error,
    },
}

pub type Result<T> = std::result::Result<T, Error>;
