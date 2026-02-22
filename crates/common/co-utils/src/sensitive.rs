use std::ops::{Deref, DerefMut};

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use zeroize::Zeroize;

/// A type to clearly mark sensitive information using the type-system. As such, it should
///
/// * *not* be logged
/// * *not* be stored in plain text
/// * *not* be presented in any way unless the user explicitly confirmed it to be displayed.
///
/// The inner value is automatically zeroized when the `Sensitive` wrapper is dropped,
/// preventing secret material from lingering in memory.
pub struct Sensitive<T: Zeroize>(pub T);

impl<T: Zeroize> Drop for Sensitive<T> {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

impl<T> Serialize for Sensitive<T>
where
    T: Serialize + Zeroize,
{
    fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        unreachable!(
            "BUG: Sensitive data cannot be serialized - it needs to be extracted and put into a struct for serialization explicitly"
        )
    }
}
impl<'de, T> Deserialize<'de> for Sensitive<T>
where
    T: Deserialize<'de> + Zeroize,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        T::deserialize(deserializer).map(Sensitive)
    }
}

impl<T> std::fmt::Debug for Sensitive<T>
where
    T: std::fmt::Debug + Zeroize,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt("<redacted>", f)
    }
}

impl<T> Default for Sensitive<T>
where
    T: Default + Zeroize,
{
    fn default() -> Self {
        Self(T::default())
    }
}

impl<T: Zeroize> Deref for Sensitive<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: Zeroize> DerefMut for Sensitive<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T: Zeroize + Default> Sensitive<T> {
    /// Consume the wrapper and return the inner value.
    ///
    /// The returned value is **not** automatically zeroized — the caller takes
    /// ownership and responsibility for it.
    pub fn into_inner(mut self) -> T {
        std::mem::take(&mut self.0)
        // `self` is dropped here, zeroizing the now-default value (no-op).
    }
}

impl<T> Clone for Sensitive<T>
where
    T: Clone + Zeroize,
{
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
