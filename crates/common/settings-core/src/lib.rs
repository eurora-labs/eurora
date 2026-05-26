//! Shared wire types for cloud-synced universal settings.
//!
//! This crate is the single source of truth for the JSON contract
//! between `be-settings-service` (Axum) and the desktop / mobile / web
//! HTTP clients, and is also the input to the TypeScript bindings
//! emitted by the workspace-level `euro-codegen` orchestrator
//! (`pnpm specta`).
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
//! ## Defaults
//!
//! Each section's `Default` impl is both the fresh-install value and
//! the per-field fallback used by `#[serde(default)]` when an older
//! client wrote a partial blob. Booleans collapse to `false` and
//! counters to `0` so a missing field can never silently opt a user
//! in; [`SharedSettings`] is the one section whose `Default` is
//! hand-rolled to set `dynamic_accent: true` (the product default),
//! and [`DesktopSettings`] scales fall back to [`DEFAULT_SCALE`]
//! because a zero-size UI is unrecoverable.
//!
//! ## Telemetry consent
//!
//! [`TelemetryConsent`] lives under each *platform* section, never
//! under [`SharedSettings`]: consent must be specific to the data
//! actually collected, and each platform ships a different telemetry
//! stack. The "current consent version" is per-platform too — see
//! [`DESKTOP_CONSENT_VERSION`]. The struct enforces monotonic recording
//! on the type itself ([`TelemetryConsent::record_for_desktop`]) so an
//! older client cannot roll back a newer client's stored consent.

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
pub use telemetry::{DESKTOP_CONSENT_VERSION, TelemetryConsent};
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
