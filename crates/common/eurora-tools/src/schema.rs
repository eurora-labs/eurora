//! Schema accessors used by `ToolDescriptor`.
//!
//! Tools declare their input and output JSON Schemas through
//! `fn() -> &'static schemars::Schema` pointers. The macro that emits these
//! descriptor tables (see `eurora-tools-macros` in a later phase) can either
//! emit its own per-type `LazyLock<Schema>` static items or call
//! [`schema_of`] to share a single process-wide cache. Both are equivalent
//! from the caller's point of view; the cache keeps the macro output tight
//! (one line per accessor) and amortises the cost of schema generation to
//! one allocation per `JsonSchema` type for the lifetime of the process.

use std::any::TypeId;
use std::sync::LazyLock;

use dashmap::DashMap;
use schemars::{JsonSchema, Schema};

/// Function pointer type stored on [`ToolDescriptor`] for lazy schema
/// evaluation.
///
/// The pointer is invoked from `to_wire()` and from any in-process consumer
/// that needs the schema (e.g. validation in tests). Pointers obtained via
/// [`schema_of`] are cheap to call — only the first call per type pays
/// schemars' generation cost.
///
/// [`ToolDescriptor`]: crate::ToolDescriptor
pub type SchemaFn = fn() -> &'static Schema;

/// Return a process-wide cached JSON Schema for `T`.
///
/// The first call for a given `T` runs `schemars::schema_for!(T)`, boxes the
/// result, and leaks it; subsequent calls return the same `&'static`
/// reference. Concurrent first-callers serialize on the inner shard so the
/// generator runs at most once per type.
///
/// The leak is intentional — schemas are referenced from `&'static`
/// descriptor tables, the per-type allocation is small (a few hundred bytes
/// of JSON-Schema metadata), and the total set of `JsonSchema` types is
/// bounded by the adapter trait declarations linked into the binary.
pub fn schema_of<T>() -> &'static Schema
where
    T: JsonSchema + 'static,
{
    static CACHE: LazyLock<DashMap<TypeId, &'static Schema>> = LazyLock::new(DashMap::new);

    let tid = TypeId::of::<T>();
    if let Some(entry) = CACHE.get(&tid) {
        return *entry;
    }
    *CACHE.entry(tid).or_insert_with(|| {
        let schema: Schema = schemars::schema_for!(T);
        let leaked: &'static Schema = Box::leak(Box::new(schema));
        leaked
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, JsonSchema)]
    struct CacheFoo {
        x: i32,
    }

    #[derive(Serialize, Deserialize, JsonSchema)]
    struct CacheBar {
        y: String,
    }

    #[test]
    fn schema_of_returns_same_pointer_on_repeated_calls() {
        let a = schema_of::<CacheFoo>() as *const Schema;
        let b = schema_of::<CacheFoo>() as *const Schema;
        assert_eq!(a, b, "schema_of must cache by TypeId");
    }

    #[test]
    fn schema_of_returns_distinct_pointers_per_type() {
        let foo = schema_of::<CacheFoo>() as *const Schema;
        let bar = schema_of::<CacheBar>() as *const Schema;
        assert_ne!(foo, bar);
    }

    #[test]
    fn schema_of_produces_valid_json_schema() {
        let schema = schema_of::<CacheFoo>();
        let value = serde_json::to_value(schema).expect("schema serializes to JSON");
        assert!(value.is_object(), "expected a JSON object, got {value}");
        let serialized = serde_json::to_string(&value).unwrap();
        assert!(
            serialized.contains("\"x\""),
            "schema should reference field name `x`: {serialized}"
        );
    }

    #[test]
    fn schema_fn_pointer_is_usable() {
        // Sanity: `schema_of::<T>` can be assigned to a `SchemaFn` pointer
        // (the descriptor-table use case).
        let fn_ptr: SchemaFn = schema_of::<CacheFoo>;
        let schema = (fn_ptr)();
        let value = serde_json::to_value(schema).unwrap();
        assert!(value.is_object());
    }
}
