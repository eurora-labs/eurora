use eur_db_schemas::conversation;
use rusqlite::{Connection, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let conn = Connection::open("eur_personal_database.sqlite")?;
    // let conn = Connection::open_in_memory()?;
    conversation::create_schema(&conn).await?;

    Ok(())
}
