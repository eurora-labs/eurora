use std::path::PathBuf;

/// Errors surfaced by [`super::SecretStore`].
///
/// `Encrypt` and `Decrypt` deliberately discard the underlying
/// `chacha20poly1305::Error`: the upstream type only ever distinguishes
/// "the cipher rejected this" without leaking which part of the input
/// caused the rejection, so logging it adds noise without informing a
/// response.
#[derive(Debug, thiserror::Error)]
pub(crate) enum SecretStoreError {
    #[error("secret store I/O failed at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("encrypting secret store failed")]
    Encrypt,
    #[error("decrypting secret store failed (wrong key or corrupted file)")]
    Decrypt,
    #[error("(de)serialising secret store failed: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("OS keychain operation failed: {0}")]
    Keyring(#[from] keyring::Error),
    #[error("main key {0}")]
    MainKey(&'static str),
    #[error("secret store mutex poisoned")]
    Poisoned,
}
