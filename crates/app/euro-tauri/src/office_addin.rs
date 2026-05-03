//! Bundled Office add-in support for the desktop app.
//!
//! Phase 4 owns: locating the bundled add-in tree under `resource_dir()`,
//! loading or generating a stable per-install GUID, and rendering
//! `manifest.template.xml` into a deployable `manifest.xml`. Phase 5 wires
//! the rendered XML into the per-OS Office catalog (macOS WEF dir, Windows
//! trusted-catalog registry).

use std::{
    fs,
    path::{Path, PathBuf},
};

use tauri::{AppHandle, Manager, Runtime};
use thiserror::Error;
use url::Url;
use uuid::Uuid;

/// Subdirectory inside `resource_dir()` where the add-in bundle ships.
pub const RESOURCE_SUBDIR: &str = "office-addin";

/// Subdirectory inside `app_data_dir()` for state we own.
const STATE_SUBDIR: &str = "office-addin";

/// File holding the stable per-install add-in GUID.
const ADDIN_ID_FILE: &str = "manifest-id";

/// Resolved on-disk paths for the bundled Office add-in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddinPaths {
    pub resource_root: PathBuf,
    pub runtime_html: PathBuf,
    pub icons_dir: PathBuf,
    pub manifest_template: PathBuf,
}

/// Substitution values for `manifest.template.xml`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestParams {
    /// `file:///…/runtime.html`.
    pub source_location: String,
    /// Hyphenated lowercase UUID.
    pub addin_id: String,
    /// Office requires a 4-part dotted version (`x.y.z.w`).
    pub version: String,
    /// `file:///…/icons/` — trailing slash required.
    pub icon_base_url: String,
}

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
}

pub type Result<T> = std::result::Result<T, Error>;

/// Resolve paths to the bundled add-in resources. Verifies that the bundle
/// is actually present — returns [`Error::MissingResource`] otherwise.
pub fn resolve_paths<R: Runtime>(app: &AppHandle<R>) -> Result<AddinPaths> {
    let resource_dir = app.path().resource_dir().map_err(|source| Error::Path {
        kind: "resource_dir",
        source,
    })?;
    let resource_root = resource_dir.join(RESOURCE_SUBDIR);
    let runtime_html = resource_root.join("runtime.html");
    let icons_dir = resource_root.join("icons");
    let manifest_template = resource_root.join("manifest.template.xml");

    for required in [&runtime_html, &manifest_template] {
        if !required.exists() {
            return Err(Error::MissingResource(required.clone()));
        }
    }

    Ok(AddinPaths {
        resource_root,
        runtime_html,
        icons_dir,
        manifest_template,
    })
}

/// Read the persisted add-in GUID, generating and persisting a fresh one on
/// first call. The GUID lives under `app_data_dir()/office-addin/manifest-id`
/// so it survives reinstalls (Office keys add-in identity off this value).
pub fn load_or_create_addin_id<R: Runtime>(app: &AppHandle<R>) -> Result<String> {
    let data_dir = app.path().app_data_dir().map_err(|source| Error::Path {
        kind: "app_data_dir",
        source,
    })?;
    let state_dir = data_dir.join(STATE_SUBDIR);
    let id_path = state_dir.join(ADDIN_ID_FILE);
    load_or_create_addin_id_at(&state_dir, &id_path)
}

fn load_or_create_addin_id_at(state_dir: &Path, id_path: &Path) -> Result<String> {
    if let Some(existing) = read_existing_id(id_path)? {
        return Ok(existing);
    }

    fs::create_dir_all(state_dir).map_err(|source| Error::Io {
        action: "creating",
        path: state_dir.to_path_buf(),
        source,
    })?;
    let id = Uuid::new_v4().hyphenated().to_string();
    fs::write(id_path, &id).map_err(|source| Error::Io {
        action: "writing",
        path: id_path.to_path_buf(),
        source,
    })?;
    Ok(id)
}

fn read_existing_id(id_path: &Path) -> Result<Option<String>> {
    match fs::read_to_string(id_path) {
        Ok(contents) => {
            let trimmed = contents.trim();
            if trimmed.is_empty() {
                Ok(None)
            } else {
                Ok(Some(trimmed.to_owned()))
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(Error::Io {
            action: "reading",
            path: id_path.to_path_buf(),
            source,
        }),
    }
}

/// Build [`ManifestParams`] from the running app. Couples the four substitution
/// values to a single source of truth: bundle layout, persisted GUID, and the
/// desktop's compiled-in version.
pub fn manifest_params<R: Runtime>(
    app: &AppHandle<R>,
    paths: &AddinPaths,
) -> Result<ManifestParams> {
    let source_location = path_to_file_url(&paths.runtime_html, false)?;
    let icon_base_url = path_to_file_url(&paths.icons_dir, true)?;
    let addin_id = load_or_create_addin_id(app)?;
    let version = office_version(&app.package_info().version.to_string())?;

    Ok(ManifestParams {
        source_location,
        addin_id,
        version,
        icon_base_url,
    })
}

/// Substitute `{{TOKEN}}` placeholders in `template` using `params`.
/// Errors on any unknown token rather than silently leaving placeholders
/// behind — matches the strictness of `scripts/render-manifest.mjs`.
pub fn render_manifest(template: &str, params: &ManifestParams) -> Result<String> {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;

    while let Some(start) = rest.find("{{") {
        out.push_str(&rest[..start]);
        let after_open = &rest[start + 2..];
        let Some(end) = after_open.find("}}") else {
            // Unterminated `{{` — leave the rest verbatim. This mirrors the JS
            // renderer's regex behaviour (no match => no substitution).
            out.push_str(&rest[start..]);
            return Ok(out);
        };
        let token = &after_open[..end];
        out.push_str(substitute(token, params)?);
        rest = &after_open[end + 2..];
    }

    out.push_str(rest);
    Ok(out)
}

/// Convenience: render the bundled template using the current app's params.
pub fn render_manifest_for_app<R: Runtime>(app: &AppHandle<R>) -> Result<String> {
    let paths = resolve_paths(app)?;
    let template = fs::read_to_string(&paths.manifest_template).map_err(|source| Error::Io {
        action: "reading",
        path: paths.manifest_template.clone(),
        source,
    })?;
    let params = manifest_params(app, &paths)?;
    render_manifest(&template, &params)
}

fn substitute<'a>(token: &str, params: &'a ManifestParams) -> Result<&'a str> {
    match token {
        "SOURCE_LOCATION" => Ok(&params.source_location),
        "ADDIN_ID" => Ok(&params.addin_id),
        "VERSION" => Ok(&params.version),
        "ICON_BASE_URL" => Ok(&params.icon_base_url),
        other => Err(Error::UnknownToken(other.to_owned())),
    }
}

fn path_to_file_url(path: &Path, as_directory: bool) -> Result<String> {
    let result = if as_directory {
        Url::from_directory_path(path)
    } else {
        Url::from_file_path(path)
    };
    result
        .map(|url| url.to_string())
        .map_err(|()| Error::UrlEncode(path.to_path_buf()))
}

/// Pad a semver `x.y.z` to Office's mandatory 4-part `x.y.z.w` form.
/// Inputs already in 4-part form are returned unchanged. Anything else
/// (non-numeric, wrong arity, pre-release suffix) is rejected.
fn office_version(raw: &str) -> Result<String> {
    let core = raw.split('-').next().unwrap_or(raw);
    let parts: Vec<&str> = core.split('.').collect();
    let valid_arity = matches!(parts.len(), 3 | 4);
    let all_numeric = parts
        .iter()
        .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()));
    if !valid_arity || !all_numeric {
        return Err(Error::Version {
            value: raw.to_owned(),
            reason: "expected `x.y.z` or `x.y.z.w` with numeric components".to_owned(),
        });
    }
    if parts.len() == 4 {
        Ok(core.to_owned())
    } else {
        Ok(format!("{core}.0"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn sample_params() -> ManifestParams {
        ManifestParams {
            source_location: "file:///opt/eurora/office-addin/runtime.html".to_owned(),
            addin_id: "12345678-1234-1234-1234-1234567890ab".to_owned(),
            version: "1.2.3.0".to_owned(),
            icon_base_url: "file:///opt/eurora/office-addin/icons/".to_owned(),
        }
    }

    #[test]
    fn render_manifest_substitutes_all_known_tokens() {
        let template =
            "id={{ADDIN_ID}} ver={{VERSION}} src={{SOURCE_LOCATION}} icons={{ICON_BASE_URL}}";
        let rendered = render_manifest(template, &sample_params()).unwrap();
        assert_eq!(
            rendered,
            "id=12345678-1234-1234-1234-1234567890ab ver=1.2.3.0 \
             src=file:///opt/eurora/office-addin/runtime.html \
             icons=file:///opt/eurora/office-addin/icons/"
        );
    }

    #[test]
    fn render_manifest_rejects_unknown_token() {
        let err = render_manifest("hello {{NOPE}}", &sample_params()).unwrap_err();
        assert!(
            matches!(&err, Error::UnknownToken(t) if t == "NOPE"),
            "{err}"
        );
    }

    #[test]
    fn render_manifest_preserves_non_token_text_byte_for_byte() {
        let template = "<root attr=\"value with } and { braces\">{{VERSION}}</root>\n";
        let rendered = render_manifest(template, &sample_params()).unwrap();
        assert_eq!(
            rendered,
            "<root attr=\"value with } and { braces\">1.2.3.0</root>\n"
        );
    }

    #[test]
    fn render_manifest_handles_unterminated_open_braces() {
        let template = "before {{ stays as-is";
        let rendered = render_manifest(template, &sample_params()).unwrap();
        assert_eq!(rendered, template);
    }

    #[test]
    fn render_manifest_real_template_round_trip() {
        let template = include_str!("../../../../apps/office-addin/manifest.template.xml");
        let rendered = render_manifest(template, &sample_params()).unwrap();
        assert!(!rendered.contains("{{"), "no token left behind: {rendered}");
        assert!(rendered.contains("12345678-1234-1234-1234-1234567890ab"));
        assert!(rendered.contains("1.2.3.0"));
        assert!(rendered.contains("file:///opt/eurora/office-addin/runtime.html"));
    }

    #[test]
    fn load_or_create_addin_id_persists_across_calls() {
        let tmp = tempdir().unwrap();
        let state_dir = tmp.path().join("office-addin");
        let id_path = state_dir.join("manifest-id");

        let first = load_or_create_addin_id_at(&state_dir, &id_path).unwrap();
        let second = load_or_create_addin_id_at(&state_dir, &id_path).unwrap();
        assert_eq!(first, second);

        let on_disk = fs::read_to_string(&id_path).unwrap();
        assert_eq!(on_disk.trim(), first);

        Uuid::parse_str(&first).expect("persisted id is a valid uuid");
    }

    #[test]
    fn load_or_create_addin_id_treats_blank_file_as_missing() {
        let tmp = tempdir().unwrap();
        let state_dir = tmp.path().join("office-addin");
        fs::create_dir_all(&state_dir).unwrap();
        let id_path = state_dir.join("manifest-id");
        fs::write(&id_path, "   \n").unwrap();

        let id = load_or_create_addin_id_at(&state_dir, &id_path).unwrap();
        Uuid::parse_str(&id).expect("regenerated id is a valid uuid");
        assert_eq!(fs::read_to_string(&id_path).unwrap().trim(), id);
    }

    #[test]
    fn office_version_pads_three_part_semver() {
        assert_eq!(office_version("1.2.3").unwrap(), "1.2.3.0");
    }

    #[test]
    fn office_version_passes_through_four_part() {
        assert_eq!(office_version("1.2.3.4").unwrap(), "1.2.3.4");
    }

    #[test]
    fn office_version_strips_prerelease_then_pads() {
        assert_eq!(office_version("1.2.3-rc.1").unwrap(), "1.2.3.0");
    }

    #[test]
    fn office_version_rejects_non_numeric_components() {
        assert!(office_version("1.2.x").is_err());
        assert!(office_version("1.2").is_err());
        assert!(office_version("1.2.3.4.5").is_err());
        assert!(office_version("").is_err());
    }
}
