//! Bundled Office add-in support for the desktop app.
//!
//! Three responsibilities:
//!
//! - **Manifest rendering** ([`render_manifest_for_app`]) — locate the bundled
//!   add-in tree under `resource_dir()`, load or generate the stable per-install
//!   add-in GUID, and render `manifest.template.xml` into a deployable XML
//!   string.
//! - **Catalog deployment** ([`install_for_app`] / [`uninstall_for_app`]) — drop
//!   the rendered manifest into the per-OS Office catalog (macOS WEF directory,
//!   Windows trusted-catalog registry) so Word picks it up on its next launch.
//! - **TLS bridge material** ([`bridge_certs`]) — mint and persist the local
//!   `Eurora Local Bridge CA` plus a `localhost` leaf, install the CA into the
//!   per-user OS root store, and tear them down on uninstall. The Word add-in
//!   (and the native-messaging host) connect over `wss://localhost:1431/bridge`
//!   using this trust chain.

pub mod bridge_certs;
mod install;
mod manifest;

cfg_select! {
    target_os = "macos" => {
        mod macos;
        use macos as platform;
    }
    target_os = "windows" => {
        mod windows;
        use windows as platform;
    }
    _ => {
        mod linux;
        use linux as platform;
    }
}

use std::path::PathBuf;

use thiserror::Error;

pub use install::{
    InstallOutcome, UninstallOutcome, install_for_app, uninstall_for_app, uninstall_standalone,
};
pub use manifest::render_manifest_for_app;

#[derive(Debug, Error)]
pub enum Error {
    #[error("could not resolve {kind} for office add-in: {source}")]
    Path {
        kind: &'static str,
        source: tauri::Error,
    },

    #[error("could not resolve {kind} via the dirs crate")]
    DirsLookup { kind: &'static str },

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

    #[error("failed to generate bridge certificate: {0}")]
    CertGenerate(#[from] rcgen::Error),

    #[error("failed to parse bridge certificate at {path}: {reason}")]
    CertParse { path: PathBuf, reason: String },
}

pub type Result<T> = std::result::Result<T, Error>;
