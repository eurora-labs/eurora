//! Unit tests for RunInfo class.
//!
//! Ported from `langchain/libs/core/tests/unit_tests/outputs/test_run_info.py`
//!
//! Note: Several Python tests involve Pydantic-specific functionality that doesn't
//! directly translate to Rust (e.g., model_dump, model_validate_json, BaseModel inheritance).
//! In Rust, we use Serde for serialization which has different semantics.

use agent_chain_core::outputs::RunInfo;
use uuid::Uuid;

/// Test suite for RunInfo class.
mod run_info_tests {
    use super::*;

    /// Test creating RunInfo with a UUID.
    #[test]
    fn test_creation_with_uuid() {
        let run_id = Uuid::new_v4();
        let run_info = RunInfo::new(run_id);
        assert_eq!(run_info.run_id, run_id);
    }

    /// Test creating RunInfo with a specific UUID string.
    #[test]
    fn test_creation_with_specific_uuid() {
        let uuid_str = "12345678-1234-5678-1234-567812345678";
        let run_id = Uuid::parse_str(uuid_str).unwrap();
        let run_info = RunInfo::new(run_id);
        assert_eq!(run_info.run_id, run_id);
        assert_eq!(run_info.run_id.to_string(), uuid_str);
    }

    /// Test that run_id is of UUID type.
    #[test]
    fn test_run_id_is_uuid_type() {
        let run_id = Uuid::new_v4();
        let run_info = RunInfo::new(run_id);
        // In Rust, type checking is compile-time, but we verify the value is correct
        let _: Uuid = run_info.run_id;
    }

    /// Test that different RunInfo instances can have different IDs.
    #[test]
    fn test_different_run_infos_have_different_ids() {
        let run_id1 = Uuid::new_v4();
        let run_id2 = Uuid::new_v4();
        let run_info1 = RunInfo::new(run_id1);
        let run_info2 = RunInfo::new(run_id2);
        assert_ne!(run_info1.run_id, run_info2.run_id);
    }

    /// Test equality for RunInfo with same run_id.
    #[test]
    fn test_equality_same_run_id() {
        let run_id = Uuid::new_v4();
        let run_info1 = RunInfo::new(run_id);
        let run_info2 = RunInfo::new(run_id);
        assert_eq!(run_info1, run_info2);
    }

    /// Test inequality for RunInfo with different run_id.
    #[test]
    fn test_inequality_different_run_id() {
        let run_id1 = Uuid::new_v4();
        let run_id2 = Uuid::new_v4();
        let run_info1 = RunInfo::new(run_id1);
        let run_info2 = RunInfo::new(run_id2);
        assert_ne!(run_info1, run_info2);
    }

    // Note: test_run_info_is_pydantic_model - Rust doesn't use Pydantic.
    // RunInfo is a plain struct with Serde derives.

    /// Test serialization of RunInfo to dictionary (via serde_json).
    #[test]
    fn test_serialization_to_dict() {
        let run_id = Uuid::new_v4();
        let run_info = RunInfo::new(run_id);
        let value = serde_json::to_value(&run_info).unwrap();
        assert!(value.get("run_id").is_some());
        assert_eq!(
            value.get("run_id").unwrap().as_str().unwrap(),
            run_id.to_string()
        );
    }

    /// Test deserialization of RunInfo from dictionary (via serde_json).
    #[test]
    fn test_deserialization_from_dict() {
        let run_id = Uuid::new_v4();
        let data = serde_json::json!({ "run_id": run_id.to_string() });
        let run_info: RunInfo = serde_json::from_value(data).unwrap();
        assert_eq!(run_info.run_id, run_id);
    }

    /// Test JSON serialization of RunInfo.
    #[test]
    fn test_json_serialization() {
        let run_id = Uuid::new_v4();
        let run_info = RunInfo::new(run_id);
        let json_str = serde_json::to_string(&run_info).unwrap();
        assert!(json_str.contains(&run_id.to_string()));
    }

    /// Test JSON deserialization of RunInfo.
    #[test]
    fn test_json_deserialization() {
        let run_id = Uuid::new_v4();
        let run_info = RunInfo::new(run_id);
        let json_str = serde_json::to_string(&run_info).unwrap();
        let deserialized: RunInfo = serde_json::from_str(&json_str).unwrap();
        assert_eq!(deserialized.run_id, run_id);
    }

    // Note: test_run_id_immutability - In Rust, struct fields are mutable by default
    // if you have ownership. This is different from Python's Pydantic models.
    // We can demonstrate mutability:
    #[test]
    fn test_run_id_mutability() {
        let run_id = Uuid::new_v4();
        let mut run_info = RunInfo::new(run_id);
        let original_id = run_info.run_id;
        let new_id = Uuid::new_v4();
        run_info.run_id = new_id;
        assert_eq!(run_info.run_id, new_id);
        assert_ne!(run_info.run_id, original_id);
    }

    /// Test that Debug repr contains the run_id.
    #[test]
    fn test_repr_contains_run_id() {
        let run_id = Uuid::new_v4();
        let run_info = RunInfo::new(run_id);
        let repr_str = format!("{:?}", run_info);
        assert!(repr_str.contains("run_id"));
        assert!(repr_str.contains(&run_id.to_string()));
    }

    /// Test string representation of RunInfo.
    #[test]
    fn test_str_representation() {
        let run_id = Uuid::new_v4();
        let run_info = RunInfo::new(run_id);
        // In Rust, Debug is used for representation
        let str_repr = format!("{:?}", run_info);
        assert!(str_repr.contains("run_id"));
    }

    // Note: test_hash_consistency - RunInfo doesn't derive Hash by default.
    // This is a deliberate difference from Python.

    /// Test that UUID version is preserved.
    #[test]
    fn test_uuid_version() {
        let run_id = Uuid::new_v4(); // Creates UUID version 4
        let run_info = RunInfo::new(run_id);
        assert_eq!(run_info.run_id.get_version_num(), 4);
    }

    /// Test creating a list of RunInfo objects.
    #[test]
    fn test_multiple_run_infos_in_list() {
        let run_ids: Vec<Uuid> = (0..5).map(|_| Uuid::new_v4()).collect();
        let run_infos: Vec<RunInfo> = run_ids.iter().map(|rid| RunInfo::new(*rid)).collect();
        assert_eq!(run_infos.len(), 5);
        for (i, run_info) in run_infos.iter().enumerate() {
            assert_eq!(run_info.run_id, run_ids[i]);
        }
    }

    /// Test creating RunInfo with UUID parsed from string.
    #[test]
    fn test_run_info_with_uuid_from_string() {
        let uuid_str = "a1b2c3d4-e5f6-7890-abcd-ef1234567890";
        let run_id = Uuid::parse_str(uuid_str).unwrap();
        let run_info = RunInfo::new(run_id);
        assert_eq!(run_info.run_id.to_string(), uuid_str);
    }

    /// Test creating RunInfo with new_random.
    #[test]
    fn test_new_random() {
        let run_info = RunInfo::new_random();
        // UUID v4 has version field set to 4
        assert_eq!(run_info.run_id.get_version_num(), 4);
    }

    /// Test Default implementation for RunInfo.
    #[test]
    fn test_default() {
        let run_info = RunInfo::default();
        // Default uses new_random, so should be v4 UUID
        assert_eq!(run_info.run_id.get_version_num(), 4);
    }
}
