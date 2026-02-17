use std::ops::Deref;

/// A lightweight wrapper that redacts its value in `Debug` output.
///
/// Use this for secrets that must not appear in logs or debug prints.
/// Backend-compatible alternative to `euro-secret::Sensitive` (which
/// requires OS keyring dependencies).
#[derive(Clone)]
pub struct Redacted<T>(T);

impl<T> Redacted<T> {
    pub fn new(value: T) -> Self {
        Self(value)
    }

    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> std::fmt::Debug for Redacted<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("<redacted>")
    }
}

impl<T> Deref for Redacted<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> From<T> for Redacted<T> {
    fn from(value: T) -> Self {
        Self(value)
    }
}

impl<T: PartialEq> PartialEq for Redacted<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}
