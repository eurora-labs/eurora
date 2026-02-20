use std::fmt;

use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorCode {
    InvalidPromptInput,
    InvalidToolResults,
    MessageCoercionFailure,
    ModelAuthentication,
    ModelNotFound,
    ModelRateLimit,
    OutputParsingFailure,
}

impl ErrorCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::InvalidPromptInput => "INVALID_PROMPT_INPUT",
            Self::InvalidToolResults => "INVALID_TOOL_RESULTS",
            Self::MessageCoercionFailure => "MESSAGE_COERCION_FAILURE",
            Self::ModelAuthentication => "MODEL_AUTHENTICATION",
            Self::ModelNotFound => "MODEL_NOT_FOUND",
            Self::ModelRateLimit => "MODEL_RATE_LIMIT",
            Self::OutputParsingFailure => "OUTPUT_PARSING_FAILURE",
        }
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

pub fn create_message(message: &str, error_code: ErrorCode) -> String {
    format!(
        "{message}
For troubleshooting, visit:          https://docs.langchain.com/oss/python/langchain/errors/{} ",
        error_code.as_str()
    )
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("{0}")]
    General(String),

    #[error("Tracer error: {0}")]
    Tracer(String),

    #[error("{message}")]
    OutputParser {
        message: String,
        observation: Option<String>,
        llm_output: Option<String>,
        send_to_llm: bool,
    },

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("API error ({status}): {message}")]
    Api { status: u16, message: String },

    #[error("Missing configuration: {0}")]
    MissingConfig(String),

    #[error("Unsupported provider: {0}")]
    UnsupportedProvider(String),

    #[error("Unable to infer provider for model '{0}'. Please specify model_provider explicitly.")]
    UnableToInferProvider(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("Tool invocation error: {0}")]
    ToolInvocation(String),

    #[error("{0}")]
    ToolException(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Indexing error: {0}")]
    Indexing(String),

    #[error("Not implemented: {0}")]
    NotImplemented(String),

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Retry exhausted: {0}")]
    RetryExhausted(String),

    #[error("Lock poisoned: {0}")]
    LockPoisoned(String),

    #[error("{0}")]
    Other(String),
}

impl Error {
    pub fn as_tool_exception(&self) -> Option<&str> {
        match self {
            Error::ToolException(msg) => Some(msg),
            Error::ToolInvocation(msg) => Some(msg),
            _ => None,
        }
    }

    pub fn as_validation_error(&self) -> Option<&str> {
        match self {
            Error::ValidationError(msg) => Some(msg),
            _ => None,
        }
    }

    pub fn output_parser(
        error: impl Into<String>,
        observation: Option<String>,
        llm_output: Option<String>,
        send_to_llm: bool,
    ) -> std::result::Result<Self, Self> {
        if send_to_llm && (observation.is_none() || llm_output.is_none()) {
            return Err(Self::InvalidConfig(
                "Arguments 'observation' & 'llm_output' are required if 'send_to_llm' is true"
                    .to_string(),
            ));
        }
        let message = create_message(&error.into(), ErrorCode::OutputParsingFailure);
        Ok(Self::OutputParser {
            message,
            observation,
            llm_output,
            send_to_llm,
        })
    }

    pub fn api(status: u16, message: impl Into<String>) -> Self {
        Self::Api {
            status,
            message: message.into(),
        }
    }

    pub fn missing_config(key: impl Into<String>) -> Self {
        Self::MissingConfig(key.into())
    }

    pub fn unsupported_provider(provider: impl Into<String>) -> Self {
        Self::UnsupportedProvider(provider.into())
    }

    pub fn unable_to_infer_provider(model: impl Into<String>) -> Self {
        Self::UnableToInferProvider(model.into())
    }

    pub fn other(message: impl Into<String>) -> Self {
        Self::Other(message.into())
    }

    pub fn is_retryable(&self) -> bool {
        match self {
            Self::Http(_) => true,
            Self::Timeout(_) => true,
            Self::Api { status, .. } => *status == 429 || *status >= 500,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_code_display() {
        assert_eq!(
            ErrorCode::OutputParsingFailure.to_string(),
            "OUTPUT_PARSING_FAILURE"
        );
        assert_eq!(
            ErrorCode::InvalidPromptInput.to_string(),
            "INVALID_PROMPT_INPUT"
        );
    }

    #[test]
    fn test_create_message() {
        let msg = create_message("bad output", ErrorCode::OutputParsingFailure);
        assert!(msg.contains("bad output"));
        assert!(msg.contains("OUTPUT_PARSING_FAILURE"));
        assert!(msg.contains("https://docs.langchain.com"));
    }

    #[test]
    fn test_output_parser_error() {
        let err = Error::output_parser("parse failed", None, None, false).unwrap();
        assert!(matches!(err, Error::OutputParser { .. }));
    }

    #[test]
    fn test_output_parser_error_send_to_llm_requires_fields() {
        let result = Error::output_parser("parse failed", None, None, true);
        assert!(result.is_err());
    }

    #[test]
    fn test_is_retryable() {
        assert!(Error::api(429, "rate limited").is_retryable());
        assert!(Error::api(500, "server error").is_retryable());
        assert!(!Error::api(400, "bad request").is_retryable());
        assert!(!Error::other("something").is_retryable());
    }
}
