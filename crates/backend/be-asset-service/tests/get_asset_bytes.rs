//! End-to-end HTTP round-trips for `GET /v1/assets/{id}`.
//!
//! Uses `#[sqlx::test]` to provision a fresh, isolated Postgres database
//! per test (migrations applied automatically) and mounts the real asset
//! router over an ephemeral `TcpListener`. `Claims` are injected
//! directly into request extensions via a layer to bypass the production
//! authz middleware — this crate owns handlers, not authentication.
//!
//! Requires `DATABASE_URL` to be set; without it the macro skips the
//! tests cleanly.

use std::sync::{Arc, Mutex};

use axum::Router;
use axum::extract::Request;
use axum::middleware::Next;
use be_asset::{AssetService, CreateAssetInput};
use be_asset_service::AppState;
use be_auth_core::{Claims, Role};
use be_remote_db::DatabaseManager;
use be_storage::{StorageConfig, StorageService};
use reqwest::StatusCode;
use sqlx::PgPool;
use tempfile::TempDir;
use uuid::Uuid;

/// Minimal valid PNG (1×1) — passes the upload path's magic-byte sniff.
const PNG_BYTES: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4,
    0x89, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0xFA, 0xCF, 0x00, 0x00,
    0x00, 0x02, 0x00, 0x01, 0xE5, 0x27, 0xDE, 0xFC, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44,
    0xAE, 0x42, 0x60, 0x82,
];

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
    service: Arc<AssetService>,
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

async fn spawn_app(pool: PgPool) -> AppHarness {
    let primary = seed_user(&pool).await;
    let other = seed_user(&pool).await;
    let storage_root = tempfile::tempdir().expect("storage tempdir");

    let db = Arc::new(DatabaseManager { pool: pool.clone() });
    let storage = Arc::new(
        StorageService::builder()
            .config(StorageConfig::FS {
                root: storage_root.path().to_string_lossy().into_owned(),
            })
            .build()
            .expect("build storage"),
    );
    let service = Arc::new(AssetService::new(Arc::clone(&db), Arc::clone(&storage)));
    let state = Arc::new(AppState::new(Arc::clone(&service)));

    let active: Arc<Mutex<Option<Uuid>>> = Arc::new(Mutex::new(Some(primary)));
    let active_for_layer = Arc::clone(&active);

    let app: Router = be_asset_service::create_router(state).layer(axum::middleware::from_fn(
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
        service,
        primary,
        other,
        active,
        _storage_root: storage_root,
    }
}

fn png_input() -> CreateAssetInput {
    CreateAssetInput {
        name: "icon.png".into(),
        content: PNG_BYTES.to_vec(),
        mime_type: "image/png".into(),
        metadata: None,
        activity_id: None,
    }
}

#[sqlx::test(migrations = "../be-remote-db/src/migrations")]
async fn returns_bytes_with_immutable_cache(pool: PgPool) {
    let app = spawn_app(pool).await;
    let asset = app
        .service
        .create_asset(png_input(), app.primary)
        .await
        .expect("create_asset");

    let response = reqwest::Client::new()
        .get(app.url(&format!("/v1/assets/{}", asset.id)))
        .send()
        .await
        .expect("GET asset");

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .expect("content-type")
            .to_str()
            .unwrap(),
        "image/png",
    );
    assert_eq!(
        response
            .headers()
            .get(reqwest::header::CACHE_CONTROL)
            .expect("cache-control")
            .to_str()
            .unwrap(),
        "private, max-age=31536000, immutable",
    );
    let body = response.bytes().await.expect("body bytes");
    assert_eq!(body.as_ref(), PNG_BYTES);
}

#[sqlx::test(migrations = "../be-remote-db/src/migrations")]
async fn foreign_user_gets_404(pool: PgPool) {
    let app = spawn_app(pool).await;
    let asset = app
        .service
        .create_asset(png_input(), app.primary)
        .await
        .expect("create_asset");

    app.act_as(app.other);

    let response = reqwest::Client::new()
        .get(app.url(&format!("/v1/assets/{}", asset.id)))
        .send()
        .await
        .expect("GET asset");

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[sqlx::test(migrations = "../be-remote-db/src/migrations")]
async fn missing_asset_returns_404(pool: PgPool) {
    let app = spawn_app(pool).await;

    let response = reqwest::Client::new()
        .get(app.url(&format!("/v1/assets/{}", Uuid::now_v7())))
        .send()
        .await
        .expect("GET asset");

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[sqlx::test(migrations = "../be-remote-db/src/migrations")]
async fn unauthenticated_returns_401(pool: PgPool) {
    let app = spawn_app(pool).await;
    let asset = app
        .service
        .create_asset(png_input(), app.primary)
        .await
        .expect("create_asset");

    app.act_anonymously();

    let response = reqwest::Client::new()
        .get(app.url(&format!("/v1/assets/{}", asset.id)))
        .send()
        .await
        .expect("GET asset");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
