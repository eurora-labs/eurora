//! Wire protocol shared between the Eurora desktop app and connected
//! clients (browser native-messaging hosts, Office.js add-ins, future
//! first-party integrations).
//!
//! The transport is JSON over a plaintext loopback WebSocket
//! (`ws://localhost:1431/bridge`). Every message in either direction
//! is a [`Frame`] whose `kind` discriminator identifies the payload
//! variant. The JSON shape is deliberately hand-tuned to match the
//! externally-tagged-enum form already consumed by the browser
//! extension at
//! `apps/browser/src/shared/background/native-messenger.ts`.
//!
//! TypeScript and Swift bindings are generated from these types via
//! `cargo run -p euro-bridge-protocol --features codegen -- --generate_specta`.
//!
//! ## Transport
//!
//! The bridge is loopback-only — the desktop binds [`BRIDGE_BIND_IP`]
//! (`127.0.0.1`) plus a best-effort `[::1]` listener on the same port,
//! and rejects non-loopback peers at upgrade time, so the channel
//! never leaves the kernel. The dual-stack bind matters in practice
//! because some Windows configurations resolve `localhost` to `::1`
//! ahead of `127.0.0.1`, and a client that tries the IPv6 address
//! first must find a listener there.
//!
//! With the loopback constraint in place we serve plaintext: `ws://`
//! carries no confidentiality cost a local attacker doesn't already
//! have (they'd need user-level code execution to sniff loopback, at
//! which point reading on-disk state is strictly easier than decoding
//! our frames). The
//! [register-frame token check](`crate::frame::RegisterFrame`) is the
//! authentication boundary, not TLS.

mod error;
mod frame;

use std::net::{IpAddr, Ipv4Addr, SocketAddr};

pub use error::BridgeError;
pub use frame::{
    CancelFrame, ErrorFrame, EventFrame, Frame, FrameKind, RegisterFrame, RequestFrame,
    ResponseFrame, ShutdownFrame,
};

/// Hostname clients dial. Resolved to [`BRIDGE_BIND_IP`] by the OS
/// resolver; both Chromium-based hosts (WebView2) and WKWebView treat
/// `localhost` as a [potentially trustworthy origin][secure-context],
/// which is what allows an HTTPS-loaded Office add-in to open a
/// plaintext `ws://localhost` connection without mixed-content blocks.
///
/// [secure-context]: https://w3c.github.io/webappsec-secure-contexts/
pub const BRIDGE_HOST: &str = "localhost";

/// Primary loopback IP the desktop binds the listener to. Separate
/// from [`BRIDGE_HOST`] because the hostname is non-routable on its
/// own and the listener needs a concrete IP. The bridge service
/// additionally opens a best-effort `[::1]` listener on the same port
/// so clients whose resolver returns IPv6 from `localhost` aren't
/// refused — see `BridgeService::bind_on` in `euro-bridge`.
pub const BRIDGE_BIND_IP: IpAddr = IpAddr::V4(Ipv4Addr::LOCALHOST);

/// Port the desktop bridge listens on.
pub const BRIDGE_PORT: u16 = 1431;

/// HTTP path that performs the WebSocket upgrade.
pub const BRIDGE_PATH: &str = "/bridge";

/// URL scheme. Plaintext WebSocket — see the module-level docs for
/// the rationale and threat model.
pub const BRIDGE_SCHEME: &str = "ws";

/// Convenience: full WebSocket URL for connecting to the local bridge
/// on its well-known port.
pub fn bridge_url() -> String {
    format!("{BRIDGE_SCHEME}://{BRIDGE_HOST}:{BRIDGE_PORT}{BRIDGE_PATH}")
}

/// Build a bridge WebSocket URL for an arbitrary bound port. Used by
/// tests that bind the bridge to an ephemeral port (port `0`) and
/// then need a URL whose hostname routes through the OS resolver to
/// the loopback interface (`localhost`). The IP component of `addr`
/// is intentionally discarded.
pub fn bridge_url_for(addr: SocketAddr) -> String {
    format!(
        "{BRIDGE_SCHEME}://{BRIDGE_HOST}:{}{BRIDGE_PATH}",
        addr.port()
    )
}

/// Build the [`specta::Types`] containing every type that
/// participates in the wire protocol. Used by the codegen binary and
/// available to other crates that want to merge these types into a
/// larger collection.
pub fn type_collection() -> specta::Types {
    specta::Types::default()
        .register::<Frame>()
        .register::<FrameKind>()
        .register::<RequestFrame>()
        .register::<ResponseFrame>()
        .register::<EventFrame>()
        .register::<ErrorFrame>()
        .register::<CancelFrame>()
        .register::<RegisterFrame>()
        .register::<ShutdownFrame>()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bridge_url_uses_plaintext_ws() {
        assert_eq!(bridge_url(), "ws://localhost:1431/bridge");
    }

    #[test]
    fn bridge_url_for_uses_supplied_port() {
        let addr: SocketAddr = "127.0.0.1:54321".parse().unwrap();
        assert_eq!(bridge_url_for(addr), "ws://localhost:54321/bridge");
    }
}
