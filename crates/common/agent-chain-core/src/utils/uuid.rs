//! UUID utility functions.
//!
//! This module provides UUID generation utilities for tracing and similar operations.
//!
//! Adapted from langchain_core/utils/uuid.py

use uuid::Timestamp;
use uuid::Uuid;

/// LangChain auto-generated ID prefix for messages and content blocks.
pub const LC_AUTO_PREFIX: &str = "lc_";

/// Internal tracing/callback system identifier.
///
/// Used for:
/// - Tracing: Every LangChain operation (LLM call, chain execution, tool use, etc.)
///   gets a unique run_id (UUID)
/// - Enables tracking parent-child relationships between operations
pub const LC_ID_PREFIX: &str = "lc_run-";

/// Generate a time-ordered UUID v7.
///
/// UUIDv7 objects feature monotonicity within a millisecond,
/// making them suitable for use as database keys or for tracing
/// where time ordering is important.
///
/// # Arguments
///
/// * `timestamp_millis` - Optional Unix timestamp in milliseconds.
///   If not provided, uses the current time.
///
/// # Returns
///
/// A new time-ordered UUID.
///
/// # Example
///
/// ```
/// use agent_chain_core::utils::uuid::uuid7;
///
/// let id = uuid7(None);
/// println!("Generated UUID v7: {}", id);
/// ```
pub fn uuid7(timestamp_millis: Option<u64>) -> Uuid {
    match timestamp_millis {
        Some(millis) => {
            let secs = millis / 1000;
            let nanos = ((millis % 1000) * 1_000_000) as u32;
            let ts = Timestamp::from_unix(uuid::NoContext, secs, nanos);
            Uuid::new_v7(ts)
        }
        None => Uuid::now_v7(),
    }
}

/// Ensure the ID is a valid string, generating a new UUID if not provided.
///
/// Auto-generated UUIDs are prefixed by `'lc_'` to indicate they are
/// LangChain-generated IDs.
///
/// # Arguments
///
/// * `id_val` - Optional string ID value to validate.
///
/// # Returns
///
/// A string ID, either the validated provided value or a newly generated UUID4.
///
/// # Example
///
/// ```
/// use agent_chain_core::utils::uuid::ensure_id;
///
/// let id = ensure_id(Some("my-custom-id".to_string()));
/// assert_eq!(id, "my-custom-id");
///
/// let generated = ensure_id(None);
/// assert!(generated.starts_with("lc_"));
/// ```
pub fn ensure_id(id_val: Option<String>) -> String {
    id_val.unwrap_or_else(|| format!("{}{}", LC_AUTO_PREFIX, uuid7(None)))
}

/// Generate a run ID with the LC_ID_PREFIX.
///
/// # Returns
///
/// A string ID prefixed with `lc_run-`.
pub fn generate_run_id() -> String {
    format!("{}{}", LC_ID_PREFIX, uuid7(None))
}

/// Parse a UUID from a string.
///
/// # Arguments
///
/// * `s` - The string to parse.
///
/// # Returns
///
/// The parsed UUID, or an error if parsing fails.
pub fn parse_uuid(s: &str) -> Result<Uuid, uuid::Error> {
    Uuid::parse_str(s)
}

/// Check if a string is a valid UUID.
///
/// # Arguments
///
/// * `s` - The string to check.
///
/// # Returns
///
/// `true` if the string is a valid UUID, `false` otherwise.
pub fn is_valid_uuid(s: &str) -> bool {
    Uuid::parse_str(s).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uuid7() {
        let id = uuid7(None);
        assert!(!id.is_nil());
    }

    #[test]
    fn test_ensure_id_with_value() {
        let id = ensure_id(Some("my-custom-id".to_string()));
        assert_eq!(id, "my-custom-id");
    }

    #[test]
    fn test_ensure_id_without_value() {
        let id = ensure_id(None);
        assert!(id.starts_with(LC_AUTO_PREFIX));
    }

    #[test]
    fn test_generate_run_id() {
        let id = generate_run_id();
        assert!(id.starts_with(LC_ID_PREFIX));
    }

    #[test]
    fn test_parse_uuid() {
        let uuid_str = "550e8400-e29b-41d4-a716-446655440000";
        let result = parse_uuid(uuid_str);
        assert!(result.is_ok());
    }

    #[test]
    fn test_is_valid_uuid() {
        assert!(is_valid_uuid("550e8400-e29b-41d4-a716-446655440000"));
        assert!(!is_valid_uuid("not-a-uuid"));
    }
}
