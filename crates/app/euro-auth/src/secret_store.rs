//! Encrypted file-backed storage for Eurora session state.
//!
//! The store holds at most three slots — access token, refresh token,
//! and the in-flight PKCE login verifier — encrypted under a 32-byte
//! main key that lives in the OS keychain. The indirection exists for
//! macOS UX: keychain access prompts the user once per *item*, so
//! storing each token directly would mean a prompt per token. With
//! the file-store layout the user is prompted once (for the main key)
//! and every other secret round-trips through the encrypted file with
//! no keychain involvement.
//!
//! [`AuthManager`] is the sole consumer of this module — the store is
//! `pub(crate)`. External crates observe session state through
//! `AuthManager` and its [`crate::AuthEvent`] bus, never by reaching
//! into storage directly. Keeping the manager as the sole writer
//! ensures the in-process view, the keyring contents, and any
//! `AuthEvent` subscribers stay in lock-step.
//!
//! [`AuthManager`]: crate::AuthManager

use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use secrecy::{ExposeSecret as _, SecretString};
use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, ZeroizeOnDrop};

use self::file::Cipher;

pub(crate) use self::error::SecretStoreError;

mod error;
mod file;
mod main_key;

/// Service name used for every Eurora keychain entry.
///
/// Centralised so the main-key load path and the legacy-migration
/// path can't drift onto different services and silently fail to find
/// each other's entries.
const SERVICE: &str = "Eurora";

/// Legacy keychain handles that may still hold session state from
/// pre-file-store builds. [`SecretStore::open`] migrates them into the
/// file once and then deletes them.
const LEGACY_ACCESS_TOKEN_HANDLE: &str = "eurora-AUTH_ACCESS_TOKEN";
const LEGACY_REFRESH_TOKEN_HANDLE: &str = "eurora-AUTH_REFRESH_TOKEN";
const LEGACY_PKCE_VERIFIER_HANDLE: &str = "eurora-LOGIN_CODE_VERIFIER";

/// Encrypted file holding the session state.
///
/// Constructed once at startup via [`SecretStore::open`]. All
/// accessors round-trip through an in-memory cache and a write-through
/// flush; concurrent calls are serialised by a single mutex. The
/// cache exists so reads stay cheap — the encrypted file is decoded
/// once on `open` and held in memory until the process exits or
/// [`SecretStore::wipe`] is called.
pub(crate) struct SecretStore {
    inner: Mutex<Inner>,
}

struct Inner {
    state: PersistedState,
    cipher: Cipher,
    path: PathBuf,
}

/// On-disk schema, also used as the in-memory cache.
///
/// `Option<String>` fields rather than a `HashMap` because the set of
/// slots is closed and small — adding a fourth would mean editing this
/// struct, which is exactly the kind of pressure we want before
/// quietly storing one more secret on every user's disk. `Zeroize`
/// derives wipe the heap allocations of every populated slot on drop.
#[derive(Default, Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
struct PersistedState {
    #[serde(default)]
    access_token: Option<String>,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default)]
    pkce_verifier: Option<String>,
}

impl SecretStore {
    /// Open (or create) the encrypted store under `data_dir`.
    ///
    /// On first run this generates a fresh main key, persists it in
    /// the OS keychain, and writes an empty `secrets.enc`. On
    /// subsequent runs the existing main key is read back and the
    /// file decrypted. If the file is missing or undecryptable
    /// (typically because the keychain entry was wiped) the store
    /// starts fresh and the user re-authenticates.
    ///
    /// Performs a one-shot migration of any leftover per-secret
    /// keychain entries from the pre-file-store era into the file,
    /// then deletes them. The migration is best-effort: a single
    /// failing handle doesn't fail the whole `open`, only logs.
    pub(crate) fn open(data_dir: &Path) -> Result<Self, SecretStoreError> {
        Self::open_with(data_dir, os_keyring_get, os_keyring_delete)
    }

    /// Implementation shared by [`SecretStore::open`] and tests.
    ///
    /// The keyring is abstracted as a pair of closures so the
    /// migration logic can be exercised without spinning up a real
    /// OS keychain — the keyring crate's `mock` backend is per-entry
    /// and can't model the cross-handle persistence we need.
    fn open_with(
        data_dir: &Path,
        keyring_get: impl Fn(&str) -> Option<String>,
        keyring_delete: impl Fn(&str),
    ) -> Result<Self, SecretStoreError> {
        let main_key = main_key::load_or_create()?;
        let cipher = Cipher::new(&main_key);
        // `cipher` now holds the only copy we need; let `main_key`
        // drop here so its bytes are zeroed promptly.
        drop(main_key);

        let path = data_dir.join(file::STORE_FILENAME);
        let mut state = file::load(&cipher, &path)?;

        let migrated = migrate_legacy_keychain_entries(&mut state, keyring_get, keyring_delete);
        if migrated {
            file::save(&cipher, &state, &path)?;
        }

        Ok(Self {
            inner: Mutex::new(Inner {
                state,
                cipher,
                path,
            }),
        })
    }

    pub(crate) fn access_token(&self) -> Result<Option<SecretString>, SecretStoreError> {
        self.read(|s| s.access_token.clone())
    }

    pub(crate) fn set_access_token(&self, token: SecretString) -> Result<(), SecretStoreError> {
        self.mutate(|s| s.access_token = Some(token.expose_secret().to_owned()))
    }

    pub(crate) fn refresh_token(&self) -> Result<Option<SecretString>, SecretStoreError> {
        self.read(|s| s.refresh_token.clone())
    }

    pub(crate) fn set_refresh_token(&self, token: SecretString) -> Result<(), SecretStoreError> {
        self.mutate(|s| s.refresh_token = Some(token.expose_secret().to_owned()))
    }

    pub(crate) fn pkce_verifier(&self) -> Result<Option<SecretString>, SecretStoreError> {
        self.read(|s| s.pkce_verifier.clone())
    }

    pub(crate) fn set_pkce_verifier(&self, verifier: SecretString) -> Result<(), SecretStoreError> {
        self.mutate(|s| s.pkce_verifier = Some(verifier.expose_secret().to_owned()))
    }

    pub(crate) fn clear_pkce_verifier(&self) -> Result<(), SecretStoreError> {
        self.mutate(|s| s.pkce_verifier = None)
    }

    fn read(
        &self,
        project: impl FnOnce(&PersistedState) -> Option<String>,
    ) -> Result<Option<SecretString>, SecretStoreError> {
        let inner = self.lock()?;
        Ok(project(&inner.state).map(SecretString::from))
    }

    fn mutate(&self, change: impl FnOnce(&mut PersistedState)) -> Result<(), SecretStoreError> {
        let mut inner = self.lock()?;
        change(&mut inner.state);
        file::save(&inner.cipher, &inner.state, &inner.path)
    }

    /// Wipe all in-memory state and remove the on-disk file.
    ///
    /// Called by `AuthManager::logout`. Leaving an empty-blob
    /// `secrets.enc` on a logged-out laptop would be a needless
    /// forensic breadcrumb; we delete the file outright instead.
    pub(crate) fn wipe(&self) -> Result<(), SecretStoreError> {
        let mut inner = self.lock()?;
        // `mem::take` drops the previous `PersistedState` which
        // zeroes any populated slots via the `ZeroizeOnDrop` derive.
        let _ = std::mem::take(&mut inner.state);
        file::remove(&inner.path)
    }

    fn lock(&self) -> Result<MutexGuard<'_, Inner>, SecretStoreError> {
        self.inner.lock().map_err(|_| SecretStoreError::Poisoned)
    }
}

impl std::fmt::Debug for SecretStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Deliberately doesn't lock or expose any state — `Debug`
        // shouldn't ever surface plaintext, and tripping the mutex
        // here would create deadlock potential in error-formatting
        // paths.
        f.debug_struct("SecretStore").finish_non_exhaustive()
    }
}

/// Pull tokens out of any legacy per-secret keychain entries and into
/// `state`, then delete the keychain entries. Returns `true` if at
/// least one slot was populated from the keychain (i.e. the caller
/// must flush the file).
///
/// `keyring_get` is expected to return `None` for "no entry" (the
/// common case once migration has run once) and any read failure.
/// `keyring_delete` is best-effort and infallible from this function's
/// point of view — there's no useful response to "couldn't remove a
/// stale entry" beyond logging at the call site.
fn migrate_legacy_keychain_entries(
    state: &mut PersistedState,
    keyring_get: impl Fn(&str) -> Option<String>,
    keyring_delete: impl Fn(&str),
) -> bool {
    let slots: [(&str, &mut Option<String>); 3] = [
        (LEGACY_ACCESS_TOKEN_HANDLE, &mut state.access_token),
        (LEGACY_REFRESH_TOKEN_HANDLE, &mut state.refresh_token),
        (LEGACY_PKCE_VERIFIER_HANDLE, &mut state.pkce_verifier),
    ];

    let mut changed = false;
    for (handle, slot) in slots {
        let Some(value) = keyring_get(handle) else {
            continue;
        };
        if slot.is_none() {
            *slot = Some(value);
            changed = true;
            tracing::info!(handle, "migrated legacy keychain entry to file store");
        }
        // The entry has served its purpose; delete it whether or not
        // we adopted the value (a stale duplicate shouldn't linger).
        keyring_delete(handle);
    }

    changed
}

/// Read a legacy keychain entry via the keyring crate, mapping every
/// non-`Ok(_)` outcome to `None` (including `NoEntry` and read
/// failures). Read failures are logged so they don't disappear.
fn os_keyring_get(handle: &str) -> Option<String> {
    match keyring::Entry::new(handle, SERVICE) {
        Ok(entry) => match entry.get_password() {
            Ok(value) => Some(value),
            Err(keyring::Error::NoEntry) => None,
            Err(err) => {
                tracing::warn!(handle, error = %err, "legacy keychain read failed");
                None
            }
        },
        Err(err) => {
            tracing::warn!(handle, error = %err, "could not open legacy keychain entry");
            None
        }
    }
}

/// Best-effort delete of a legacy keychain entry.
fn os_keyring_delete(handle: &str) {
    if let Ok(entry) = keyring::Entry::new(handle, SERVICE) {
        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => {}
            Err(err) => {
                tracing::warn!(handle, error = %err, "legacy keychain delete failed");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    /// In-memory keyring used by tests. Backed by a Mutex<HashMap> so
    /// it can be shared across the read/delete closures the migration
    /// expects.
    #[derive(Default)]
    struct FakeKeyring {
        entries: Mutex<HashMap<String, String>>,
    }

    impl FakeKeyring {
        fn with(entries: &[(&str, &str)]) -> Self {
            let map = entries
                .iter()
                .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
                .collect();
            Self {
                entries: Mutex::new(map),
            }
        }

        fn get(&self, handle: &str) -> Option<String> {
            self.entries.lock().unwrap().get(handle).cloned()
        }

        fn delete(&self, handle: &str) {
            self.entries.lock().unwrap().remove(handle);
        }

        fn contains(&self, handle: &str) -> bool {
            self.entries.lock().unwrap().contains_key(handle)
        }
    }

    fn open_with_fake_keyring(dir: &Path, keyring: &FakeKeyring) -> SecretStore {
        SecretStore::open_with(dir, |h| keyring.get(h), |h| keyring.delete(h))
            .expect("open SecretStore")
    }

    fn open_with_empty_keyring(dir: &Path) -> SecretStore {
        SecretStore::open_with(dir, |_| None, |_| {}).expect("open SecretStore")
    }

    #[test]
    fn round_trip_through_open() {
        let dir = tempfile::tempdir().unwrap();
        let store = open_with_empty_keyring(dir.path());

        assert!(store.access_token().unwrap().is_none());

        store
            .set_access_token(SecretString::from("a-token"))
            .unwrap();
        store
            .set_refresh_token(SecretString::from("r-token"))
            .unwrap();
        store
            .set_pkce_verifier(SecretString::from("verifier"))
            .unwrap();

        // Reopen against the same dir to prove durability.
        drop(store);
        let store = open_with_empty_keyring(dir.path());
        assert_eq!(
            store.access_token().unwrap().unwrap().expose_secret(),
            "a-token"
        );
        assert_eq!(
            store.refresh_token().unwrap().unwrap().expose_secret(),
            "r-token"
        );
        assert_eq!(
            store.pkce_verifier().unwrap().unwrap().expose_secret(),
            "verifier"
        );
    }

    #[test]
    fn clear_pkce_verifier_persists() {
        let dir = tempfile::tempdir().unwrap();
        let store = open_with_empty_keyring(dir.path());
        store
            .set_pkce_verifier(SecretString::from("verifier"))
            .unwrap();
        store.clear_pkce_verifier().unwrap();

        drop(store);
        let store = open_with_empty_keyring(dir.path());
        assert!(store.pkce_verifier().unwrap().is_none());
    }

    #[test]
    fn wipe_removes_file_and_state() {
        let dir = tempfile::tempdir().unwrap();
        let store = open_with_empty_keyring(dir.path());
        store
            .set_access_token(SecretString::from("a-token"))
            .unwrap();

        let path = dir.path().join(file::STORE_FILENAME);
        assert!(path.exists());

        store.wipe().unwrap();
        assert!(!path.exists());
        assert!(store.access_token().unwrap().is_none());
    }

    #[test]
    fn migrates_legacy_keychain_entries_on_open() {
        let keyring = FakeKeyring::with(&[
            (LEGACY_ACCESS_TOKEN_HANDLE, "legacy-access"),
            (LEGACY_REFRESH_TOKEN_HANDLE, "legacy-refresh"),
        ]);

        let dir = tempfile::tempdir().unwrap();
        let store = open_with_fake_keyring(dir.path(), &keyring);

        // Tokens lifted into the store.
        assert_eq!(
            store.access_token().unwrap().unwrap().expose_secret(),
            "legacy-access"
        );
        assert_eq!(
            store.refresh_token().unwrap().unwrap().expose_secret(),
            "legacy-refresh"
        );
        // PKCE verifier was never seeded; stays empty.
        assert!(store.pkce_verifier().unwrap().is_none());

        // Legacy entries deleted post-migration.
        assert!(!keyring.contains(LEGACY_ACCESS_TOKEN_HANDLE));
        assert!(!keyring.contains(LEGACY_REFRESH_TOKEN_HANDLE));

        // Reopening with an empty keychain leaves the migrated values in place.
        drop(store);
        let store = open_with_empty_keyring(dir.path());
        assert_eq!(
            store.access_token().unwrap().unwrap().expose_secret(),
            "legacy-access"
        );
    }

    #[test]
    fn migration_skips_slots_already_present_in_file() {
        let dir = tempfile::tempdir().unwrap();
        {
            let store = open_with_empty_keyring(dir.path());
            store
                .set_access_token(SecretString::from("from-file"))
                .unwrap();
        }

        // A stale legacy entry shouldn't overwrite the file value,
        // but the migration should still clean it up so it can't keep
        // confusing future opens.
        let keyring = FakeKeyring::with(&[(LEGACY_ACCESS_TOKEN_HANDLE, "from-keychain")]);
        let store = open_with_fake_keyring(dir.path(), &keyring);

        assert_eq!(
            store.access_token().unwrap().unwrap().expose_secret(),
            "from-file"
        );
        assert!(!keyring.contains(LEGACY_ACCESS_TOKEN_HANDLE));
    }
}
