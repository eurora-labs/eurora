use serde::{Deserialize, Serialize};
use settings_core::CloudSettings;
use uuid::Uuid;

/// On-disk shape of `~/.config/eurora/cloud.json`.
///
/// Holds the last-pulled [`CloudSettings`] blob plus enough metadata for
/// the sync engine (added in Phase 5+) to decide whether the cache is
/// safe to trust. Phase 4 only writes / reads this file — the network
/// trigger comes later.
///
/// `last_user_id` is `None` until a successful pull stamps it with the
/// authenticated user's JWT subject. The sync engine compares this
/// against the current auth state on every pull and discards the cache
/// on mismatch so a shared machine never leaks one user's settings to
/// another's session.
///
/// `settings` is nested (not flattened) so future cache-side metadata
/// — e.g. `last_pull_at`, retry counters — can land here without
/// disturbing the wire shape.
#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct CloudSettingsCache {
    pub last_user_id: Option<Uuid>,
    pub settings: CloudSettings,
}

#[cfg(test)]
mod tests {
    use super::*;
    use settings_core::CURRENT_SCHEMA_VERSION;

    #[test]
    fn defaults_round_trip() {
        let c = CloudSettingsCache::default();
        let v = serde_json::to_value(&c).unwrap();
        let back: CloudSettingsCache = serde_json::from_value(v).unwrap();
        assert_eq!(back, c);
    }

    #[test]
    fn defaults_match_settings_core_fresh_install() {
        let c = CloudSettingsCache::default();
        assert!(c.last_user_id.is_none());
        assert_eq!(c.settings.schema_version, CURRENT_SCHEMA_VERSION);
    }

    #[test]
    fn empty_object_deserialises_to_defaults() {
        let c: CloudSettingsCache = serde_json::from_str("{}").unwrap();
        assert_eq!(c, CloudSettingsCache::default());
    }
}
