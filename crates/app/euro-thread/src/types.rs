use agent_chain::BaseMessage;
use chrono::{DateTime, TimeZone, Utc};
use proto_gen::thread::Thread as ProtoThread;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{Error, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thread {
    id: Option<Uuid>,
    title: String,
    messages: Vec<BaseMessage>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl Thread {
    pub fn id(&self) -> Option<Uuid> {
        self.id
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn set_id(&mut self, id: Uuid) -> Result<()> {
        if self.id.is_none() {
            self.id = Some(id);
            return Ok(());
        }
        Err(Error::SetId("Thread ID is already set".to_string()))
    }

    pub fn messages(&self) -> &Vec<BaseMessage> {
        &self.messages
    }
}

impl Default for Thread {
    fn default() -> Self {
        let title = "New Chat".to_string();
        let created_at = Utc::now();
        let updated_at = created_at;

        Self {
            id: None,
            title,
            messages: Vec::new(),
            created_at,
            updated_at,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ThreadEvent {
    NewThread { id: Option<Uuid>, title: String },
}

impl From<ProtoThread> for Thread {
    fn from(c: ProtoThread) -> Self {
        let id = Some(Uuid::parse_str(&c.id).expect("Thread id is not a valid uuid"));
        let title = c.title;
        let created_at = c.created_at.expect("created_at is required");
        let created_at: DateTime<Utc> = Utc
            .timestamp_opt(created_at.seconds, created_at.nanos as u32)
            .unwrap();
        let updated_at = c.updated_at.expect("updated_at is required");
        let updated_at: DateTime<Utc> = Utc
            .timestamp_opt(updated_at.seconds, updated_at.nanos as u32)
            .unwrap();

        Self {
            id,
            title,
            messages: Vec::new(),
            created_at,
            updated_at,
        }
    }
}
