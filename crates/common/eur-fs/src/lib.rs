use std::{
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result};
use bstr::BString;
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

// Return an iterator of worktree-relative slash-separated paths for files inside the `worktree_dir`, recursively.
// Fails if the `worktree_dir` isn't a valid git repository.
pub fn iter_worktree_files(
    worktree_dir: impl AsRef<Path>,
) -> Result<impl Iterator<Item = BString>> {
    let worktree_dir = worktree_dir.as_ref();

    // Check if it's a git repository by looking for .git directory or file
    let git_dir = worktree_dir.join(".git");
    if !git_dir.exists() {
        return Err(anyhow::anyhow!(
            "Not a git repository: {}",
            worktree_dir.display()
        ));
    }

    // Use git ls-files to get tracked files
    let output = Command::new("git")
        .arg("ls-files")
        .arg("--cached")
        .arg("--others")
        .arg("--exclude-standard")
        .current_dir(worktree_dir)
        .output()
        .context("Failed to execute git ls-files")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!("git ls-files failed: {}", stderr));
    }

    let stdout =
        String::from_utf8(output.stdout).context("git ls-files output is not valid UTF-8")?;

    let files: Vec<BString> = stdout
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| BString::from(line))
        .collect();

    Ok(files.into_iter())
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

    let temp_path = create_temp_file_path(parent_dir)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

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
