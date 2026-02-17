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
        Self::validate_path(relative_path.as_ref())?;
        let full_path = self.local_data_dir.join(relative_path);

        if !full_path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(full_path)?;
        Ok(Some(content))
    }

    pub fn write(&self, relative_path: impl AsRef<Path>, content: &str) -> std::io::Result<()> {
        Self::validate_path(relative_path.as_ref())?;
        let full_path = self.local_data_dir.join(relative_path);

        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(full_path, content)
    }

    pub fn delete(&self, relative_path: impl AsRef<Path>) -> std::io::Result<()> {
        Self::validate_path(relative_path.as_ref())?;
        let full_path = self.local_data_dir.join(relative_path);

        if full_path.exists() {
            fs::remove_file(full_path)?;
        }

        Ok(())
    }

    fn validate_path(relative_path: &Path) -> std::io::Result<()> {
        if relative_path
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Path traversal detected",
            ));
        }
        Ok(())
    }
}
