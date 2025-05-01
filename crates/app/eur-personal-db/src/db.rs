use chrono::{DateTime, Utc};
use image::DynamicImage;
use libsqlite3_sys::sqlite3_auto_extension;
use sqlite_vec::sqlite3_vec_init;
use sqlx::Column;
use sqlx::Error as SqlxError;
use sqlx::Row;
use sqlx::TypeInfo;
use sqlx::ValueRef;
use sqlx::migrate::MigrateDatabase;
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, error, warn};

use std::collections::BTreeMap;

use zerocopy::FromBytes;

use futures::future::try_join_all;

pub struct DatabaseManager {
    pub pool: SqlitePool,
}

impl DatabaseManager {
    pub async fn new(database_path: &str) -> Result<Self, sqlx::Error> {
        debug!(
            "Initializing DatabaseManager with database path: {}",
            database_path
        );
        let connection_string = format!("sqlite:{}", database_path);

        unsafe {
            sqlite3_auto_extension(Some(
                std::mem::transmute::<*const (), unsafe extern "C" fn()>(
                    sqlite3_vec_init as *const (),
                ),
            ));
        }

        // Create the database if it doesn't exist
        if !sqlx::Sqlite::database_exists(&connection_string).await? {
            sqlx::Sqlite::create_database(&connection_string).await?;
        }

        let pool = SqlitePoolOptions::new()
            .max_connections(50)
            .min_connections(3) // Minimum number of idle connections
            .acquire_timeout(Duration::from_secs(10))
            .connect(&connection_string)
            .await?;

        // Enable WAL mode
        sqlx::query("PRAGMA journal_mode = WAL;")
            .execute(&pool)
            .await?;

        // Enable SQLite's query result caching
        // PRAGMA cache_size = -2000; -- Set cache size to 2MB
        // PRAGMA temp_store = MEMORY; -- Store temporary tables and indices in memory
        sqlx::query("PRAGMA cache_size = -2000;")
            .execute(&pool)
            .await?;
        sqlx::query("PRAGMA temp_store = MEMORY;")
            .execute(&pool)
            .await?;

        let db_manager = DatabaseManager { pool };

        // Run migrations after establishing the connection
        Self::run_migrations(&db_manager.pool).await?;

        Ok(db_manager)
    }

    async fn run_migrations(pool: &SqlitePool) -> Result<(), sqlx::Error> {
        let mut migrator = sqlx::migrate!("./src/migrations");
        migrator.set_ignore_missing(true);
        match migrator.run(pool).await {
            Ok(_) => Ok(()),
            Err(e) => Err(e.into()),
        }
    }
}
