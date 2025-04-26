use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: String,
    pub title: Option<String>,
    pub icon: Option<String>,
}

impl Document {
    pub fn new(id: String, name: String) -> Self {
        Self {
            id,
            title: Some(name),
            icon: None,
        }
    }
}
