use std::{
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use serde::de::DeserializeOwned;
use walkdir::WalkDir;

// Returns an ordered list of relative paths for files inside a directory recursively.
pub fn list_files<P: AsRef<Path>>(
    dir_path: P,
    ignore_prefixes: &[P],
    recursive: bool,
    remove_prefix: Option<P>,
) -> Result<Vec<PathBuf>> {
    let mut files = vec![];
    let dir_path = dir_path.as_ref();
    if !dir_path.exists() {
        return Ok(files);
    }

    for entry in WalkDir::new(dir_path).max_depth(if recursive { usize::MAX } else { 1 }) {
        let entry = entry?;
        if !entry.file_type().is_dir() {
            let path = entry.path();

            let path = if let Some(prefix) = remove_prefix.as_ref() {
                path.strip_prefix(prefix)?
            } else {
                path
            };

            let path = path.to_path_buf();
            if ignore_prefixes
                .iter()
                .any(|prefix| path.starts_with(prefix.as_ref()))
            {
                continue;
            }
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

/// Write a single file so that the write either fully succeeds, or fully fails,
/// assuming the containing directory already exists.
pub fn write<P: AsRef<Path>>(file_path: P, contents: impl AsRef<[u8]>) -> anyhow::Result<()> {
    let file_path = file_path.as_ref();
    let parent_dir = file_path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("File path has no parent directory"))?;

    // Create a temporary file in the same directory
    let temp_path = create_temp_file_path(parent_dir)?;

    // Write to temporary file
    fs::write(&temp_path, contents.as_ref())
        .with_context(|| format!("Failed to write to temporary file: {}", temp_path.display()))?;

    // Atomically move temporary file to final location
    fs::rename(&temp_path, file_path).with_context(|| {
        format!(
            "Failed to move temporary file to final location: {}",
            file_path.display()
        )
    })?;

    Ok(())
}

/// Write a single file so that the write either fully succeeds, or fully fails,
/// and create all leading directories.
pub fn create_dirs_then_write<P: AsRef<Path>>(
    file_path: P,
    contents: impl AsRef<[u8]>,
) -> std::io::Result<()> {
    let file_path = file_path.as_ref();

    // Create all parent directories
    if let Some(parent_dir) = file_path.parent() {
        fs::create_dir_all(parent_dir)?;
    }

    // Create a temporary file in the same directory
    let parent_dir = file_path.parent().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "File path has no parent directory",
        )
    })?;

    let temp_path = create_temp_file_path(parent_dir).map_err(std::io::Error::other)?;

    // Write to temporary file
    fs::write(&temp_path, contents.as_ref())?;

    // Atomically move temporary file to final location
    fs::rename(&temp_path, file_path)?;

    Ok(())
}

/// Create a unique temporary file path in the given directory
fn create_temp_file_path(dir: &Path) -> anyhow::Result<PathBuf> {
    use std::time::{SystemTime, UNIX_EPOCH};

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();

    let process_id = std::process::id();
    let temp_name = format!(".tmp_{}__{}", process_id, timestamp);

    Ok(dir.join(temp_name))
}

/// Reads and parses the state file.
///
/// If the file does not exist, it will be created.
pub fn read_toml_file_or_default<T: DeserializeOwned + Default>(path: &Path) -> Result<T> {
    let mut file = match File::open(path) {
        Ok(f) => f,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(T::default()),
        Err(err) => return Err(err.into()),
    };
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    let value: T =
        toml::from_str(&contents).with_context(|| format!("Failed to parse {}", path.display()))?;
    Ok(value)
}
