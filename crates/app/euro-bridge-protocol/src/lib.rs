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
    CancelFrame, ClientKind, ErrorFrame, EventFrame, Frame, FrameKind, RegisterFrame, RequestFrame,
    ResponseFrame,
};

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
        .register::<ClientKind>()
}
