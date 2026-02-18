//! Path utilities for converting file paths to import paths.

use std::path::{Path, PathBuf};

/// Get the path of the file as a relative path to a base directory.
///
/// # Arguments
///
/// * `file` - The file path to convert.
/// * `relative_to` - The base path to make the file path relative to.
///
/// # Returns
///
/// The relative path as a `PathBuf`, or `None` if the path cannot be made relative.
///
/// # Example
///
/// ```
/// use std::path::Path;
/// use agent_chain_core::api::get_relative_path;
///
/// let base = Path::new("/home/user/project");
/// let file = Path::new("/home/user/project/src/main.rs");
/// let relative = get_relative_path(file, base);
/// assert_eq!(relative, Some("src/main.rs".into()));
/// ```
pub fn get_relative_path<P: AsRef<Path>, B: AsRef<Path>>(
    file: P,
    relative_to: B,
) -> Option<String> {
    let file = file.as_ref();
    let base = relative_to.as_ref();

    file.strip_prefix(base)
        .ok()
        .map(|p| p.to_string_lossy().into_owned())
}

/// Convert a file path to an import path (module path format).
///
/// This converts a file path like `src/api/path.rs` to a module path like `api::path`.
///
/// # Arguments
///
/// * `file` - The file path to convert.
/// * `suffix` - An optional suffix to append to the import path.
/// * `relative_to` - The base path to make the file path relative to.
///
/// # Returns
///
/// The import path as a `String`, or `None` if the conversion fails.
///
/// # Example
///
/// ```
/// use std::path::Path;
/// use agent_chain_core::api::as_import_path;
///
/// let base = Path::new("/home/user/project/src");
/// let file = Path::new("/home/user/project/src/api/path.rs");
/// let import = as_import_path(file, None, base);
/// assert_eq!(import, Some("api::path".to_string()));
/// ```
pub fn as_import_path<P: AsRef<Path>, B: AsRef<Path>>(
    file: P,
    suffix: Option<&str>,
    relative_to: B,
) -> Option<String> {
    let file = file.as_ref();
    let relative_path = get_relative_path(file, relative_to)?;

    let path = PathBuf::from(&relative_path);

    let without_extension = if path.extension().is_some() {
        path.with_extension("")
    } else {
        path
    };

    let import_path = without_extension
        .to_string_lossy()
        .replace([std::path::MAIN_SEPARATOR, '/', '\\'], "::");

    if let Some(suffix) = suffix {
        Some(format!("{}::{}", import_path, suffix))
    } else {
        Some(import_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_get_relative_path() {
        let base = Path::new("/home/user/project");
        let file = Path::new("/home/user/project/src/main.rs");
        let relative = get_relative_path(file, base);
        assert_eq!(relative, Some("src/main.rs".to_string()));
    }

    #[test]
    fn test_get_relative_path_same_dir() {
        let base = Path::new("/home/user/project");
        let file = Path::new("/home/user/project/main.rs");
        let relative = get_relative_path(file, base);
        assert_eq!(relative, Some("main.rs".to_string()));
    }

    #[test]
    fn test_get_relative_path_not_relative() {
        let base = Path::new("/home/user/project");
        let file = Path::new("/other/path/main.rs");
        let relative = get_relative_path(file, base);
        assert_eq!(relative, None);
    }

    #[test]
    fn test_as_import_path() {
        let base = Path::new("/home/user/project/src");
        let file = Path::new("/home/user/project/src/api/path.rs");
        let import = as_import_path(file, None, base);
        assert_eq!(import, Some("api::path".to_string()));
    }

    #[test]
    fn test_as_import_path_with_suffix() {
        let base = Path::new("/home/user/project/src");
        let file = Path::new("/home/user/project/src/api/path.rs");
        let import = as_import_path(file, Some("MyStruct"), base);
        assert_eq!(import, Some("api::path::MyStruct".to_string()));
    }

    #[test]
    fn test_as_import_path_directory() {
        let base = Path::new("/home/user/project/src");
        let file = Path::new("/home/user/project/src/api");
        let import = as_import_path(file, None, base);
        assert_eq!(import, Some("api".to_string()));
    }
}
