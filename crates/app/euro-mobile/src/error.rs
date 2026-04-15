use std::fmt;

#[derive(Debug)]
pub enum AppError {
    Unavailable(&'static str),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unavailable(name) => write!(f, "{name} not available"),
        }
    }
}

impl From<AppError> for String {
    fn from(e: AppError) -> Self {
        e.to_string()
    }
}

pub(crate) trait ResultExt<T> {
    fn ctx(self, msg: &str) -> Result<T, String>;
}

impl<T, E: fmt::Display> ResultExt<T> for Result<T, E> {
    fn ctx(self, msg: &str) -> Result<T, String> {
        self.map_err(|e| format!("{msg}: {e}"))
    }
}
