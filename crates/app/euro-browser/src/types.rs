use serde::{Deserialize, Serialize};
use specta::Type;

mod metadata;

pub use metadata::*;

/// Envelope for every payload the browser native-messaging host
/// exchanges with the desktop bridge. Externally tagged on `kind` with
/// the inner payload under `data` so the JSON shape stays stable as
/// new wire-payload variants are added.
///
/// At present only [`NativeMetadata`] crosses the bridge — page content
/// is delivered through granular adapter tools (`browser::web::*`,
/// `browser::youtube::*`, …) rather than through pre-bundled assets or
/// snapshots.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(tag = "kind", content = "data")]
pub enum NativeMessage {
    NativeMetadata(NativeMetadata),
}
