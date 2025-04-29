// // Eurora Personal DB - SQLite database interface for Eurora personal data

// mod schema;

// use sqlx::sqlite::SqlitePool;
// use std::path::Path;
// use uuid::Uuid;

// pub use schema::{
//     Activity, ActivityAsset, ActivitySnapshot, Frame, FrameText, VideoChunk, get_activity,
//     get_all_activities, get_frame_text, get_frames_for_video_chunk, get_snapshots_for_activity,
// };

// /// PersonalDb provides a high-level interface to the Eurora personal database
// pub struct PersonalDb {
//     pool: SqlitePool,
// }

// impl PersonalDb {
//     /// Create a new PersonalDb instance
//     pub async fn new(db_path: &str) -> Result<Self, sqlx::Error> {
//         // Ensure the parent directory exists
//         if let Some(parent) = Path::new(db_path).parent() {
//             std::fs::create_dir_all(parent).map_err(|e| {
//                 sqlx::Error::Configuration(format!("Failed to create directory: {}", e).into())
//             })?;
//         }

//         // Initialize the database
//         let pool = schema::initialize_db(db_path).await?;

//         Ok(Self { pool })
//     }

//     /// Get the SQLite connection pool
//     pub fn pool(&self) -> &SqlitePool {
//         &self.pool
//     }

//     /// Insert a new activity
//     pub async fn insert_activity(
//         &self,
//         name: &str,
//         app_name: &str,
//         window_name: &str,
//     ) -> Result<String, sqlx::Error> {
//         let id = Uuid::new_v4().to_string();
//         let now = chrono::Utc::now().to_rfc3339();

//         sqlx::query!(
//             r#"
//             INSERT INTO activity (id, name, app_name, window_name, started_at)
//             VALUES (?, ?, ?, ?, ?)
//             "#,
//             id,
//             name,
//             app_name,
//             window_name,
//             now
//         )
//         .execute(&self.pool)
//         .await?;

//         Ok(id)
//     }

//     /// End an activity by setting its end time
//     pub async fn end_activity(&self, id: &str) -> Result<(), sqlx::Error> {
//         let now = chrono::Utc::now().to_rfc3339();

//         sqlx::query!(
//             r#"
//             UPDATE activity
//             SET ended_at = ?
//             WHERE id = ?
//             "#,
//             now,
//             id
//         )
//         .execute(&self.pool)
//         .await?;

//         Ok(())
//     }

//     /// Add an asset to an activity
//     pub async fn add_activity_asset(
//         &self,
//         activity_id: &str,
//         data: &str,
//     ) -> Result<String, sqlx::Error> {
//         let id = Uuid::new_v4().to_string();
//         let now = chrono::Utc::now().to_rfc3339();

//         sqlx::query!(
//             r#"
//             INSERT INTO activity_asset (id, activity_id, data, created_at, updated_at)
//             VALUES (?, ?, ?, ?, ?)
//             "#,
//             id,
//             activity_id,
//             data,
//             now,
//             now
//         )
//         .execute(&self.pool)
//         .await?;

//         Ok(id)
//     }

//     /// Add a video chunk
//     pub async fn add_video_chunk(&self, file_path: &str) -> Result<String, sqlx::Error> {
//         let id = Uuid::new_v4().to_string();

//         sqlx::query!(
//             r#"
//             INSERT INTO video_chunk (id, file_path)
//             VALUES (?, ?)
//             "#,
//             id,
//             file_path
//         )
//         .execute(&self.pool)
//         .await?;

//         Ok(id)
//     }

//     /// Add a frame to a video chunk
//     pub async fn add_frame(
//         &self,
//         video_chunk_id: &str,
//         relative_index: i32,
//     ) -> Result<String, sqlx::Error> {
//         let id = Uuid::new_v4().to_string();

//         sqlx::query!(
//             r#"
//             INSERT INTO frame (id, video_chunk_id, relative_index)
//             VALUES (?, ?, ?)
//             "#,
//             id,
//             video_chunk_id,
//             relative_index
//         )
//         .execute(&self.pool)
//         .await?;

//         Ok(id)
//     }

//     /// Add text to a frame
//     pub async fn add_frame_text(
//         &self,
//         frame_id: &str,
//         text: &str,
//         text_json: Option<&str>,
//         ocr_engine: &str,
//     ) -> Result<String, sqlx::Error> {
//         let id = Uuid::new_v4().to_string();

//         sqlx::query!(
//             r#"
//             INSERT INTO frame_text (id, frame_id, text, text_json, ocr_engine)
//             VALUES (?, ?, ?, ?, ?)
//             "#,
//             id,
//             frame_id,
//             text,
//             text_json,
//             ocr_engine
//         )
//         .execute(&self.pool)
//         .await?;

//         Ok(id)
//     }

//     /// Create a snapshot linking an activity to a frame
//     pub async fn create_activity_snapshot(
//         &self,
//         activity_id: &str,
//         frame_id: &str,
//     ) -> Result<String, sqlx::Error> {
//         let id = Uuid::new_v4().to_string();

//         sqlx::query!(
//             r#"
//             INSERT INTO activity_snapshot (id, activity_id, frame_id)
//             VALUES (?, ?, ?)
//             "#,
//             id,
//             activity_id,
//             frame_id
//         )
//         .execute(&self.pool)
//         .await?;

//         Ok(id)
//     }

//     /// Get all video chunks
//     pub async fn get_all_video_chunks(&self) -> Result<Vec<VideoChunk>, sqlx::Error> {
//         sqlx::query_as!(
//             VideoChunk,
//             r#"
//             SELECT id, file_path
//             FROM video_chunk
//             "#
//         )
//         .fetch_all(&self.pool)
//         .await
//     }

//     /// Get active activities (not ended)
//     pub async fn get_active_activities(&self) -> Result<Vec<Activity>, sqlx::Error> {
//         sqlx::query_as!(
//             Activity,
//             r#"
//             SELECT id, name, app_name, window_name, started_at, ended_at
//             FROM activity
//             WHERE ended_at IS NULL
//             "#
//         )
//         .fetch_all(&self.pool)
//         .await
//     }
// }

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use tempfile::tempdir;

//     #[tokio::test]
//     async fn test_db_creation() {
//         let temp_dir = tempdir().unwrap();
//         let db_path = temp_dir.path().join("test.db");
//         let db_path_str = db_path.to_str().unwrap();

//         let db = PersonalDb::new(db_path_str).await.unwrap();
//         assert!(db_path.exists());

//         // Test inserting an activity
//         let activity_id = db
//             .insert_activity("test activity", "test app", "test window")
//             .await
//             .unwrap();

//         // Verify it was inserted
//         let activity = get_activity(db.pool(), &activity_id).await.unwrap();
//         assert!(activity.is_some());
//         let activity = activity.unwrap();
//         assert_eq!(activity.name, "test activity");
//         assert_eq!(activity.app_name, "test app");
//         assert_eq!(activity.window_name, "test window");
//     }
// }
