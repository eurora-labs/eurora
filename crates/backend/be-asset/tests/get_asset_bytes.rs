//! Integration tests for [`AssetService::get_asset_bytes`].
//!
//! Each test wires the real `DatabaseManager` (via `#[sqlx::test]`) and
//! the real `StorageService` (filesystem backend rooted at a `TempDir`)
//! so the codepath exercised matches production except for the storage
//! medium. Without `DATABASE_URL` the `#[sqlx::test]` macro turns this
//! binary into a no-op so `cargo test` still passes.

use std::sync::Arc;

use be_asset::{AssetError, AssetService, CreateAssetInput};
use be_remote_db::DatabaseManager;
use be_storage::{StorageConfig, StorageService};
use sqlx::PgPool;
use tempfile::TempDir;
use uuid::Uuid;

/// Minimal valid PNG (1×1, single colour) — passes the upload path's
/// magic-byte sniff so we can exercise `create_asset` end-to-end.
const PNG_BYTES: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4,
    0x89, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0xFA, 0xCF, 0x00, 0x00,
    0x00, 0x02, 0x00, 0x01, 0xE5, 0x27, 0xDE, 0xFC, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44,
    0xAE, 0x42, 0x60, 0x82,
];

struct Harness {
    service: AssetService,
    db_pool: PgPool,
    _storage_root: TempDir,
}

impl Harness {
    fn new(pool: PgPool) -> Self {
        let storage_root = tempfile::tempdir().expect("storage tempdir");
        let storage = StorageService::builder()
            .config(StorageConfig::FS {
                root: storage_root.path().to_string_lossy().into_owned(),
            })
            .build()
            .expect("build storage");
        let db = Arc::new(DatabaseManager { pool: pool.clone() });
        Self {
            service: AssetService::new(db, Arc::new(storage)),
            db_pool: pool,
            _storage_root: storage_root,
        }
    }
}

async fn seed_user(pool: &PgPool) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query("INSERT INTO users (id, email) VALUES ($1, $2)")
        .bind(id)
        .bind(format!("user-{id}@test.local"))
        .execute(pool)
        .await
        .expect("seed user");
    id
}

fn png_input() -> CreateAssetInput {
    CreateAssetInput {
        name: "icon.png".into(),
        content: PNG_BYTES.to_vec(),
        mime_type: "image/png".into(),
        metadata: None,
    }
}

#[sqlx::test(migrations = "../be-remote-db/src/migrations")]
async fn round_trips_uploaded_bytes(pool: PgPool) {
    let harness = Harness::new(pool);
    let user_id = seed_user(&harness.db_pool).await;

    let created = harness
        .service
        .create_asset(png_input(), user_id)
        .await
        .expect("create_asset");

    let fetched = harness
        .service
        .get_asset_bytes(created.id, user_id)
        .await
        .expect("get_asset_bytes");

    assert_eq!(fetched.bytes, PNG_BYTES);
    assert_eq!(fetched.mime_type, "image/png");
}

#[sqlx::test(migrations = "../be-remote-db/src/migrations")]
async fn foreign_user_gets_not_found(pool: PgPool) {
    // Non-disclosure invariant: the asset must read as missing — not
    // forbidden — for a non-owner, so attackers can't enumerate ids.
    let harness = Harness::new(pool);
    let owner = seed_user(&harness.db_pool).await;
    let intruder = seed_user(&harness.db_pool).await;

    let created = harness
        .service
        .create_asset(png_input(), owner)
        .await
        .expect("create_asset");

    let err = harness
        .service
        .get_asset_bytes(created.id, intruder)
        .await
        .expect_err("intruder must not read another user's asset");

    assert!(matches!(err, AssetError::NotFound), "got {err:?}");
}

#[sqlx::test(migrations = "../be-remote-db/src/migrations")]
async fn missing_asset_id_returns_not_found(pool: PgPool) {
    let harness = Harness::new(pool);
    let user_id = seed_user(&harness.db_pool).await;

    let err = harness
        .service
        .get_asset_bytes(Uuid::now_v7(), user_id)
        .await
        .expect_err("unknown asset id must error");

    assert!(matches!(err, AssetError::NotFound), "got {err:?}");
}

#[sqlx::test(migrations = "../be-remote-db/src/migrations")]
async fn storage_miss_collapses_to_not_found(pool: PgPool) {
    // The DB row points at storage that no longer holds the blob (object
    // pruned out-of-band). The service must surface this as `NotFound`
    // so HTTP callers render a clean 404 — not a 500 — on a dangling
    // row.
    let harness = Harness::new(pool);
    let user_id = seed_user(&harness.db_pool).await;

    let created = harness
        .service
        .create_asset(png_input(), user_id)
        .await
        .expect("create_asset");

    // Delete the storage blob behind the row's back.
    harness
        .service
        .storage()
        .delete(&storage_uri_for(&harness.db_pool, created.id).await)
        .await
        .expect("storage delete");

    let err = harness
        .service
        .get_asset_bytes(created.id, user_id)
        .await
        .expect_err("expected NotFound for missing blob");

    assert!(matches!(err, AssetError::NotFound), "got {err:?}");
}

async fn storage_uri_for(pool: &PgPool, asset_id: Uuid) -> String {
    sqlx::query_scalar::<_, String>("SELECT storage_uri FROM assets WHERE id = $1")
        .bind(asset_id)
        .fetch_one(pool)
        .await
        .expect("lookup storage_uri")
}
