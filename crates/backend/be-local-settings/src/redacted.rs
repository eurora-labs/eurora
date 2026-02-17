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

impl Redacted<String> {
    /// Returns a masked version of the string for safe display.
    /// Shows the first 3 characters followed by `****`, or just `****`
    /// if the string is too short.
    pub fn masked(&self) -> String {
        if self.0.len() > 3 {
            format!("{}****", &self.0[..3])
        } else {
            "****".to_string()
        }
    }
}

impl PartialEq for Redacted<String> {
    fn eq(&self, other: &Self) -> bool {
        let a = self.0.as_bytes();
        let b = other.0.as_bytes();
        if a.len() != b.len() {
            return false;
        }
        let mut diff = 0u8;
        for (x, y) in a.iter().zip(b.iter()) {
            diff |= x ^ y;
        }
        diff == 0
    }
}
