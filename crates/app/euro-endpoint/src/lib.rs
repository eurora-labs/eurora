#[cfg(not(any(feature = "tls-native-roots", feature = "tls-webpki-roots")))]
compile_error!(
    "euro-endpoint requires a TLS root provider. \
     Enable either the `tls-native-roots` feature (desktop) \
     or the `tls-webpki-roots` feature (mobile)."
);

mod error;

pub use error::{EndpointError, Result};

use std::sync::RwLock;

use url::Url;

/// Default base URL the binary was compiled against. Baked at
/// compile time from `BACKEND_URL` (workspace `.env`) so forks
/// rebuilding for their own organisation pick up the right endpoint
/// without source changes.
pub const DEFAULT_API_URL: &str = env!("BACKEND_URL");

/// Owns the live backend base URL plus a single shared [`reqwest::Client`].
///
/// The client is cheap to clone (internally `Arc`-based) and shares its
/// connection pool across every consumer that takes one from
/// [`EndpointManager::client`]. The base URL is parsed up front and
/// re-validated on every change via [`EndpointManager::set_global_backend_url`].
pub struct EndpointManager {
    client: reqwest::Client,
    base_url: RwLock<Url>,
}

impl EndpointManager {
    pub fn new(initial_url: &str) -> Result<Self> {
        let url = if initial_url.is_empty() {
            DEFAULT_API_URL
        } else {
            initial_url
        };

        let base_url = parse_base_url(url)?;
        let client = reqwest::Client::builder()
            .build()
            .map_err(EndpointError::Build)?;

        Ok(Self {
            client,
            base_url: RwLock::new(base_url),
        })
    }

    /// Returns the URL the manager is currently pointing at.
    ///
    /// Stays in sync with whatever [`EndpointManager::set_global_backend_url`]
    /// was last called with. Always carries a trailing slash on its path so
    /// callers can `join` relative paths against it.
    pub fn current_url(&self) -> Url {
        self.base_url.read().unwrap().clone()
    }

    /// Returns a clone of the shared HTTP client. Cloning is essentially free
    /// (an internal `Arc` bump) and the resulting client shares its
    /// connection pool with every other clone taken from the same
    /// [`EndpointManager`].
    pub fn client(&self) -> reqwest::Client {
        self.client.clone()
    }

    /// Build an absolute URL for `path` against the current base URL.
    ///
    /// `path` may begin with `/` or not — the base URL is normalised to end
    /// with `/`, so both forms produce the same absolute URL. Panics only on
    /// a programmer bug: a `path` containing characters that don't form a
    /// valid relative URL.
    pub fn url(&self, path: &str) -> Url {
        let relative = path.strip_prefix('/').unwrap_or(path);
        self.current_url()
            .join(relative)
            .expect("path forms a valid relative URL against the base")
    }

    pub fn set_global_backend_url(&self, url: &str) -> Result<()> {
        let parsed = parse_base_url(url)?;
        *self.base_url.write().unwrap() = parsed;
        tracing::info!("Switched API endpoint");
        Ok(())
    }
}

// Normalise the base URL so its path ends with `/`, ensuring `Url::join`
// treats it as a directory rather than replacing its last segment.
fn parse_base_url(input: &str) -> Result<Url> {
    let mut url = Url::parse(input).map_err(EndpointError::InvalidUrl)?;
    if !url.path().ends_with('/') {
        let mut path = url.path().to_owned();
        path.push('/');
        url.set_path(&path);
    }
    Ok(url)
}
