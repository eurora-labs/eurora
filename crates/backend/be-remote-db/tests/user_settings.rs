//! Integration tests for the `user_settings` persistence layer.
//!
//! These tests use `#[sqlx::test]`, which provisions a fresh, isolated
//! database per test and applies the crate's migrations before handing
//! the test body a `PgPool`. Running them requires `DATABASE_URL` to be
//! set; without it, `cargo test -p be-remote-db` skips this binary
//! cleanly via the macro.

use std::sync::Arc;

use be_remote_db::{DatabaseManager, UpsertOutcome, UserSettingsRow};
use chrono::{DateTime, Utc};
use serde_json::json;
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

fn expect_inserted(outcome: UpsertOutcome) -> UserSettingsRow {
    match outcome {
        UpsertOutcome::Inserted(row) => row,
        other => panic!("expected Inserted, got {other:?}"),
    }
}

fn expect_updated(outcome: UpsertOutcome) -> UserSettingsRow {
    match outcome {
        UpsertOutcome::Updated(row) => row,
        other => panic!("expected Updated, got {other:?}"),
    }
}

fn expect_conflict(outcome: UpsertOutcome) -> UserSettingsRow {
    match outcome {
        UpsertOutcome::Conflict { current } => current,
        other => panic!("expected Conflict, got {other:?}"),
    }
}

async fn upsert(
    db: &DatabaseManager,
    user_id: Uuid,
    schema_version: i32,
    settings: serde_json::Value,
    base_updated_at: Option<DateTime<Utc>>,
) -> UpsertOutcome {
    db.upsert_user_settings()
        .user_id(user_id)
        .schema_version(schema_version)
        .settings(settings)
        .maybe_base_updated_at(base_updated_at)
        .call()
        .await
        .expect("upsert_user_settings")
}

#[sqlx::test(migrations = "./src/migrations")]
async fn get_returns_none_for_unknown_user(pool: PgPool) {
    let db = DatabaseManager { pool };
    let missing = Uuid::now_v7();

    let result = db
        .get_user_settings()
        .user_id(missing)
        .call()
        .await
        .expect("get_user_settings");

    assert!(result.is_none());
}

#[sqlx::test(migrations = "./src/migrations")]
async fn insert_then_read_round_trip(pool: PgPool) {
    let db = DatabaseManager { pool };
    let user_id = seed_user(&db.pool).await;
    let payload = json!({"theme": "dark", "scale": 1.0});

    let inserted = expect_inserted(upsert(&db, user_id, 1, payload.clone(), None).await);
    assert_eq!(inserted.user_id, user_id);
    assert_eq!(inserted.schema_version, 1);
    assert_eq!(inserted.settings, payload);

    let fetched = db
        .get_user_settings()
        .user_id(user_id)
        .call()
        .await
        .expect("get_user_settings")
        .expect("row should exist");

    assert_eq!(fetched.user_id, inserted.user_id);
    assert_eq!(fetched.schema_version, inserted.schema_version);
    assert_eq!(fetched.settings, inserted.settings);
    assert_eq!(fetched.created_at, inserted.created_at);
    assert_eq!(fetched.updated_at, inserted.updated_at);
}

#[sqlx::test(migrations = "./src/migrations")]
async fn upsert_with_matching_base_updates(pool: PgPool) {
    let db = DatabaseManager { pool };
    let user_id = seed_user(&db.pool).await;

    let initial = expect_inserted(upsert(&db, user_id, 1, json!({"v": 0}), None).await);

    let updated =
        expect_updated(upsert(&db, user_id, 1, json!({"v": 1}), Some(initial.updated_at)).await);

    assert_eq!(updated.settings, json!({"v": 1}));
    assert_eq!(updated.created_at, initial.created_at);
    assert!(updated.updated_at >= initial.updated_at);
}

#[sqlx::test(migrations = "./src/migrations")]
async fn upsert_with_no_base_against_existing_row_conflicts(pool: PgPool) {
    let db = DatabaseManager { pool };
    let user_id = seed_user(&db.pool).await;

    let initial = expect_inserted(upsert(&db, user_id, 1, json!({"v": 0}), None).await);

    let current = expect_conflict(upsert(&db, user_id, 1, json!({"v": 99}), None).await);

    assert_eq!(current.user_id, initial.user_id);
    assert_eq!(current.settings, initial.settings);
    assert_eq!(current.updated_at, initial.updated_at);

    let stored = db
        .get_user_settings()
        .user_id(user_id)
        .call()
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.settings, initial.settings);
}

#[sqlx::test(migrations = "./src/migrations")]
async fn upsert_with_stale_base_returns_current(pool: PgPool) {
    let db = DatabaseManager { pool };
    let user_id = seed_user(&db.pool).await;

    let initial = expect_inserted(upsert(&db, user_id, 1, json!({"v": 0}), None).await);
    let winner =
        expect_updated(upsert(&db, user_id, 1, json!({"v": 1}), Some(initial.updated_at)).await);

    let current =
        expect_conflict(upsert(&db, user_id, 1, json!({"v": 2}), Some(initial.updated_at)).await);

    assert_eq!(current.settings, winner.settings);
    assert_eq!(current.updated_at, winner.updated_at);
}

#[sqlx::test(migrations = "./src/migrations")]
async fn upsert_with_base_against_missing_row_inserts(pool: PgPool) {
    let db = DatabaseManager { pool };
    let user_id = seed_user(&db.pool).await;
    let stale_base = Utc::now();

    let inserted = expect_inserted(
        upsert(
            &db,
            user_id,
            1,
            json!({"recovered": true}),
            Some(stale_base),
        )
        .await,
    );

    assert_eq!(inserted.user_id, user_id);
    assert_eq!(inserted.settings, json!({"recovered": true}));
}

#[sqlx::test(migrations = "./src/migrations")]
async fn schema_version_preserved_through_round_trip(pool: PgPool) {
    let db = DatabaseManager { pool };
    let user_id = seed_user(&db.pool).await;

    let inserted = expect_inserted(upsert(&db, user_id, 7, json!({}), None).await);
    assert_eq!(inserted.schema_version, 7);

    let updated =
        expect_updated(upsert(&db, user_id, 42, json!({}), Some(inserted.updated_at)).await);
    assert_eq!(updated.schema_version, 42);

    let fetched = db
        .get_user_settings()
        .user_id(user_id)
        .call()
        .await
        .unwrap()
        .unwrap();
    assert_eq!(fetched.schema_version, 42);
}

#[sqlx::test(migrations = "./src/migrations")]
async fn delete_is_idempotent(pool: PgPool) {
    let db = DatabaseManager { pool };
    let user_id = seed_user(&db.pool).await;

    db.delete_user_settings()
        .user_id(user_id)
        .call()
        .await
        .expect("delete on empty row");

    expect_inserted(upsert(&db, user_id, 1, json!({"v": 0}), None).await);

    db.delete_user_settings()
        .user_id(user_id)
        .call()
        .await
        .expect("delete existing row");

    let after = db
        .get_user_settings()
        .user_id(user_id)
        .call()
        .await
        .unwrap();
    assert!(after.is_none());

    db.delete_user_settings()
        .user_id(user_id)
        .call()
        .await
        .expect("second delete is a no-op");
}

#[sqlx::test(migrations = "./src/migrations")]
async fn delete_cascades_from_users(pool: PgPool) {
    let db = DatabaseManager { pool };
    let user_id = seed_user(&db.pool).await;
    expect_inserted(upsert(&db, user_id, 1, json!({"v": 0}), None).await);

    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(user_id)
        .execute(&db.pool)
        .await
        .expect("delete user");

    let after = db
        .get_user_settings()
        .user_id(user_id)
        .call()
        .await
        .unwrap();
    assert!(after.is_none());
}

#[sqlx::test(migrations = "./src/migrations")]
async fn concurrent_upserts_produce_one_winner_and_one_conflict(pool: PgPool) {
    let db = Arc::new(DatabaseManager { pool });
    let user_id = seed_user(&db.pool).await;
    let initial = expect_inserted(upsert(&db, user_id, 1, json!({"v": 0}), None).await);
    let base = initial.updated_at;

    let db_a = Arc::clone(&db);
    let db_b = Arc::clone(&db);

    let (a, b) = tokio::join!(
        async move {
            db_a.upsert_user_settings()
                .user_id(user_id)
                .schema_version(1)
                .settings(json!({"writer": "a"}))
                .base_updated_at(base)
                .call()
                .await
                .unwrap()
        },
        async move {
            db_b.upsert_user_settings()
                .user_id(user_id)
                .schema_version(1)
                .settings(json!({"writer": "b"}))
                .base_updated_at(base)
                .call()
                .await
                .unwrap()
        },
    );

    let outcomes = [a, b];
    let updates: Vec<_> = outcomes
        .iter()
        .filter(|o| matches!(o, UpsertOutcome::Updated(_)))
        .collect();
    let conflicts: Vec<_> = outcomes
        .iter()
        .filter(|o| matches!(o, UpsertOutcome::Conflict { .. }))
        .collect();

    assert_eq!(
        updates.len(),
        1,
        "exactly one writer should win: {outcomes:?}"
    );
    assert_eq!(
        conflicts.len(),
        1,
        "exactly one writer should see Conflict: {outcomes:?}"
    );

    let winning_row = match updates[0] {
        UpsertOutcome::Updated(row) => row.clone(),
        _ => unreachable!(),
    };
    let conflict_current = match conflicts[0] {
        UpsertOutcome::Conflict { current } => current.clone(),
        _ => unreachable!(),
    };

    assert_eq!(conflict_current.updated_at, winning_row.updated_at);
    assert_eq!(conflict_current.settings, winning_row.settings);

    let fetched = db
        .get_user_settings()
        .user_id(user_id)
        .call()
        .await
        .unwrap()
        .unwrap();
    assert_eq!(fetched.settings, winning_row.settings);
}
