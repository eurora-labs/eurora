use thiserror::Error;

#[derive(Debug, Error)]
pub enum EmailError {
    #[error("Email sending failed: {0}")]
    Send(String),
    #[error("Email configuration error: {0}")]
    Config(String),
}
