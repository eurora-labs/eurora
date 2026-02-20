use std::path::{Path, PathBuf};

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
