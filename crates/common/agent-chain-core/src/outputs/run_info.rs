use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub struct RunInfo {
    pub run_id: Uuid,
}

impl RunInfo {
    pub fn new(run_id: Uuid) -> Self {
        Self { run_id }
    }

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
