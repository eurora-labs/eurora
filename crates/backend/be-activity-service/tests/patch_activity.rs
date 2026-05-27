//! End-to-end HTTP round-trips for the `/activity-sessions/*` and
//! `/activities*` endpoints.
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

use std::sync::{Arc, Mutex};

use activity_core::{
    ActivityErrorResponse, ActivityInsert, InsertActivitySessionRequest,
    InsertActivitySessionResponse, ListActivitiesResponse, UpdateActivitySessionRequest,
    UpdateActivitySessionResponse,
};
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
    /// Tests are scoped to this user by default; held so individual
    /// tests can re-activate it after switching identity via [`act_as`].
    _primary: Uuid,
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
        _primary: primary,
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

fn insert_body(identity_key: &str, display_name: &str) -> InsertActivitySessionRequest {
    InsertActivitySessionRequest {
        session_id: None,
        activity: ActivityInsert {
            identity_key: identity_key.to_string(),
            display_name: display_name.to_string(),
            icon_png_base64: None,
        },
        process_name: "chrome".to_string(),
        process_id: Some(42),
        window_title: Some("Watching a video".to_string()),
        url: Some("https://youtube.com/watch?v=abc".to_string()),
        started_at: fixed_started_at(),
        ended_at: None,
    }
}

async fn post_session(
    app: &AppHarness,
    body: &InsertActivitySessionRequest,
) -> InsertActivitySessionResponse {
    let response = reqwest::Client::new()
        .post(app.url("/activity-sessions"))
        .json(body)
        .send()
        .await
        .expect("POST");
    assert_eq!(response.status(), StatusCode::OK);
    response.json().await.expect("decode")
}

#[sqlx::test(migrations = "../be-remote-db/src/migrations")]
async fn post_creates_parent_and_session(pool: PgPool) {
    let app = spawn_app(pool).await;
    let body = insert_body("youtube", "Youtube");
    let resp = post_session(&app, &body).await;

    assert_eq!(resp.activity.identity_key, "youtube");
    assert_eq!(resp.activity.display_name, "Youtube");
    assert_eq!(resp.session.activity_id, resp.activity.id);
    assert_eq!(
        resp.session.url.as_deref(),
        Some("https://youtube.com/watch?v=abc")
    );
    assert!(resp.session.ended_at.is_none());
}

#[sqlx::test(migrations = "../be-remote-db/src/migrations")]
async fn post_with_same_identity_reuses_parent(pool: PgPool) {
    let app = spawn_app(pool).await;
    let first = post_session(&app, &insert_body("youtube", "Youtube")).await;
    let second = post_session(&app, &insert_body("youtube", "Youtube")).await;

    assert_eq!(
        first.activity.id, second.activity.id,
        "parent id must be stable across visits"
    );
    assert_ne!(
        first.session.id, second.session.id,
        "each visit gets its own session"
    );
}

#[sqlx::test(migrations = "../be-remote-db/src/migrations")]
async fn post_display_name_is_set_once(pool: PgPool) {
    let app = spawn_app(pool).await;
    post_session(&app, &insert_body("youtube", "Youtube")).await;
    let second = post_session(&app, &insert_body("youtube", "Something Else")).await;

    // Server preserves the original display_name — a future rename
    // endpoint is the only thing that mutates it.
    assert_eq!(second.activity.display_name, "Youtube");
}

#[sqlx::test(migrations = "../be-remote-db/src/migrations")]
async fn post_closes_prior_live_session_for_same_activity(pool: PgPool) {
    let app = spawn_app(pool).await;
    let first = post_session(&app, &insert_body("youtube", "Youtube")).await;
    let _second = post_session(&app, &insert_body("youtube", "Youtube")).await;

    let prior_ended_at: Option<DateTime<Utc>> =
        sqlx::query_scalar("SELECT ended_at FROM activity_sessions WHERE id = $1")
            .bind(first.session.id)
            .fetch_one(&app.pool)
            .await
            .expect("fetch prior session");

    assert!(
        prior_ended_at.is_some(),
        "prior live session must be auto-closed when a new one opens",
    );
}

#[sqlx::test(migrations = "../be-remote-db/src/migrations")]
async fn patch_sets_ended_at_and_bumps_parent(pool: PgPool) {
    let app = spawn_app(pool).await;
    let inserted = post_session(&app, &insert_body("youtube", "Youtube")).await;

    let body = UpdateActivitySessionRequest {
        window_title: None,
        url: None,
        ended_at: Some(fixed_ended_at()),
    };

    let response = reqwest::Client::new()
        .patch(app.url(&format!("/activity-sessions/{}", inserted.session.id)))
        .json(&body)
        .send()
        .await
        .expect("PATCH");
    assert_eq!(response.status(), StatusCode::OK);
    let parsed: UpdateActivitySessionResponse = response.json().await.expect("parse body");
    assert_eq!(parsed.session.id, inserted.session.id);
    assert_eq!(parsed.session.ended_at, Some(fixed_ended_at()));

    let last_used_at: DateTime<Utc> =
        sqlx::query_scalar("SELECT last_used_at FROM activities WHERE id = $1")
            .bind(inserted.activity.id)
            .fetch_one(&app.pool)
            .await
            .expect("fetch parent");
    assert!(
        last_used_at > inserted.activity.last_used_at,
        "closing PATCH must advance the parent's last_used_at",
    );
}

#[sqlx::test(migrations = "../be-remote-db/src/migrations")]
async fn patch_updates_window_title_only(pool: PgPool) {
    let app = spawn_app(pool).await;
    let inserted = post_session(&app, &insert_body("youtube", "Youtube")).await;

    let body = UpdateActivitySessionRequest {
        window_title: Some("Refined Title".to_string()),
        url: None,
        ended_at: None,
    };

    let response = reqwest::Client::new()
        .patch(app.url(&format!("/activity-sessions/{}", inserted.session.id)))
        .json(&body)
        .send()
        .await
        .expect("PATCH");
    assert_eq!(response.status(), StatusCode::OK);
    let parsed: UpdateActivitySessionResponse = response.json().await.expect("parse body");
    assert_eq!(
        parsed.session.window_title.as_deref(),
        Some("Refined Title")
    );
    assert!(parsed.session.ended_at.is_none());
}

#[sqlx::test(migrations = "../be-remote-db/src/migrations")]
async fn patch_rejects_empty_body(pool: PgPool) {
    let app = spawn_app(pool).await;
    let inserted = post_session(&app, &insert_body("youtube", "Youtube")).await;

    let body = UpdateActivitySessionRequest::default();
    let response = reqwest::Client::new()
        .patch(app.url(&format!("/activity-sessions/{}", inserted.session.id)))
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
    let inserted = post_session(&app, &insert_body("youtube", "Youtube")).await;

    app.act_as(app.other);

    let body = UpdateActivitySessionRequest {
        window_title: None,
        url: None,
        ended_at: Some(fixed_ended_at()),
    };
    let response = reqwest::Client::new()
        .patch(app.url(&format!("/activity-sessions/{}", inserted.session.id)))
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
    let inserted = post_session(&app, &insert_body("youtube", "Youtube")).await;

    app.act_anonymously();

    let body = UpdateActivitySessionRequest {
        window_title: None,
        url: None,
        ended_at: Some(fixed_ended_at()),
    };
    let response = reqwest::Client::new()
        .patch(app.url(&format!("/activity-sessions/{}", inserted.session.id)))
        .json(&body)
        .send()
        .await
        .expect("PATCH");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrations = "../be-remote-db/src/migrations")]
async fn list_activities_returns_parents_with_latest_session(pool: PgPool) {
    let app = spawn_app(pool).await;
    let youtube = post_session(&app, &insert_body("youtube", "Youtube")).await;
    let code = post_session(
        &app,
        &InsertActivitySessionRequest {
            session_id: None,
            activity: ActivityInsert {
                identity_key: "code".into(),
                display_name: "Code".into(),
                icon_png_base64: None,
            },
            process_name: "code".into(),
            process_id: Some(99),
            window_title: Some("main.rs".into()),
            url: None,
            started_at: fixed_started_at() + chrono::Duration::seconds(1),
            ended_at: None,
        },
    )
    .await;

    let response = reqwest::Client::new()
        .get(app.url("/activities"))
        .send()
        .await
        .expect("GET");
    assert_eq!(response.status(), StatusCode::OK);

    let parsed: ListActivitiesResponse = response.json().await.expect("decode");
    assert_eq!(parsed.activities.len(), 2);
    // Most recently used (Code) sorts first.
    assert_eq!(parsed.activities[0].activity.id, code.activity.id);
    assert_eq!(parsed.activities[1].activity.id, youtube.activity.id);
    assert_eq!(
        parsed.activities[0].latest_session.as_ref().map(|s| s.id),
        Some(code.session.id),
    );
}
