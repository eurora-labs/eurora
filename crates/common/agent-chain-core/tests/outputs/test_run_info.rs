use agent_chain_core::outputs::RunInfo;
use uuid::Uuid;

mod run_info_tests {
    use super::*;

    #[test]
    fn test_creation_with_uuid() {
        let run_id = Uuid::new_v4();
        let run_info = RunInfo::new(run_id);
        assert_eq!(run_info.run_id, run_id);
    }

    #[test]
    fn test_creation_with_specific_uuid() {
        let uuid_str = "12345678-1234-5678-1234-567812345678";
        let run_id = Uuid::parse_str(uuid_str).unwrap();
        let run_info = RunInfo::new(run_id);
        assert_eq!(run_info.run_id, run_id);
        assert_eq!(run_info.run_id.to_string(), uuid_str);
    }

    #[test]
    fn test_run_id_is_uuid_type() {
        let run_id = Uuid::new_v4();
        let run_info = RunInfo::new(run_id);
        let _: Uuid = run_info.run_id;
    }

    #[test]
    fn test_different_run_infos_have_different_ids() {
        let run_id1 = Uuid::new_v4();
        let run_id2 = Uuid::new_v4();
        let run_info1 = RunInfo::new(run_id1);
        let run_info2 = RunInfo::new(run_id2);
        assert_ne!(run_info1.run_id, run_info2.run_id);
    }

    #[test]
    fn test_equality_same_run_id() {
        let run_id = Uuid::new_v4();
        let run_info1 = RunInfo::new(run_id);
        let run_info2 = RunInfo::new(run_id);
        assert_eq!(run_info1, run_info2);
    }

    #[test]
    fn test_inequality_different_run_id() {
        let run_id1 = Uuid::new_v4();
        let run_id2 = Uuid::new_v4();
        let run_info1 = RunInfo::new(run_id1);
        let run_info2 = RunInfo::new(run_id2);
        assert_ne!(run_info1, run_info2);
    }

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

    #[test]
    fn test_deserialization_from_dict() {
        let run_id = Uuid::new_v4();
        let data = serde_json::json!({ "run_id": run_id.to_string() });
        let run_info: RunInfo = serde_json::from_value(data).unwrap();
        assert_eq!(run_info.run_id, run_id);
    }

    #[test]
    fn test_json_serialization() {
        let run_id = Uuid::new_v4();
        let run_info = RunInfo::new(run_id);
        let json_str = serde_json::to_string(&run_info).unwrap();
        assert!(json_str.contains(&run_id.to_string()));
    }

    #[test]
    fn test_json_deserialization() {
        let run_id = Uuid::new_v4();
        let run_info = RunInfo::new(run_id);
        let json_str = serde_json::to_string(&run_info).unwrap();
        let deserialized: RunInfo = serde_json::from_str(&json_str).unwrap();
        assert_eq!(deserialized.run_id, run_id);
    }

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

    #[test]
    fn test_repr_contains_run_id() {
        let run_id = Uuid::new_v4();
        let run_info = RunInfo::new(run_id);
        let repr_str = format!("{:?}", run_info);
        assert!(repr_str.contains("run_id"));
        assert!(repr_str.contains(&run_id.to_string()));
    }

    #[test]
    fn test_str_representation() {
        let run_id = Uuid::new_v4();
        let run_info = RunInfo::new(run_id);
        let str_repr = format!("{:?}", run_info);
        assert!(str_repr.contains("run_id"));
    }

    #[test]
    fn test_hash_consistency() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let run_id = Uuid::new_v4();
        let run_info1 = RunInfo::new(run_id);
        let run_info2 = RunInfo::new(run_id);

        let mut hasher1 = DefaultHasher::new();
        run_info1.hash(&mut hasher1);
        let hash1 = hasher1.finish();

        let mut hasher2 = DefaultHasher::new();
        run_info2.hash(&mut hasher2);
        let hash2 = hasher2.finish();

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_uuid_version() {
        let run_id = Uuid::new_v4(); // Creates UUID version 4
        let run_info = RunInfo::new(run_id);
        assert_eq!(run_info.run_id.get_version_num(), 4);
    }

    #[test]
    fn test_multiple_run_infos_in_list() {
        let run_ids: Vec<Uuid> = (0..5).map(|_| Uuid::new_v4()).collect();
        let run_infos: Vec<RunInfo> = run_ids.iter().map(|rid| RunInfo::new(*rid)).collect();
        assert_eq!(run_infos.len(), 5);
        for (i, run_info) in run_infos.iter().enumerate() {
            assert_eq!(run_info.run_id, run_ids[i]);
        }
    }

    #[test]
    fn test_run_info_with_uuid_from_string() {
        let uuid_str = "a1b2c3d4-e5f6-7890-abcd-ef1234567890";
        let run_id = Uuid::parse_str(uuid_str).unwrap();
        let run_info = RunInfo::new(run_id);
        assert_eq!(run_info.run_id.to_string(), uuid_str);
    }

    #[test]
    fn test_new_random() {
        let run_info = RunInfo::new_random();
        assert_eq!(run_info.run_id.get_version_num(), 4);
    }

    #[test]
    fn test_default() {
        let run_info = RunInfo::default();
        assert_eq!(run_info.run_id.get_version_num(), 4);
    }
}

mod run_info_serde_coercion_tests {
    use super::*;

    #[test]
    fn test_creation_from_string_uuid() {
        let uuid_str = "12345678-1234-5678-1234-567812345678";
        let data = serde_json::json!({ "run_id": uuid_str });
        let run_info: RunInfo =
            serde_json::from_value(data).expect("should deserialize from string UUID");
        assert_eq!(run_info.run_id.to_string(), uuid_str);
    }

    #[test]
    fn test_deserialize_from_value() {
        let run_id = Uuid::new_v4();
        let data = serde_json::json!({ "run_id": run_id.to_string() });
        let run_info: RunInfo =
            serde_json::from_value(data).expect("should deserialize from value");
        assert_eq!(run_info.run_id, run_id);
    }

    #[test]
    fn test_deserialize_with_string_uuid() {
        let uuid_str = "12345678-1234-5678-1234-567812345678";
        let data = serde_json::json!({ "run_id": uuid_str });
        let run_info: RunInfo =
            serde_json::from_value(data).expect("should deserialize from string UUID");
        assert_eq!(run_info.run_id.to_string(), uuid_str);
    }

    #[test]
    fn test_serialized_fields() {
        let run_id = Uuid::new_v4();
        let run_info = RunInfo::new(run_id);
        let value = serde_json::to_value(&run_info).expect("should serialize");
        let fields: Vec<&String> = value
            .as_object()
            .expect("should be object")
            .keys()
            .collect();
        assert!(fields.contains(&&"run_id".to_string()));
    }

    #[test]
    fn test_clone() {
        let run_id = Uuid::new_v4();
        let original = RunInfo::new(run_id);
        let cloned = original.clone();
        assert_eq!(cloned.run_id, original.run_id);
        assert_eq!(cloned, original);
    }

    #[test]
    fn test_clone_independence() {
        let run_id = Uuid::new_v4();
        let original = RunInfo::new(run_id);
        let mut cloned = original.clone();
        assert_eq!(cloned, original);
        cloned.run_id = Uuid::new_v4();
        assert_ne!(cloned, original);
    }
}
