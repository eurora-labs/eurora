//! Shared wire types for cloud-synced universal settings.
//!
//! This crate is the single source of truth for the JSON contract
//! between `be-settings-service` (Axum) and the desktop / mobile / web
//! HTTP clients, and is also the input to the TypeScript bindings
//! emitted by the workspace-level `euro-api-codegen` orchestrator
//! (`pnpm specta:backend`).
//!
//! Types are pure data with `serde` derives; the optional `specta`
//! feature adds `specta::Type` so the same definitions can be
//! re-exported as TS. No HTTP, database, or transport dependencies
//! live here on purpose — pulling this crate into a leaf binary must
//! not drag in transport plumbing.
//!
//! ## Server is blob-opaque
//!
//! The server stores the settings document as an opaque JSON blob,
//! addressed by `(user_id, schema_version, updated_at)`. It never
//! parses the body, which means clients are responsible for keeping
//! the document well-formed. Field-level invariants (e.g. UI scale
//! bounds) are carried by the field *types* themselves — see
//! [`DesktopSettings::interface_scale`] / [`DesktopSettings::text_scale`],
//! which are newtypes that clamp on every entry path including
//! `Deserialize`. Bump [`CURRENT_SCHEMA_VERSION`] when the structural
//! shape of the blob changes incompatibly.
//!
//! ## Forward-compatibility
//!
//! Every leaf section carries a flattened `extras: serde_json::Map`
//! so unknown fields written by a newer client (or a different
//! platform) round-trip verbatim. Releases can therefore add fields
//! to any section without bumping the schema version, as long as the
//! field has a sensible default. [`CURRENT_SCHEMA_VERSION`] only
//! advances when the *structural* shape changes incompatibly.
//!
//! ## Two tiers of defaults
//!
//! Defaults serve two distinct roles, which this crate keeps separate:
//!
//! 1. **Fresh-install defaults** — what a brand-new user sees on first
//!    launch. Owned by `assets/defaults.jsonc`. Reached through
//!    [`CloudSettings::default()`]. Edit the JSONC to change them.
//!
//! 2. **Wire fallback defaults** — what `#[serde(default)]` fills in
//!    when an older client wrote a partial blob and a newer field is
//!    missing. Owned by each leaf section's `Default` impl. These are
//!    deliberately *inert* (booleans → `false`, counters → `0`) so a
//!    missing field can never silently opt a user in. The single
//!    exception is [`DesktopSettings`], whose scales fall back to
//!    [`DEFAULT_SCALE`] (via [`InterfaceScale::DEFAULT`] /
//!    [`TextScale::DEFAULT`]) because a zero-size UI is unrecoverable.
//!
//! The two tiers may diverge intentionally — e.g. `dynamicAccent` is
//! `true` on a fresh install (JSONC) but `false` as a wire fallback
//! (derived `Default`). Callers wanting "the product default" must use
//! [`CloudSettings::default()`], not leaf `Default` impls.

pub mod cloud;
pub mod desktop;
pub mod dto;
pub mod mobile;
pub mod shared;
pub mod telemetry;
pub mod web;

pub use cloud::{CURRENT_SCHEMA_VERSION, CloudSettings};
pub use desktop::{DEFAULT_SCALE, DesktopSettings, InterfaceScale, TextScale};
pub use dto::{
    GetSettingsResponse, PutSettingsAcceptedResponse, PutSettingsConflictResponse,
    PutSettingsRequest,
};
pub use mobile::MobileSettings;
pub use shared::{SharedSettings, ThemePreference};
pub use telemetry::TelemetryConsent;
pub use web::WebSettings;

/// Build a [`specta::Types`] containing every settings wire type the
/// desktop / mobile / web clients need. Used by the codegen binary to
/// emit `settings.ts`.
#[cfg(feature = "specta")]
pub fn type_collection() -> specta::Types {
    specta::Types::default()
        .register::<CloudSettings>()
        .register::<SharedSettings>()
        .register::<DesktopSettings>()
        .register::<MobileSettings>()
        .register::<WebSettings>()
        .register::<TelemetryConsent>()
        .register::<ThemePreference>()
        .register::<GetSettingsResponse>()
        .register::<PutSettingsRequest>()
        .register::<PutSettingsAcceptedResponse>()
        .register::<PutSettingsConflictResponse>()
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
        for expected in [
            "CloudSettings",
            "SharedSettings",
            "DesktopSettings",
            "MobileSettings",
            "WebSettings",
            "TelemetryConsent",
            "ThemePreference",
            "GetSettingsResponse",
            "PutSettingsRequest",
            "PutSettingsAcceptedResponse",
            "PutSettingsConflictResponse",
        ] {
            assert!(
                names.iter().any(|n| n == expected),
                "missing {expected} from collection: {names:?}"
            );
        }
    }
}
