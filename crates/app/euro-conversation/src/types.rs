use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Conversation {
    id: Option<Uuid>,
    title: String,

    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}
