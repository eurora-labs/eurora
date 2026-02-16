//! Error types for the remote database system

use thiserror::Error;

#[derive(Error, Debug)]
pub enum DbError {
    #[error("{entity} not found{}", .id.as_ref().map(|id| format!(": {}", id)).unwrap_or_default())]
    NotFound {
        entity: &'static str,
        id: Option<String>,
    },

    #[error("Duplicate {field}: {value}")]
    Duplicate { field: &'static str, value: String },

    #[error("Referenced {entity} does not exist")]
    ForeignKeyViolation { entity: &'static str },

    #[error("Database connection error: {0}")]
    Connection(String),

    #[error("Migration error: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),

    #[error("Connection pool error: {0}")]
    Pool(String),

    #[error("Transaction error: {0}")]
    Transaction(String),

    #[error("Query error: {0}")]
    Query(String),

    #[error("Data encoding error: {0}")]
    Encoding(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Authentication error: {0}")]
    Authentication(String),

    #[error("Token error: {0}")]
    Token(String),

    #[error("Database error: {0}")]
    Database(#[source] sqlx::Error),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl DbError {
    pub fn not_found(entity: &'static str) -> Self {
        Self::NotFound { entity, id: None }
    }

    pub fn not_found_with_id(entity: &'static str, id: impl Into<String>) -> Self {
        Self::NotFound {
            entity,
            id: Some(id.into()),
        }
    }

    pub fn duplicate(field: &'static str, value: impl Into<String>) -> Self {
        Self::Duplicate {
            field,
            value: value.into(),
        }
    }

    pub fn foreign_key(entity: &'static str) -> Self {
        Self::ForeignKeyViolation { entity }
    }

    pub fn connection(msg: impl Into<String>) -> Self {
        Self::Connection(msg.into())
    }

    pub fn pool(msg: impl Into<String>) -> Self {
        Self::Pool(msg.into())
    }

    pub fn transaction(msg: impl Into<String>) -> Self {
        Self::Transaction(msg.into())
    }

    pub fn query(msg: impl Into<String>) -> Self {
        Self::Query(msg.into())
    }

    pub fn encoding(msg: impl Into<String>) -> Self {
        Self::Encoding(msg.into())
    }

    pub fn invalid_input(msg: impl Into<String>) -> Self {
        Self::InvalidInput(msg.into())
    }

    pub fn authentication(msg: impl Into<String>) -> Self {
        Self::Authentication(msg.into())
    }

    pub fn token(msg: impl Into<String>) -> Self {
        Self::Token(msg.into())
    }

    pub fn is_not_found(&self) -> bool {
        matches!(self, Self::NotFound { .. })
    }

    pub fn is_duplicate(&self) -> bool {
        matches!(self, Self::Duplicate { .. })
    }
}

impl From<sqlx::Error> for DbError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => Self::NotFound {
                entity: "record",
                id: None,
            },
            sqlx::Error::Database(db_err) => {
                // PostgreSQL error codes
                // 23505 = unique_violation
                // 23503 = foreign_key_violation
                // 23502 = not_null_violation
                if let Some(code) = db_err.code() {
                    match code.as_ref() {
                        "23505" => {
                            // Try to extract constraint name for better error messages
                            let constraint = db_err.constraint().unwrap_or("unknown").to_string();
                            Self::Duplicate {
                                field: "constraint",
                                value: constraint,
                            }
                        }
                        "23503" => {
                            let entity_name = db_err
                                .constraint()
                                .unwrap_or("referenced record")
                                .to_string();
                            Self::Query(format!("Foreign key violation: {}", entity_name))
                        }
                        _ => Self::Database(sqlx::Error::Database(db_err)),
                    }
                } else {
                    Self::Database(sqlx::Error::Database(db_err))
                }
            }
            sqlx::Error::PoolTimedOut => Self::Pool("Connection pool timed out".to_string()),
            sqlx::Error::PoolClosed => Self::Pool("Connection pool is closed".to_string()),
            sqlx::Error::Io(io_err) => Self::Connection(io_err.to_string()),
            sqlx::Error::Tls(tls_err) => Self::Connection(format!("TLS error: {}", tls_err)),
            other => Self::Database(other),
        }
    }
}

pub type DbResult<T> = std::result::Result<T, DbError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_not_found_error() {
        let err = DbError::not_found("user");
        assert!(err.is_not_found());
        assert_eq!(err.to_string(), "user not found");

        let err_with_id = DbError::not_found_with_id("user", "123");
        assert!(err_with_id.is_not_found());
        assert_eq!(err_with_id.to_string(), "user not found: 123");
    }

    #[test]
    fn test_duplicate_error() {
        let err = DbError::duplicate("email", "test@example.com");
        assert!(err.is_duplicate());
        assert_eq!(err.to_string(), "Duplicate email: test@example.com");
    }

    #[test]
    fn test_connection_error() {
        let err = DbError::connection("Failed to connect");
        assert_eq!(
            err.to_string(),
            "Database connection error: Failed to connect"
        );
    }

    #[test]
    fn test_from_sqlx_row_not_found() {
        let sqlx_err = sqlx::Error::RowNotFound;
        let db_err: DbError = sqlx_err.into();
        assert!(db_err.is_not_found());
    }
}
