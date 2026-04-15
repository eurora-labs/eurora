use std::collections::HashMap;

use euro_thread::ThreadManager;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

pub type SharedThreadManager = Mutex<ThreadManager>;
pub type ActiveStreamTokens = Mutex<HashMap<String, CancellationToken>>;
