//! End-to-end HTTP round-trips for the settings service.
//!
//! These tests use `#[sqlx::test]`, which provisions a fresh, isolated
//! Postgres database per test and applies the `be-remote-db` migrations
//! before handing the test body a `PgPool`. The full Axum router is
//! mounted over an ephemeral `TcpListener` so requests exercise the real
//! extractor / handler / response pipeline. `Claims` are injected
//! directly into request extensions, bypassing the production
//! `authz_middleware`: this crate is a *handler* crate and is not
//! responsible for authentication.
//!
//! Running them requires `DATABASE_URL` to be set (e.g.
//! `postgresql://postgres:postgres@localhost:5434/eurora` against the
//! project's `docker-compose.yml`). Without it `#[sqlx::test]` will
//! panic — run just the lib + envelope tests with
//! `cargo test -p be-settings-service --lib --test error_envelope`
//! when no database is available.

use std::sync::{Arc, Mutex};

use axum::Router;
use axum::extract::Request;
use axum::middleware::Next;
use be_auth_core::{Claims, Role};
use be_remote_db::DatabaseManager;
use be_settings_service::{AppState, create_router};
use chrono::{DateTime, Utc};
use reqwest::StatusCode;
use serde_json::{Value, json};
use settings_core::{
    CURRENT_SCHEMA_VERSION, CloudSettings, GetSettingsResponse, PutSettingsAcceptedResponse,
    PutSettingsConflictResponse, PutSettingsRequest, ThemePreference,
};
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
    other: Uuid,
    active: Arc<Mutex<Uuid>>,
}

impl AppHarness {
    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    fn act_as(&self, user_id: Uuid) {
        *self.active.lock().expect("mutex not poisoned") = user_id;
    }
}

/// Spin up the settings router on an ephemeral port. Seeds two users so
/// cross-user isolation tests have a second identity to switch to.
async fn spawn_app(pool: PgPool) -> AppHarness {
    let primary = seed_user(&pool).await;
    let other = seed_user(&pool).await;
    let db = Arc::new(DatabaseManager { pool });
    let state = Arc::new(AppState::new(db));

    let active = Arc::new(Mutex::new(primary));
    let active_for_layer = active.clone();

    let app: Router = create_router(state).layer(axum::middleware::from_fn(
        move |mut req: Request, next: Next| {
            let user_id = *active_for_layer.lock().expect("mutex not poisoned");
            async move {
                req.extensions_mut().insert(claims_for(user_id));
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
        other,
        active,
    }
}

/// Build a `CloudSettings` blob with `theme` overridden and serialize
/// to the opaque `serde_json::Value` shape the wire expects. Field-level
/// invariants are carried by the field types (`InterfaceScale`,
/// `TextScale`), so no separate sanitize pass is required.
fn sample_settings(theme: ThemePreference) -> Value {
    let mut s = CloudSettings::default();
    s.shared.theme = theme;
    serde_json::to_value(s).expect("serialize sample settings")
}

fn put_request(settings: Value, base_updated_at: Option<DateTime<Utc>>) -> PutSettingsRequest {
    PutSettingsRequest {
        schema_version: CURRENT_SCHEMA_VERSION,
        settings,
        base_updated_at,
    }
}

// ── tests ─────────────────────────────────────────────────────────────

#[sqlx::test(migrations = "../be-remote-db/src/migrations")]
async fn get_returns_404_for_first_run(pool: PgPool) {
    let app = spawn_app(pool).await;
    let res = reqwest::get(app.url("/settings")).await.expect("GET");
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let body: Value = res.json().await.expect("envelope");
    assert_eq!(body["error"], "not_found");
}

#[sqlx::test(migrations = "../be-remote-db/src/migrations")]
async fn put_then_get_round_trip(pool: PgPool) {
    let app = spawn_app(pool).await;
    let client = reqwest::Client::new();

    let blob = sample_settings(ThemePreference::Dark);
    let put = client
        .put(app.url("/settings"))
        .json(&put_request(blob.clone(), None))
        .send()
        .await
        .expect("PUT");
    assert_eq!(put.status(), StatusCode::OK);
    let accepted: PutSettingsAcceptedResponse = put.json().await.expect("accepted body");
    assert_eq!(accepted.schema_version, CURRENT_SCHEMA_VERSION);

    let get = reqwest::get(app.url("/settings")).await.expect("GET");
    assert_eq!(get.status(), StatusCode::OK);
    let resp: GetSettingsResponse = get.json().await.expect("get body");
    assert_eq!(resp.schema_version, CURRENT_SCHEMA_VERSION);
    assert_eq!(resp.updated_at, accepted.updated_at);
    // Server stores the blob verbatim — bytes back equal bytes in.
    assert_eq!(resp.settings, blob);
    // And the client can still parse it through CloudSettings.
    let parsed: CloudSettings = serde_json::from_value(resp.settings).expect("parse blob");
    assert_eq!(parsed.shared.theme, ThemePreference::Dark);
}

#[sqlx::test(migrations = "../be-remote-db/src/migrations")]
async fn put_with_stale_base_returns_409_with_current(pool: PgPool) {
    let app = spawn_app(pool).await;
    let client = reqwest::Client::new();

    let initial_blob = sample_settings(ThemePreference::Light);
    let first: PutSettingsAcceptedResponse = client
        .put(app.url("/settings"))
        .json(&put_request(initial_blob.clone(), None))
        .send()
        .await
        .expect("PUT 1")
        .json()
        .await
        .expect("accept 1");

    // Deliberately stale base: UNIX_EPOCH can't possibly match the row's
    // real updated_at.
    let stale_base: DateTime<Utc> = DateTime::<Utc>::UNIX_EPOCH;
    assert_ne!(stale_base, first.updated_at);

    let second = client
        .put(app.url("/settings"))
        .json(&put_request(
            sample_settings(ThemePreference::Dark),
            Some(stale_base),
        ))
        .send()
        .await
        .expect("PUT 2");
    assert_eq!(second.status(), StatusCode::CONFLICT);
    let conflict: PutSettingsConflictResponse = second.json().await.expect("conflict body");
    assert_eq!(conflict.schema_version, CURRENT_SCHEMA_VERSION);
    assert_eq!(conflict.updated_at, first.updated_at);
    assert_eq!(conflict.current, initial_blob);
}

#[sqlx::test(migrations = "../be-remote-db/src/migrations")]
async fn put_recovers_after_delete(pool: PgPool) {
    let app = spawn_app(pool).await;
    let client = reqwest::Client::new();

    let first: PutSettingsAcceptedResponse = client
        .put(app.url("/settings"))
        .json(&put_request(sample_settings(ThemePreference::Dark), None))
        .send()
        .await
        .expect("PUT 1")
        .json()
        .await
        .expect("accept 1");

    let del = client
        .delete(app.url("/settings"))
        .send()
        .await
        .expect("DELETE");
    assert_eq!(del.status(), StatusCode::NO_CONTENT);

    // DB-layer recovery contract: PUT against a deleted row inserts a
    // fresh one (not Conflict), so the client doesn't wedge.
    let recovery_blob = sample_settings(ThemePreference::Light);
    let recovery = client
        .put(app.url("/settings"))
        .json(&put_request(recovery_blob.clone(), Some(first.updated_at)))
        .send()
        .await
        .expect("PUT recovery");
    assert_eq!(recovery.status(), StatusCode::OK);

    let get: GetSettingsResponse = reqwest::get(app.url("/settings"))
        .await
        .expect("GET")
        .json()
        .await
        .expect("get body");
    assert_eq!(get.settings, recovery_blob);
}

#[sqlx::test(migrations = "../be-remote-db/src/migrations")]
async fn delete_is_idempotent_when_empty(pool: PgPool) {
    let app = spawn_app(pool).await;
    let res = reqwest::Client::new()
        .delete(app.url("/settings"))
        .send()
        .await
        .expect("DELETE");
    assert_eq!(res.status(), StatusCode::NO_CONTENT);
}

#[sqlx::test(migrations = "../be-remote-db/src/migrations")]
async fn schema_version_persists_through_round_trip(pool: PgPool) {
    let app = spawn_app(pool).await;
    let client = reqwest::Client::new();

    let put: PutSettingsAcceptedResponse = client
        .put(app.url("/settings"))
        .json(&put_request(json!({}), None))
        .send()
        .await
        .expect("PUT")
        .json()
        .await
        .expect("accept");
    assert_eq!(put.schema_version, CURRENT_SCHEMA_VERSION);

    let get: GetSettingsResponse = reqwest::get(app.url("/settings"))
        .await
        .expect("GET")
        .json()
        .await
        .expect("get");
    assert_eq!(get.schema_version, CURRENT_SCHEMA_VERSION);
}

#[sqlx::test(migrations = "../be-remote-db/src/migrations")]
async fn schema_version_overflowing_i32_is_rejected(pool: PgPool) {
    let app = spawn_app(pool).await;

    let body = json!({
        "schemaVersion": (i32::MAX as u32) + 1,
        "settings": {},
        "baseUpdatedAt": null,
    });

    let res = reqwest::Client::new()
        .put(app.url("/settings"))
        .json(&body)
        .send()
        .await
        .expect("PUT");
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    let env: Value = res.json().await.expect("envelope");
    assert_eq!(env["error"], "invalid_argument");
}

#[sqlx::test(migrations = "../be-remote-db/src/migrations")]
async fn unknown_blob_shape_round_trips_verbatim(pool: PgPool) {
    // The server promises blob opacity: a body that doesn't match the
    // current CloudSettings shape — different keys, unexpected types,
    // a future schema_version — must round-trip byte-for-byte.
    let app = spawn_app(pool).await;
    let client = reqwest::Client::new();

    let arbitrary = json!({
        "future_section": { "knob": 42, "nested": ["a", "b"] },
        "list": [1, 2, 3],
        "scalar": "hello",
    });

    let put: PutSettingsAcceptedResponse = client
        .put(app.url("/settings"))
        .json(&PutSettingsRequest {
            schema_version: 999,
            settings: arbitrary.clone(),
            base_updated_at: None,
        })
        .send()
        .await
        .expect("PUT")
        .json()
        .await
        .expect("accept");
    assert_eq!(put.schema_version, 999);

    let get: GetSettingsResponse = reqwest::get(app.url("/settings"))
        .await
        .expect("GET")
        .json()
        .await
        .expect("get");
    assert_eq!(get.schema_version, 999);
    assert_eq!(get.settings, arbitrary);
}

#[sqlx::test(migrations = "../be-remote-db/src/migrations")]
async fn users_do_not_see_each_others_settings(pool: PgPool) {
    let app = spawn_app(pool).await;
    let client = reqwest::Client::new();

    client
        .put(app.url("/settings"))
        .json(&put_request(sample_settings(ThemePreference::Dark), None))
        .send()
        .await
        .expect("PUT primary");

    app.act_as(app.other);

    let res = reqwest::get(app.url("/settings")).await.expect("GET other");
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[sqlx::test(migrations = "../be-remote-db/src/migrations")]
async fn concurrent_puts_produce_one_winner_and_one_conflict(pool: PgPool) {
    let app = spawn_app(pool).await;
    let client = reqwest::Client::new();

    let first: PutSettingsAcceptedResponse = client
        .put(app.url("/settings"))
        .json(&put_request(sample_settings(ThemePreference::System), None))
        .send()
        .await
        .expect("PUT seed")
        .json()
        .await
        .expect("accept seed");
    let base = first.updated_at;

    let url = app.url("/settings");
    let a = {
        let client = client.clone();
        let url = url.clone();
        tokio::spawn(async move {
            client
                .put(url)
                .json(&put_request(
                    sample_settings(ThemePreference::Light),
                    Some(base),
                ))
                .send()
                .await
                .expect("PUT a")
        })
    };
    let b = {
        let client = client.clone();
        let url = url.clone();
        tokio::spawn(async move {
            client
                .put(url)
                .json(&put_request(
                    sample_settings(ThemePreference::Dark),
                    Some(base),
                ))
                .send()
                .await
                .expect("PUT b")
        })
    };

    let (ra, rb) = (a.await.unwrap(), b.await.unwrap());
    let mut statuses = [ra.status(), rb.status()];
    statuses.sort_by_key(|s| s.as_u16());
    assert_eq!(
        statuses,
        [StatusCode::OK, StatusCode::CONFLICT],
        "expected exactly one OK and one CONFLICT, got {statuses:?}",
    );
}
