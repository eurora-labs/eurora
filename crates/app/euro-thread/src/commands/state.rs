use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::ThreadManager;

/// `ThreadManager` is `Clone` and stateless across calls, so we share it via
/// a plain `Arc` instead of wrapping it in a mutex — handlers concurrently
/// hit the HTTP API and the WebSocket without contention on this state.
pub type SharedThreadManager = Arc<ThreadManager>;

/// Per-thread cancellation tokens for in-flight chat streams. Inserted when
/// a stream begins, removed when it terminates (or by `chat_cancel_query`).
pub type ActiveStreamTokens = Mutex<HashMap<Uuid, CancellationToken>>;
