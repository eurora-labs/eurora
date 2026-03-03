use std::fmt;

#[derive(Debug)]
pub enum AppError {
    Io(std::io::Error),
    Endpoint(euro_endpoint::EndpointError),
    Encrypt(euro_encrypt::EncryptError),
    Anyhow(anyhow::Error),
    UrlParse(url::ParseError),
    Json(serde_json::Error),
    Unavailable(&'static str),
    Msg(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "{e}"),
            Self::Endpoint(e) => write!(f, "{e}"),
            Self::Encrypt(e) => write!(f, "{e}"),
            Self::Anyhow(e) => write!(f, "{e}"),
            Self::UrlParse(e) => write!(f, "{e}"),
            Self::Json(e) => write!(f, "{e}"),
            Self::Unavailable(name) => write!(f, "{name} not available"),
            Self::Msg(e) => write!(f, "{e}"),
        }
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<euro_endpoint::EndpointError> for AppError {
    fn from(e: euro_endpoint::EndpointError) -> Self {
        Self::Endpoint(e)
    }
}

impl From<euro_encrypt::EncryptError> for AppError {
    fn from(e: euro_encrypt::EncryptError) -> Self {
        Self::Encrypt(e)
    }
}

impl From<anyhow::Error> for AppError {
    fn from(e: anyhow::Error) -> Self {
        Self::Anyhow(e)
    }
}

impl From<url::ParseError> for AppError {
    fn from(e: url::ParseError) -> Self {
        Self::UrlParse(e)
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        Self::Json(e)
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
