//! Wire protocol shared between the Eurora desktop app and connected
//! clients (browser native-messaging hosts, Office.js add-ins, future
//! first-party integrations).
//!
//! The transport is JSON over WebSocket. Every message in either
//! direction is a [`Frame`] whose `kind` discriminator identifies the
//! payload variant. The JSON shape is deliberately hand-tuned to match
//! the externally-tagged-enum form already consumed by the browser
//! extension at `apps/browser/src/shared/background/native-messenger.ts`.
//!
//! TypeScript and Swift bindings are generated from these types via
//! `cargo run -p euro-bridge-protocol --features codegen -- --generate_specta`.

mod error;
mod frame;

pub use error::BridgeError;
pub use frame::{
    CancelFrame, ErrorFrame, EventFrame, Frame, FrameKind, RegisterFrame, RequestFrame,
    ResponseFrame,
};

/// Loopback address the desktop bridge listens on. Bridge clients
/// (browser native-messaging hosts, Office add-ins, …) connect here.
pub const BRIDGE_HOST: &str = "127.0.0.1";

/// Port the desktop bridge listens on.
pub const BRIDGE_PORT: u16 = 1431;

/// HTTP path that performs the WebSocket upgrade.
pub const BRIDGE_PATH: &str = "/bridge";

/// Convenience: full WebSocket URL for connecting to the local bridge.
pub fn bridge_url() -> String {
    format!("ws://{BRIDGE_HOST}:{BRIDGE_PORT}{BRIDGE_PATH}")
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
