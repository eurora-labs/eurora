//! End-to-end HTTP round-trips for `PATCH /activities/{id}`.
//!
//! Uses `#[sqlx::test]` to provision a fresh, isolated Postgres database
//! per test (migrations applied automatically) and mounts the real
//! activity router over an ephemeral `TcpListener`. `Claims` are
//! injected directly into request extensions to bypass the production
//! authz middleware — this crate owns handlers, not authentication.
//!
//! Requires `DATABASE_URL` to be set. Run the wire-type round-trip
//! tests alone with
//! `cargo test -p be-activity-service --lib --test auth_extractor`
//! when no database is available.
//!
//! The PATCH handler is the heartbeat / end-of-activity path: the
//! desktop collector ratchets `ended_at` forward every 30 s while an
//! activity is live, then writes the precise `ended_at` once on
//! transition or graceful shutdown. These tests cover the cross-user
//! safety net, partial-field semantics, and the empty-body guard.

use std::sync::{Arc, Mutex};

use activity_core::{ActivityErrorResponse, UpdateActivityRequest, UpdateActivityResponse};
use axum::Router;
use axum::extract::Request;
use axum::middleware::Next;
use be_activity_service::AppState;
use be_asset::AssetService;
use be_auth_core::{Claims, Role};
use be_remote_db::DatabaseManager;
use be_storage::{StorageConfig, StorageService};
use chrono::{DateTime, TimeZone, Utc};
use reqwest::StatusCode;
use sqlx::PgPool;
use tempfile::TempDir;
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

async fn seed_activity(
    pool: &PgPool,
    user_id: Uuid,
    started_at: DateTime<Utc>,
    ended_at: Option<DateTime<Utc>>,
) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO activities (id, user_id, name, process_name, window_title, started_at, ended_at) \
         VALUES ($1, $2, 'seed', 'seed-proc', 'seed-title', $3, $4)",
    )
    .bind(id)
    .bind(user_id)
    .bind(started_at)
    .bind(ended_at)
    .execute(pool)
    .await
    .expect("seed activity");
    id
}

fn claims_for(user_id: Uuid) -> Claims {
    Claims {
        sub: user_id.to_string(),
        email: format!("user-{user_id}@test.local"),
        display_name: None,
        iat: 0,
        exp: i64::MAX,
        token_type: "access".into(),
        role: Role::Free,
        aud: "eurora".into(),
        email_verified: true,
        jti: Uuid::now_v7().to_string(),
    }
}

struct AppHarness {
    base_url: String,
    pool: PgPool,
    primary: Uuid,
    other: Uuid,
    active: Arc<Mutex<Option<Uuid>>>,
    _storage_root: TempDir,
}

impl AppHarness {
    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    fn act_as(&self, user_id: Uuid) {
        *self.active.lock().expect("mutex not poisoned") = Some(user_id);
    }

    fn act_anonymously(&self) {
        *self.active.lock().expect("mutex not poisoned") = None;
    }
}

/// Stand up the activity router on an ephemeral port with the real
/// asset service wired through filesystem storage rooted at a tempdir.
/// Activity PATCH does not touch assets, but `AppState::new` still
/// requires an `AssetService` instance.
async fn spawn_app(pool: PgPool) -> AppHarness {
    let primary = seed_user(&pool).await;
    let other = seed_user(&pool).await;
    let storage_root = tempfile::tempdir().expect("tempdir for storage root");

    let db = Arc::new(DatabaseManager { pool: pool.clone() });
    let storage_config = StorageConfig::FS {
        root: storage_root.path().to_string_lossy().into_owned(),
    };
    let storage = Arc::new(
        StorageService::builder()
            .config(storage_config)
            .build()
            .expect("build storage service"),
    );
    let asset_service = Arc::new(AssetService::new(Arc::clone(&db), storage));
    let state = Arc::new(AppState::new(db, asset_service));

    let active: Arc<Mutex<Option<Uuid>>> = Arc::new(Mutex::new(Some(primary)));
    let active_for_layer = Arc::clone(&active);

    let app: Router = be_activity_service::create_router(state).layer(axum::middleware::from_fn(
        move |mut req: Request, next: Next| {
            let user_id = *active_for_layer.lock().expect("mutex not poisoned");
            async move {
                if let Some(uid) = user_id {
                    req.extensions_mut().insert(claims_for(uid));
                }
                next.run(req).await
            }
        },
    ));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind ephemeral");
    let addr = listener.local_addr().expect("local_addr");
    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("serve");
    });

    AppHarness {
        base_url: format!("http://{addr}"),
        pool,
        primary,
        other,
        active,
        _storage_root: storage_root,
    }
}

fn fixed_started_at() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 5, 16, 12, 0, 0).unwrap()
}

fn fixed_ended_at() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 5, 16, 12, 30, 0).unwrap()
}

#[sqlx::test(migrations = "../be-remote-db/src/migrations")]
async fn patch_sets_ended_at(pool: PgPool) {
    let app = spawn_app(pool).await;
    let activity_id = seed_activity(&app.pool, app.primary, fixed_started_at(), None).await;

    let body = UpdateActivityRequest {
        name: None,
        window_title: None,
        ended_at: Some(fixed_ended_at()),
    };

    let response = reqwest::Client::new()
        .patch(app.url(&format!("/activities/{activity_id}")))
        .json(&body)
        .send()
        .await
        .expect("PATCH");

    assert_eq!(response.status(), StatusCode::OK);
    let parsed: UpdateActivityResponse = response.json().await.expect("parse body");
    assert_eq!(parsed.activity.id, activity_id);
    assert_eq!(parsed.activity.ended_at, Some(fixed_ended_at()));
    // Unset fields are preserved (COALESCE on the SQL side).
    assert_eq!(parsed.activity.name, "seed");
    assert_eq!(parsed.activity.window_title, "seed-title");
}

#[sqlx::test(migrations = "../be-remote-db/src/migrations")]
async fn patch_updates_window_title_only(pool: PgPool) {
    let app = spawn_app(pool).await;
    let activity_id = seed_activity(&app.pool, app.primary, fixed_started_at(), None).await;

    let body = UpdateActivityRequest {
        name: None,
        window_title: Some("Refined Title".to_string()),
        ended_at: None,
    };

    let response = reqwest::Client::new()
        .patch(app.url(&format!("/activities/{activity_id}")))
        .json(&body)
        .send()
        .await
        .expect("PATCH");

    assert_eq!(response.status(), StatusCode::OK);
    let parsed: UpdateActivityResponse = response.json().await.expect("parse body");
    assert_eq!(parsed.activity.window_title, "Refined Title");
    assert!(parsed.activity.ended_at.is_none());
}

#[sqlx::test(migrations = "../be-remote-db/src/migrations")]
async fn heartbeat_then_final_end_overrides(pool: PgPool) {
    let app = spawn_app(pool).await;
    let activity_id = seed_activity(&app.pool, app.primary, fixed_started_at(), None).await;

    // Heartbeat ratchet: the collector pushes a coarse `ended_at` every
    // 30 s while the activity is live.
    let heartbeat_value = fixed_started_at() + chrono::Duration::seconds(30);
    let heartbeat_body = UpdateActivityRequest {
        name: None,
        window_title: None,
        ended_at: Some(heartbeat_value),
    };
    let response = reqwest::Client::new()
        .patch(app.url(&format!("/activities/{activity_id}")))
        .json(&heartbeat_body)
        .send()
        .await
        .expect("PATCH heartbeat");
    assert_eq!(response.status(), StatusCode::OK);

    // Real transition / shutdown: the precise `ended_at` overwrites the
    // last heartbeat value.
    let final_body = UpdateActivityRequest {
        name: None,
        window_title: None,
        ended_at: Some(fixed_ended_at()),
    };
    let response = reqwest::Client::new()
        .patch(app.url(&format!("/activities/{activity_id}")))
        .json(&final_body)
        .send()
        .await
        .expect("PATCH final");
    assert_eq!(response.status(), StatusCode::OK);
    let parsed: UpdateActivityResponse = response.json().await.expect("parse body");
    assert_eq!(parsed.activity.ended_at, Some(fixed_ended_at()));
}

#[sqlx::test(migrations = "../be-remote-db/src/migrations")]
async fn patch_rejects_empty_body(pool: PgPool) {
    let app = spawn_app(pool).await;
    let activity_id = seed_activity(&app.pool, app.primary, fixed_started_at(), None).await;

    let body = UpdateActivityRequest::default();

    let response = reqwest::Client::new()
        .patch(app.url(&format!("/activities/{activity_id}")))
        .json(&body)
        .send()
        .await
        .expect("PATCH");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let envelope: ActivityErrorResponse = response.json().await.expect("envelope");
    assert_eq!(envelope.error, "invalid_argument");
}

#[sqlx::test(migrations = "../be-remote-db/src/migrations")]
async fn patch_cross_user_returns_404(pool: PgPool) {
    let app = spawn_app(pool).await;
    let activity_id = seed_activity(&app.pool, app.primary, fixed_started_at(), None).await;

    // Switch the injected identity to a different real user. The row
    // exists but doesn't belong to them, so the WHERE clause on
    // `user_id` rejects it as "not found" rather than leaking existence
    // via 403.
    app.act_as(app.other);

    let body = UpdateActivityRequest {
        name: None,
        window_title: None,
        ended_at: Some(fixed_ended_at()),
    };
    let response = reqwest::Client::new()
        .patch(app.url(&format!("/activities/{activity_id}")))
        .json(&body)
        .send()
        .await
        .expect("PATCH");

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let envelope: ActivityErrorResponse = response.json().await.expect("envelope");
    assert_eq!(envelope.error, "not_found");
}

#[sqlx::test(migrations = "../be-remote-db/src/migrations")]
async fn patch_unauthenticated_returns_401(pool: PgPool) {
    let app = spawn_app(pool).await;
    let activity_id = seed_activity(&app.pool, app.primary, fixed_started_at(), None).await;

    app.act_anonymously();

    let body = UpdateActivityRequest {
        name: None,
        window_title: None,
        ended_at: Some(fixed_ended_at()),
    };
    let response = reqwest::Client::new()
        .patch(app.url(&format!("/activities/{activity_id}")))
        .json(&body)
        .send()
        .await
        .expect("PATCH");

    // When the authz middleware is absent, the `AuthUser` extractor's
    // own `MissingClaims` rejection short-circuits before the handler
    // runs. That rejection renders a plain-text body (defence-in-depth
    // for a misconfigured router), not the JSON envelope — assert on
    // the status only.
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
