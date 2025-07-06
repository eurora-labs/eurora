use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone)]
pub struct Storage {
    local_data_dir: PathBuf,
}

impl Storage {
    pub fn new(local_data_dir: impl Into<PathBuf>) -> Storage {
        Storage {
            local_data_dir: local_data_dir.into(),
        }
    }

    pub fn read(&self, relative_path: impl AsRef<Path>) -> std::io::Result<Option<String>> {
        let full_path = self.local_data_dir.join(relative_path);

        if !full_path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(full_path)?;
        Ok(Some(content))
    }

    pub fn write(&self, relative_path: impl AsRef<Path>, content: &str) -> std::io::Result<()> {
        let full_path = self.local_data_dir.join(relative_path);

        // Create parent directories if they don't exist
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(full_path, content)
    }

    pub fn delete(&self, relative_path: impl AsRef<Path>) -> std::io::Result<()> {
        let full_path = self.local_data_dir.join(relative_path);

        if full_path.exists() {
            fs::remove_file(full_path)?;
        }

        Ok(())
    }
}
