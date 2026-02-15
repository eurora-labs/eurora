//! **Load** module helps with serialization and deserialization.
//!
//! This module provides functionality for serializing and deserializing LangChain
//! objects to/from JSON, mirroring `langchain_core.load`.
//!
//! # Overview
//!
//! The load module contains:
//! - [`Serializable`] trait for objects that can be serialized
//! - [`Serialized`] types for different serialization representations
//! - [`dumps`] and [`dumpd`] functions for serialization
//! - [`loads`] and [`load`] functions for deserialization
//! - [`Reviver`] for customizing deserialization behavior
//!
//! # Example
//!
//! ```ignore
//! use agent_chain_core::load::{dumps, loads, Serializable};
//!
//! #[derive(Serialize, Deserialize)]
//! struct MyModel {
//!     name: String,
//! }
//!
//! impl Serializable for MyModel {
//!     fn is_lc_serializable() -> bool { true }
//!     fn get_lc_namespace() -> Vec<String> {
//!         vec!["my_package".to_string()]
//!     }
//! }
//!
//! let model = MyModel { name: "test".to_string() };
//! let json = dumps(&model, false)?;
//! let loaded = loads(&json, None)?;
//! ```

mod dump;
mod loader;
mod mapping;
mod serializable;

// Re-export serializable types
pub use serializable::{
    BaseSerialized, LC_VERSION, Serializable, Serialized, SerializedConstructor,
    SerializedConstructorData, SerializedNotImplemented, SerializedNotImplementedData,
    SerializedSecret, SerializedSecretData, to_json_not_implemented, to_json_not_implemented_value,
};

// Re-export dump functions
pub use dump::{dumpd, dumps};

// Re-export load functions and types
pub use loader::{
    ConstructorInfo, RevivedValue, Reviver, ReviverConfig, load, loads, loads_with_namespaces,
    loads_with_secrets,
};

// Re-export mapping types and constants
pub use mapping::{
    DEFAULT_NAMESPACES, DISALLOW_LOAD_FROM_PATH, JS_SERIALIZABLE_MAPPING, NamespaceMapping,
    OG_SERIALIZABLE_MAPPING, OLD_CORE_NAMESPACES_MAPPING, SERIALIZABLE_MAPPING,
    get_all_serializable_mappings,
};

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize)]
    struct TestSerializable {
        content: String,
    }

    impl Serializable for TestSerializable {
        fn is_lc_serializable() -> bool {
            true
        }

        fn get_lc_namespace() -> Vec<String> {
            vec!["langchain_core".to_string(), "test".to_string()]
        }
    }

    #[test]
    fn test_roundtrip_serialization() {
        let obj = TestSerializable {
            content: "Hello, World!".to_string(),
        };

        let json = dumps(&obj, false).unwrap();
        assert!(json.contains("constructor"));
        assert!(json.contains("Hello, World!"));

        // Load with default config - langchain_core is a valid namespace
        let loaded = loads(&json, None).unwrap();
        assert!(loaded.is_object());
    }

    #[test]
    fn test_roundtrip_with_custom_namespace() {
        #[derive(Debug, Serialize, Deserialize)]
        struct CustomSerializable {
            value: i32,
        }

        impl Serializable for CustomSerializable {
            fn is_lc_serializable() -> bool {
                true
            }

            fn get_lc_namespace() -> Vec<String> {
                vec!["custom_namespace".to_string(), "models".to_string()]
            }
        }

        let obj = CustomSerializable { value: 42 };
        let json = dumps(&obj, false).unwrap();

        // Load with custom namespace allowed
        let config =
            ReviverConfig::new().with_valid_namespaces(vec!["custom_namespace".to_string()]);
        let loaded = loads(&json, Some(config)).unwrap();
        assert!(loaded.is_object());
    }

    #[test]
    fn test_serializable_trait() {
        assert!(TestSerializable::is_lc_serializable());
        assert_eq!(
            TestSerializable::get_lc_namespace(),
            vec!["langchain_core".to_string(), "test".to_string()]
        );
    }

    #[test]
    fn test_mapping_exists() {
        assert!(!SERIALIZABLE_MAPPING.is_empty());
    }
}
