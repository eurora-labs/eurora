//! RunInfo class.
//!
//! This module contains the `RunInfo` type which holds metadata for a single
//! execution of a Chain or model.
//! Mirrors `langchain_core.outputs.run_info`.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Class that contains metadata for a single execution of a Chain or model.
///
/// Defined for backwards compatibility with older versions of langchain_core.
///
/// This model will likely be deprecated in the future.
///
/// Users can acquire the run_id information from callbacks or via run_id
/// information present in the astream_event API (depending on the use case).

#[derive(Debug, Clone, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub struct RunInfo {
    /// A unique identifier for the model or chain run.
    pub run_id: Uuid,
}

impl RunInfo {
    /// Create a new RunInfo with the given run_id.
    pub fn new(run_id: Uuid) -> Self {
        Self { run_id }
    }

    /// Create a new RunInfo with a randomly generated run_id.
    pub fn new_random() -> Self {
        Self {
            run_id: Uuid::new_v4(),
        }
    }
}

impl Default for RunInfo {
    fn default() -> Self {
        Self::new_random()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run_info_new() {
        let id = Uuid::new_v4();
        let info = RunInfo::new(id);
        assert_eq!(info.run_id, id);
    }

    #[test]
    fn test_run_info_serialization() {
        let info = RunInfo::new_random();
        let json = serde_json::to_string(&info).unwrap();
        let deserialized: RunInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.run_id, info.run_id);
    }
}
