//! Error types for the remote database system

use thiserror::Error;

/// Errors that can occur in database operations
#[derive(Error, Debug)]
pub enum DbError {
    /// The requested resource was not found
    #[error("{entity} not found{}", .id.as_ref().map(|id| format!(": {}", id)).unwrap_or_default())]
    NotFound {
        /// The type of entity that was not found
        entity: &'static str,
        /// Optional identifier for the entity
        id: Option<String>,
    },

    /// A unique constraint was violated (e.g., duplicate username or email)
    #[error("Duplicate {field}: {value}")]
    Duplicate {
        /// The field that had a duplicate value
        field: &'static str,
        /// The value that caused the conflict
        value: String,
    },

    /// Foreign key constraint violation
    #[error("Referenced {entity} does not exist")]
    ForeignKeyViolation {
        /// The entity that was referenced but doesn't exist
        entity: &'static str,
    },

    /// Database connection error
    #[error("Database connection error: {0}")]
    Connection(String),

    /// Database migration error
    #[error("Migration error: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),

    /// Database pool error
    #[error("Connection pool error: {0}")]
    Pool(String),

    /// Transaction error
    #[error("Transaction error: {0}")]
    Transaction(String),

    /// Query execution error
    #[error("Query error: {0}")]
    Query(String),

    /// Data serialization/deserialization error
    #[error("Data encoding error: {0}")]
    Encoding(String),

    /// Invalid input data
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// Authentication/authorization error
    #[error("Authentication error: {0}")]
    Authentication(String),

    /// Token expired or invalid
    #[error("Token error: {0}")]
    Token(String),

    /// Internal database error (catch-all for underlying sqlx errors)
    #[error("Database error: {0}")]
    Internal(#[source] sqlx::Error),
}

impl DbError {
    /// Create a not found error for an entity
    pub fn not_found(entity: &'static str) -> Self {
        Self::NotFound { entity, id: None }
    }

    /// Create a not found error for an entity with a specific ID
    pub fn not_found_with_id(entity: &'static str, id: impl Into<String>) -> Self {
        Self::NotFound {
            entity,
            id: Some(id.into()),
        }
    }

    /// Create a duplicate error
    pub fn duplicate(field: &'static str, value: impl Into<String>) -> Self {
        Self::Duplicate {
            field,
            value: value.into(),
        }
    }

    /// Create a foreign key violation error
    pub fn foreign_key(entity: &'static str) -> Self {
        Self::ForeignKeyViolation { entity }
    }

    /// Create a connection error
    pub fn connection(msg: impl Into<String>) -> Self {
        Self::Connection(msg.into())
    }

    /// Create a pool error
    pub fn pool(msg: impl Into<String>) -> Self {
        Self::Pool(msg.into())
    }

    /// Create a transaction error
    pub fn transaction(msg: impl Into<String>) -> Self {
        Self::Transaction(msg.into())
    }

    /// Create a query error
    pub fn query(msg: impl Into<String>) -> Self {
        Self::Query(msg.into())
    }

    /// Create an encoding error
    pub fn encoding(msg: impl Into<String>) -> Self {
        Self::Encoding(msg.into())
    }

    /// Create an invalid input error
    pub fn invalid_input(msg: impl Into<String>) -> Self {
        Self::InvalidInput(msg.into())
    }

    /// Create an authentication error
    pub fn authentication(msg: impl Into<String>) -> Self {
        Self::Authentication(msg.into())
    }

    /// Create a token error
    pub fn token(msg: impl Into<String>) -> Self {
        Self::Token(msg.into())
    }

    /// Check if this error represents a "not found" condition
    pub fn is_not_found(&self) -> bool {
        matches!(self, Self::NotFound { .. })
    }

    /// Check if this error represents a duplicate/conflict condition
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
                        _ => Self::Internal(sqlx::Error::Database(db_err)),
                    }
                } else {
                    Self::Internal(sqlx::Error::Database(db_err))
                }
            }
            sqlx::Error::PoolTimedOut => Self::Pool("Connection pool timed out".to_string()),
            sqlx::Error::PoolClosed => Self::Pool("Connection pool is closed".to_string()),
            sqlx::Error::Io(io_err) => Self::Connection(io_err.to_string()),
            sqlx::Error::Tls(tls_err) => Self::Connection(format!("TLS error: {}", tls_err)),
            other => Self::Internal(other),
        }
    }
}

/// Result type alias for database operations
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
