use serde::{Deserialize, Serialize};

#[cfg(feature = "specta")]
use specta::Type;

/// Third-party identity provider supported by the auth service.
///
/// Wire format is lowercase JSON (`"google"`, `"github"`, `"apple"`)
/// so it reads naturally in URLs and request bodies.
#[derive(Debug, Serialize, Deserialize, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(feature = "specta", derive(Type))]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    Google,
    Github,
    Apple,
}

impl Provider {
    pub fn as_str(self) -> &'static str {
        match self {
            Provider::Google => "google",
            Provider::Github => "github",
            Provider::Apple => "apple",
        }
    }
}

impl std::fmt::Display for Provider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}
