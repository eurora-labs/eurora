//! Wire protocol shared between the Eurora desktop app and connected
//! clients (browser native-messaging hosts, Office.js add-ins, future
//! first-party integrations).
//!
//! The transport is JSON over a `wss://` WebSocket terminated on a
//! per-user trust chain provisioned by the desktop on first run. Every
//! message in either direction is a [`Frame`] whose `kind` discriminator
//! identifies the payload variant. The JSON shape is deliberately
//! hand-tuned to match the externally-tagged-enum form already consumed
//! by the browser extension at
//! `apps/browser/src/shared/background/native-messenger.ts`.
//!
//! TypeScript and Swift bindings are generated from these types via
//! `cargo run -p euro-bridge-protocol --features codegen -- --generate_specta`.

mod error;
mod frame;

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};

pub use error::BridgeError;
pub use frame::{
    CancelFrame, ErrorFrame, EventFrame, Frame, FrameKind, RegisterFrame, RequestFrame,
    ResponseFrame,
};

/// SNI hostname clients dial. Must match the leaf certificate's DNS SAN —
/// IP-based SNI is fragile across TLS stacks (notably WebView2) so the
/// canonical address is hostname-based, not IP-based.
pub const BRIDGE_HOST: &str = "localhost";

/// Loopback IP the desktop binds the listener to. Separate from
/// [`BRIDGE_HOST`] because the SNI hostname is non-routable on its own
/// and the listener needs a concrete IP.
pub const BRIDGE_BIND_IP: IpAddr = IpAddr::V4(Ipv4Addr::LOCALHOST);

/// Port the desktop bridge listens on.
pub const BRIDGE_PORT: u16 = 1431;

/// HTTP path that performs the WebSocket upgrade.
pub const BRIDGE_PATH: &str = "/bridge";

/// URL scheme. WebSocket-over-TLS only — there is no plaintext fallback.
pub const BRIDGE_SCHEME: &str = "wss";

/// File name the desktop writes the bridge CA cert under inside
/// [`bridge_data_dir`]. Shared with the native-messaging host so both
/// sides agree on the path without coordination.
pub const BRIDGE_CA_FILENAME: &str = "ca.crt";

/// Top-level subdirectory under the platform data dir that contains
/// every Eurora-owned per-user file. All callers that compose paths
/// under this root must do so via [`eurora_data_root_under`] or
/// [`eurora_data_root`] so the literal lives in exactly one place.
pub const EURORA_DATA_SUBDIR: &str = "Eurora";

/// Subdirectory under [`EURORA_DATA_SUBDIR`] that holds bridge TLS
/// material.
pub const BRIDGE_DATA_SUBDIR: &str = "bridge";

/// Convenience: full WebSocket URL for connecting to the local bridge
/// on its well-known port.
pub fn bridge_url() -> String {
    format!("{BRIDGE_SCHEME}://{BRIDGE_HOST}:{BRIDGE_PORT}{BRIDGE_PATH}")
}

/// Build a bridge WebSocket URL for an arbitrary bound port. Used by
/// tests that bind the bridge to an ephemeral port (port `0`) and then
/// need a URL whose SNI hostname still matches the cert's DNS SAN
/// (`localhost`). The IP component of `addr` is intentionally discarded.
pub fn bridge_url_for(addr: SocketAddr) -> String {
    format!(
        "{BRIDGE_SCHEME}://{BRIDGE_HOST}:{}{BRIDGE_PATH}",
        addr.port()
    )
}

/// Eurora's per-user data root underneath an arbitrary platform data
/// dir. Exposed so callers that already hold a resolved data dir
/// (e.g. Tauri's `app.path().data_dir()`) compose paths through the
/// same join the `dirs`-based [`eurora_data_root`] uses, rather than
/// hand-concatenating [`EURORA_DATA_SUBDIR`].
pub fn eurora_data_root_under(data_dir: &Path) -> PathBuf {
    data_dir.join(EURORA_DATA_SUBDIR)
}

/// Directory holding bridge TLS material underneath an arbitrary
/// platform data dir. Counterpart to [`bridge_data_dir`] for callers
/// that resolve the platform data dir themselves.
pub fn bridge_data_dir_under(data_dir: &Path) -> PathBuf {
    eurora_data_root_under(data_dir).join(BRIDGE_DATA_SUBDIR)
}

/// Eurora's per-user data root, resolved via the `dirs` crate. Mirrors
/// the convention used by the Office add-in install code: both Tauri
/// (`app.path().data_dir()`) and the standalone uninstall CLI converge
/// on the same path here, so cross-process readers (e.g. the
/// native-messaging host) can derive bridge file locations without
/// having to be passed through Tauri's runtime.
pub fn eurora_data_root() -> Option<PathBuf> {
    dirs::data_dir().map(|root| eurora_data_root_under(&root))
}

/// Directory holding bridge TLS material. Both the desktop (writer)
/// and the native-messaging host (reader) resolve this path the same
/// way so they agree without out-of-band coordination.
pub fn bridge_data_dir() -> Option<PathBuf> {
    dirs::data_dir().map(|root| bridge_data_dir_under(&root))
}

/// Path the desktop writes the bridge CA cert to and that other local
/// clients (the native-messaging host) read it from.
pub fn bridge_ca_path() -> Option<PathBuf> {
    bridge_data_dir().map(|dir| dir.join(BRIDGE_CA_FILENAME))
}

/// Build the [`specta::TypeCollection`] containing every type that
/// participates in the wire protocol. Used by the codegen binary and
/// available to other crates that want to merge these types into a
/// larger collection.
pub fn type_collection() -> specta::TypeCollection {
    specta::TypeCollection::default()
        .register::<Frame>()
        .register::<FrameKind>()
        .register::<RequestFrame>()
        .register::<ResponseFrame>()
        .register::<EventFrame>()
        .register::<ErrorFrame>()
        .register::<CancelFrame>()
        .register::<RegisterFrame>()
}
