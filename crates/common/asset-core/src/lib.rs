//! Shared HTTP DTOs for the Eurora asset service.
//!
//! Used by `be-asset` (domain logic), `be-asset-service` (HTTP server) and the
//! desktop / web HTTP clients. The same types feed Specta-generated TypeScript
//! bindings via the workspace-level `euro-api-codegen` orchestrator.

pub mod asset;

pub use asset::{Asset, CreateAssetRequest};

/// Build a [`specta::Types`] containing every asset wire type the frontend
/// needs. Consumed by `euro-api-codegen` to emit `asset.ts`.
#[cfg(feature = "specta")]
pub fn type_collection() -> specta::Types {
    specta::Types::default()
        .register::<Asset>()
        .register::<CreateAssetRequest>()
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "specta")]
    #[test]
    fn type_collection_contains_all_wire_types() {
        let types = super::type_collection();
        let names: Vec<String> = types
            .into_unsorted_iter()
            .map(|ndt| ndt.name.to_string())
            .collect();
        for expected in ["Asset", "CreateAssetRequest"] {
            assert!(
                names.iter().any(|n| n == expected),
                "missing {expected} from collection: {names:?}"
            );
        }
    }
}
