pub mod parent_pid;
pub mod types;
pub mod utils;

pub use euro_bridge_protocol::{
    BRIDGE_HOST, BRIDGE_PATH, BRIDGE_PORT, CancelFrame, ErrorFrame, EventFrame, Frame, FrameKind,
    RegisterFrame, RequestFrame, ResponseFrame, bridge_url,
};
pub use types::*;

/// Cap on the size of any single JSON frame exchanged with Chrome over
/// stdin/stdout. Chrome's native messaging max is 1 MiB; we allow more
/// here for SNAPSHOT/ASSETS payloads that include base64-encoded
/// images.
pub const MAX_FRAME_SIZE: usize = 16 * 1024 * 1024;
