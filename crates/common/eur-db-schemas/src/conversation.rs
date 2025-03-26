use rusqlite::{Connection, Result};

#[derive(Debug)]
pub struct Conversation {
    pub id: i32,
    pub messages: Vec<ChatMessage>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    pub visible: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

pub async fn create_schema(conn: &Connection) -> Result<()> {
    conn.execute(
        "CREATE TABLE conversation (
            id INTEGER PRIMARY KEY,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        )",
        (),
    )?;

    conn.execute(
        "CREATE TABLE chat_message (
            id INTEGER PRIMARY KEY,
            role TEXT NOT NULL,
            content TEXT NOT NULL,
            visible INTEGER NOT NULL,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            conversation_id INTEGER NOT NULL,
            FOREIGN KEY (conversation_id) REFERENCES conversation (id)
        )",
        (),
    )?;

    Ok(())
}
