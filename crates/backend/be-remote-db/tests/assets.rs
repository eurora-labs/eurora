//! Integration tests for the assets DB layer.
//!
//! Uses `#[sqlx::test]` like `user_settings.rs`: each test runs against a
//! freshly migrated, isolated database. Without `DATABASE_URL` the macro
//! makes this binary a no-op so `cargo test` still passes in
//! database-free environments.

use be_remote_db::DatabaseManager;
use sqlx::PgPool;
use uuid::Uuid;

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

async fn seed_asset(db: &DatabaseManager, user_id: Uuid) -> Uuid {
    db.create_asset()
        .user_id(user_id)
        .name("icon.png".to_owned())
        .mime_type("image/png".to_owned())
        .size_bytes(42)
        .storage_backend("filesystem".to_owned())
        .storage_uri(format!("file:///tmp/{}.png", Uuid::now_v7()))
        .call()
        .await
        .expect("create_asset")
        .id
}

#[sqlx::test(migrations = "./src/migrations")]
async fn get_asset_for_user_returns_owner_row(pool: PgPool) {
    let db = DatabaseManager { pool };
    let user_id = seed_user(&db.pool).await;
    let asset_id = seed_asset(&db, user_id).await;

    let fetched = db
        .get_asset_for_user()
        .asset_id(asset_id)
        .user_id(user_id)
        .call()
        .await
        .expect("get_asset_for_user");

    assert_eq!(fetched.id, asset_id);
    assert_eq!(fetched.user_id, user_id);
    assert_eq!(fetched.mime_type, "image/png");
}

#[sqlx::test(migrations = "./src/migrations")]
async fn get_asset_for_user_treats_foreign_id_as_not_found(pool: PgPool) {
    // The ownership predicate lives inside the WHERE clause so user A's
    // asset surfaces as `NotFound` (not `Forbidden`) to user B — that
    // prevents probing the existence of another user's asset ids.
    let db = DatabaseManager { pool };
    let owner = seed_user(&db.pool).await;
    let intruder = seed_user(&db.pool).await;
    let asset_id = seed_asset(&db, owner).await;

    let err = db
        .get_asset_for_user()
        .asset_id(asset_id)
        .user_id(intruder)
        .call()
        .await
        .expect_err("foreign user must not read another user's asset");

    assert!(err.is_not_found(), "expected NotFound, got {err:?}");
}

#[sqlx::test(migrations = "./src/migrations")]
async fn get_asset_for_user_returns_not_found_for_missing_asset(pool: PgPool) {
    let db = DatabaseManager { pool };
    let user_id = seed_user(&db.pool).await;

    let err = db
        .get_asset_for_user()
        .asset_id(Uuid::now_v7())
        .user_id(user_id)
        .call()
        .await
        .expect_err("unknown asset id must error");

    assert!(err.is_not_found(), "expected NotFound, got {err:?}");
}
