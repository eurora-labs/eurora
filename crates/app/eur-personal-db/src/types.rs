use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(FromRow, Debug, Serialize, Deserialize)]
pub struct Conversation {
    pub id: String,
    pub title: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(FromRow, Debug, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: String,
    pub conversation_id: String,
    pub role: String,
    pub content: String,
    pub visible: bool,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Activity table structure
#[derive(FromRow, Debug)]
pub struct Activity {
    pub id: String,
    pub name: String,
    pub app_name: String,
    pub window_name: String,
    pub started_at: String,
    pub ended_at: Option<String>,
}

/// Activity asset table structure
#[derive(FromRow, Debug)]
pub struct ActivityAsset {
    pub id: String,
    pub activity_id: String,
    pub data: String, // JSON blob stored as text

    pub created_at: String,
    pub updated_at: String,
}

/// Video chunk table structure
#[derive(FromRow, Debug)]
pub struct VideoChunk {
    pub id: String,
    pub file_path: String,
}

/// Frame table structure
#[derive(FromRow, Debug)]
pub struct Frame {
    pub id: String,
    pub video_chunk_id: String,
    pub relative_index: i32,
}

/// Activity snapshot table structure
#[derive(FromRow, Debug)]
pub struct ActivitySnapshot {
    pub id: String,
    pub frame_id: String,
    pub activity_id: String,
}

/// Frame text table structure
#[derive(FromRow, Debug)]
pub struct FrameText {
    pub id: String,
    pub frame_id: String,
    pub text: String,
    pub text_json: Option<String>,
    pub ocr_engine: String,
}

// /// Database schema initialization
// pub async fn initialize_db(db_path: &str) -> Result<SqlitePool, sqlx::Error> {
//     // Create a connection pool to the SQLite database
//     let pool = SqlitePool::connect(db_path).await?;

//     // Run migrations
//     sqlx::migrate!("./src/migrations").run(&pool).await?;

//     Ok(pool)
// }

// // Query helper functions for common operations

// /// Get all activities
// pub async fn get_all_activities(pool: &SqlitePool) -> Result<Vec<Activity>, sqlx::Error> {
//     sqlx::query_as!(
//         Activity,
//         r#"
//         SELECT id, name, app_name, window_name, started_at, ended_at
//         FROM activity
//         "#
//     )
//     .fetch_all(pool)
//     .await
// }

// /// Get activity by id
// pub async fn get_activity(pool: &SqlitePool, id: &str) -> Result<Option<Activity>, sqlx::Error> {
//     sqlx::query_as!(
//         Activity,
//         r#"
//         SELECT id, name, app_name, window_name, started_at, ended_at
//         FROM activity
//         WHERE id = ?
//         "#,
//         id
//     )
//     .fetch_optional(pool)
//     .await
// }

// /// Get frames for a video chunk
// pub async fn get_frames_for_video_chunk(
//     pool: &SqlitePool,
//     video_chunk_id: &str,
// ) -> Result<Vec<Frame>, sqlx::Error> {
//     sqlx::query_as!(
//         Frame,
//         r#"
//         SELECT id, video_chunk_id, relative_index
//         FROM frame
//         WHERE video_chunk_id = ?
//         ORDER BY relative_index
//         "#,
//         video_chunk_id
//     )
//     .fetch_all(pool)
//     .await
// }

// /// Get text for a frame
// pub async fn get_frame_text(
//     pool: &SqlitePool,
//     frame_id: &str,
// ) -> Result<Vec<FrameText>, sqlx::Error> {
//     sqlx::query_as!(
//         FrameText,
//         r#"
//         SELECT id, frame_id, text, text_json, ocr_engine
//         FROM frame_text
//         WHERE frame_id = ?
//         "#,
//         frame_id
//     )
//     .fetch_all(pool)
//     .await
// }

// /// Get activity snapshots for an activity
// pub async fn get_snapshots_for_activity(
//     pool: &SqlitePool,
//     activity_id: &str,
// ) -> Result<Vec<ActivitySnapshot>, sqlx::Error> {
//     sqlx::query_as!(
//         ActivitySnapshot,
//         r#"
//         SELECT id, frame_id, activity_id
//         FROM activity_snapshot
//         WHERE activity_id = ?
//         "#,
//         activity_id
//     )
//     .fetch_all(pool)
//     .await
// }
