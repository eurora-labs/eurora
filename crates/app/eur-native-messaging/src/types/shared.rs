use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Serialize, Deserialize, Default, Type)]
pub struct NativeTwitterTweet {
    pub text: String,
    pub timestamp: Option<String>,
    pub author: Option<String>,
}
