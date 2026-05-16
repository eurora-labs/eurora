//! End-to-end tests for the cloud-settings sync engine.
//!
//! Each test stands up a `wiremock::MockServer`, points a transport at
//! it, and drives the engine through `pull_now` / `request_push` /
//! `start`. The transport substituted in tests is not the production
//! [`euro_settings::ReqwestTransport`] — that one demands a working
//! keyring-backed [`euro_auth::AuthManager`], which would force tests
//! to provision secrets out-of-band. Instead the tests use
//! [`HttpTestTransport`], which speaks the same HTTP wire format
//! against the mock server without auth headers. The auth-classifying
//! branches of the engine are covered by unit tests inside
//! `src/sync/error.rs`.

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, TimeZone, Utc};
use euro_auth::{AuthEvent, Claims, Role};
use euro_settings::{
    AuthIdentity, BackoffConfig, CloudSettingsCache, PullOutcome, PushOutcome, SettingsState,
    SettingsTransport, SyncEngine, SyncError, SyncResult, SyncStatus,
};
use reqwest::StatusCode;
use settings_core::{
    CURRENT_SCHEMA_VERSION, CloudSettings, GetSettingsResponse, PutSettingsAcceptedResponse,
    PutSettingsConflictResponse, PutSettingsRequest, ThemePreference,
};
use tempfile::TempDir;
use tokio::sync::{Mutex, broadcast, watch};
use uuid::Uuid;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// In-test transport: talks HTTP to a `wiremock::MockServer` without
/// the auth dance that production `ReqwestTransport` requires. Errors
/// and successful response bodies pass through the same classification
/// rules as the production transport — body-decode failures land on
/// [`SyncError::Decode`] rather than [`SyncError::Transport`].
#[derive(Clone)]
struct HttpTestTransport {
    base: reqwest::Url,
    http: reqwest::Client,
}

impl HttpTestTransport {
    fn new(base: &str) -> Self {
        let mut url: reqwest::Url = base.parse().expect("valid mock url");
        if !url.path().ends_with('/') {
            let mut path = url.path().to_owned();
            path.push('/');
            url.set_path(&path);
        }
        Self {
            base: url,
            http: reqwest::Client::new(),
        }
    }

    fn url(&self) -> reqwest::Url {
        self.base.join("settings").expect("valid /settings url")
    }
}

async fn decode_body<T: serde::de::DeserializeOwned>(response: reqwest::Response) -> SyncResult<T> {
    let bytes = response.bytes().await.map_err(SyncError::from_transport)?;
    Ok(serde_json::from_slice(&bytes)?)
}

#[async_trait]
impl SettingsTransport for HttpTestTransport {
    async fn get(&self) -> SyncResult<PullOutcome> {
        let response = self
            .http
            .get(self.url())
            .send()
            .await
            .map_err(SyncError::from_transport)?;
        match response.status() {
            StatusCode::OK => Ok(PullOutcome::Found(decode_body(response).await?)),
            StatusCode::NOT_FOUND => Ok(PullOutcome::NotFound),
            status => {
                let message = response.text().await.unwrap_or_default();
                Err(SyncError::Server { status, message })
            }
        }
    }

    async fn put(&self, body: PutSettingsRequest) -> SyncResult<PushOutcome> {
        let response = self
            .http
            .put(self.url())
            .json(&body)
            .send()
            .await
            .map_err(SyncError::from_transport)?;
        match response.status() {
            StatusCode::OK => Ok(PushOutcome::Accepted(decode_body(response).await?)),
            StatusCode::CONFLICT => Ok(PushOutcome::Conflict(decode_body(response).await?)),
            status => {
                let message = response.text().await.unwrap_or_default();
                Err(SyncError::Server { status, message })
            }
        }
    }

    async fn delete(&self) -> SyncResult<()> {
        let response = self
            .http
            .delete(self.url())
            .send()
            .await
            .map_err(SyncError::from_transport)?;
        match response.status() {
            StatusCode::OK | StatusCode::NO_CONTENT => Ok(()),
            status => {
                let message = response.text().await.unwrap_or_default();
                Err(SyncError::Server { status, message })
            }
        }
    }
}

/// Identity stub mirroring `euro_auth::AuthManager`'s contract without
/// the keyring. The engine reaches through `AuthIdentity` for the
/// `Option<Uuid>` it stamps into the cache; tests parameterize this
/// to model "logged in as X", "logged out", and the transient-error
/// retry path.
#[derive(Clone, Debug)]
enum IdentityScript {
    User(Uuid),
    LoggedOut,
    Transient,
}

struct ScriptedIdentity {
    script: IdentityScript,
}

impl ScriptedIdentity {
    fn user(uid: Uuid) -> Arc<Self> {
        Arc::new(Self {
            script: IdentityScript::User(uid),
        })
    }

    fn logged_out() -> Arc<Self> {
        Arc::new(Self {
            script: IdentityScript::LoggedOut,
        })
    }

    fn transient() -> Arc<Self> {
        Arc::new(Self {
            script: IdentityScript::Transient,
        })
    }
}

#[async_trait]
impl AuthIdentity for ScriptedIdentity {
    async fn current_user_id(&self) -> SyncResult<Option<Uuid>> {
        match self.script {
            IdentityScript::User(uid) => Ok(Some(uid)),
            IdentityScript::LoggedOut => Ok(None),
            IdentityScript::Transient => Err(SyncError::Internal(anyhow::anyhow!(
                "scripted transient identity failure"
            ))),
        }
    }
}

/// Stable user id used by tests that don't care about distinguishing
/// "user A" from "user B". Aliased here so the value reads the same in
/// every test and in the harness defaults.
fn default_test_user() -> Uuid {
    Uuid::parse_str("00000000-0000-4000-8000-000000000001").unwrap()
}

/// Subscribes to the engine's status channel and drops the value
/// already present at subscribe-time, so [`StatusWatcher::wait_for`]
/// only resolves on transitions caused by the test's *next* action.
///
/// Construct one *before* triggering the action you care about. This
/// is what makes the engine's "post-pull push" tests deterministic:
/// without discarding the initial value, a test that drives the
/// engine through two consecutive `Synced` transitions would match
/// the first one immediately and race against the second.
struct StatusWatcher {
    rx: watch::Receiver<SyncStatus>,
}

impl StatusWatcher {
    fn new(engine: &SyncEngine) -> Self {
        let mut rx = engine.subscribe();
        rx.mark_unchanged();
        Self { rx }
    }

    async fn wait_for<F>(&mut self, mut pred: F)
    where
        F: FnMut(&SyncStatus) -> bool,
    {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        loop {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                panic!(
                    "timed out waiting for status transition; last seen: {:?}",
                    *self.rx.borrow()
                );
            }
            if tokio::time::timeout(remaining, self.rx.changed())
                .await
                .is_err()
            {
                panic!(
                    "timed out waiting for status change; last seen: {:?}",
                    *self.rx.borrow()
                );
            }
            if pred(&self.rx.borrow_and_update()) {
                return;
            }
        }
    }
}

/// Test scaffolding: a wiremock server, a tempdir holding
/// `local.json` / `cloud.json`, and an engine bound to both. Drop
/// order is well-defined: `engine` (and any subscriber receivers) →
/// `_tmp` last, so the `.json` files exist for the duration of the
/// test.
struct Harness {
    server: MockServer,
    state: Arc<Mutex<SettingsState>>,
    engine: SyncEngine,
    tmp: TempDir,
}

impl Harness {
    async fn new() -> Self {
        Self::with_cache(CloudSettingsCache::default()).await
    }

    async fn with_cache(cache: CloudSettingsCache) -> Self {
        Self::with_cache_and_identity(cache, ScriptedIdentity::user(default_test_user())).await
    }

    async fn with_identity(identity: Arc<dyn AuthIdentity>) -> Self {
        Self::with_cache_and_identity(CloudSettingsCache::default(), identity).await
    }

    async fn with_cache_and_identity(
        cache: CloudSettingsCache,
        identity: Arc<dyn AuthIdentity>,
    ) -> Self {
        Self::build(cache, identity, None).await
    }

    /// Build a harness whose engine is wired to an `AuthEvent`
    /// broadcast channel. The returned `Sender` drives the engine's
    /// auth-event listener directly, simulating the bus that
    /// [`euro_auth::AuthManager::subscribe`] feeds in production.
    async fn with_auth_events(
        cache: CloudSettingsCache,
        identity: Arc<dyn AuthIdentity>,
    ) -> (Self, broadcast::Sender<AuthEvent>) {
        let (tx, rx) = broadcast::channel(16);
        let harness = Self::build(cache, identity, Some(rx)).await;
        (harness, tx)
    }

    async fn build(
        cache: CloudSettingsCache,
        identity: Arc<dyn AuthIdentity>,
        auth_events: Option<broadcast::Receiver<AuthEvent>>,
    ) -> Self {
        let server = MockServer::start().await;
        let tmp = TempDir::new().expect("tempdir");

        let initial = SettingsState {
            cache,
            ..SettingsState::default()
        };
        initial.save_local(tmp.path()).expect("seed local.json");
        initial.save_cache(tmp.path()).expect("seed cloud.json");
        // Reload through the canonical path so the harness exercises
        // the same load flow the production binary does.
        let loaded = SettingsState::load_or_migrate(tmp.path()).expect("load state");
        let state = Arc::new(Mutex::new(loaded));

        let transport: Arc<dyn SettingsTransport> = Arc::new(HttpTestTransport::new(&server.uri()));
        // bon's type-state builder forces a fixed call order, so we
        // can't conditionally chain `.auth_events(...)` — instead we
        // pass through the `maybe_*` setter which accepts the option
        // directly.
        let engine = SyncEngine::builder()
            .settings(state.clone())
            .transport(transport)
            .identity(identity)
            .config_dir(tmp.path().to_owned())
            .backoff(
                BackoffConfig::builder()
                    .initial(Duration::from_millis(10))
                    .max(Duration::from_millis(40))
                    .jitter(0.0)
                    .build(),
            )
            .maybe_auth_events(auth_events)
            .build();

        Self {
            server,
            state,
            engine,
            tmp,
        }
    }

    fn config_path(&self) -> &Path {
        self.tmp.path()
    }

    async fn cache_clone(&self) -> CloudSettingsCache {
        self.state.lock().await.cache.clone()
    }
}

/// Build a minimal `Claims` whose `sub` is the supplied uuid.
/// Other fields carry inert defaults — the listener only ever reads
/// `claims.sub`, and tests that exercise other fields should construct
/// their own claims.
fn claims_for(uid: Uuid) -> Claims {
    Claims {
        sub: uid.to_string(),
        email: "test@example.com".to_owned(),
        display_name: None,
        exp: 0,
        iat: 0,
        token_type: "access".to_owned(),
        role: Role::Free,
        aud: String::new(),
        email_verified: true,
        jti: String::new(),
    }
}

/// Poll the cache until `predicate` returns true or the deadline
/// elapses. Used by the auth-listener tests, which have no direct sync
/// point with the listener task — the listener consumes events on its
/// own runtime and writes through the shared `Mutex<SettingsState>`.
async fn wait_for_cache<F>(h: &Harness, mut predicate: F)
where
    F: FnMut(&CloudSettingsCache) -> bool,
{
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        if predicate(&h.cache_clone().await) {
            return;
        }
        if tokio::time::Instant::now() >= deadline {
            panic!(
                "timed out waiting for cache predicate; last seen: {:?}",
                h.cache_clone().await
            );
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

/// Wait for the engine's status watch to settle on a value matching
/// `pred`. Mirrors [`StatusWatcher::wait_for`] but takes a fresh
/// receiver so callers don't need to construct one before driving the
/// transition.
async fn wait_for_status<F>(engine: &SyncEngine, mut pred: F)
where
    F: FnMut(&SyncStatus) -> bool,
{
    let mut rx = engine.subscribe();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        if pred(&rx.borrow_and_update()) {
            return;
        }
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() || tokio::time::timeout(remaining, rx.changed()).await.is_err() {
            panic!(
                "timed out waiting for status; last seen: {:?}",
                *rx.borrow()
            );
        }
    }
}

fn timestamp(year: i32, month: u32, day: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(year, month, day, 0, 0, 0)
        .single()
        .unwrap()
}

/// Build a cache whose theme is set and whose OCC baseline says we've
/// already observed a server row stamped at `baseline_at`. Used by the
/// "cache fresher than server" branches.
fn cache_synced_at(theme: ThemePreference, baseline_at: DateTime<Utc>) -> CloudSettingsCache {
    let mut cache = CloudSettingsCache::default();
    cache.settings.shared.theme = theme;
    cache.base_updated_at = Some(baseline_at);
    cache
}

// --- Pull 404 → first-run upload ------------------------------------------

#[tokio::test]
async fn pull_404_triggers_first_run_upload() {
    let h = Harness::new().await;

    Mock::given(method("GET"))
        .and(path("/settings"))
        .respond_with(ResponseTemplate::new(404))
        .expect(1)
        .mount(&h.server)
        .await;

    let put_mock = Mock::given(method("PUT"))
        .and(path("/settings"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(PutSettingsAcceptedResponse {
                schema_version: CURRENT_SCHEMA_VERSION,
                updated_at: timestamp(2026, 1, 2),
            }),
        )
        .expect(1)
        .mount_as_scoped(&h.server)
        .await;

    let status = h.engine.pull_now().await.expect("pull_now ok");
    assert!(matches!(status, SyncStatus::Synced { .. }));

    let cache = h.cache_clone().await;
    assert_eq!(cache.base_updated_at, Some(timestamp(2026, 1, 2)));

    // The first-run upload must carry `baseUpdatedAt: null` so the
    // server inserts rather than colliding with an unrelated row.
    let calls = put_mock.received_requests().await;
    let body: serde_json::Value =
        serde_json::from_slice(&calls.last().unwrap().body).expect("PUT body JSON");
    assert_eq!(body["baseUpdatedAt"], serde_json::Value::Null);
}

// --- Pull 200 (server fresher) → cache replaced ---------------------------

#[tokio::test]
async fn pull_200_server_fresher_replaces_cache() {
    let h = Harness::with_cache(cache_synced_at(
        ThemePreference::Light,
        timestamp(2026, 1, 1),
    ))
    .await;

    let server_blob = serde_json::json!({
        "shared": { "theme": "dark", "dynamicAccent": true },
        "desktop": {
            "interfaceScale": 1.25,
            "textScale": 1.5,
            "telemetry": {
                "consentVersion": 1,
                "anonymousMetrics": true,
                "anonymousErrors": true,
                "nonAnonymousMetrics": false,
            },
        },
        "mobile": {},
        "web": {},
    });
    Mock::given(method("GET"))
        .and(path("/settings"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(GetSettingsResponse {
                schema_version: CURRENT_SCHEMA_VERSION,
                updated_at: timestamp(2026, 2, 1),
                settings: server_blob,
            }),
        )
        .expect(1)
        .mount(&h.server)
        .await;

    let status = h.engine.pull_now().await.expect("pull_now ok");
    assert!(matches!(status, SyncStatus::Synced { .. }));

    let cache = h.cache_clone().await;
    assert_eq!(cache.settings.shared.theme, ThemePreference::Dark);
    assert_eq!(cache.settings.desktop.interface_scale.get(), 1.25);
    assert_eq!(cache.base_updated_at, Some(timestamp(2026, 2, 1)));

    // Disk must mirror in-memory cache so subsequent runs hit the
    // fast path in `SettingsState::load_or_migrate`.
    let reloaded = SettingsState::load_or_migrate(h.config_path()).unwrap();
    assert_eq!(reloaded.cache.settings.shared.theme, ThemePreference::Dark);
    assert_eq!(reloaded.cache.base_updated_at, Some(timestamp(2026, 2, 1)));
}

// --- Pull 200 (cache fresher) → push queued -------------------------------

#[tokio::test]
async fn pull_200_cache_fresher_enqueues_push() {
    let h = Harness::with_cache(cache_synced_at(
        ThemePreference::Dark,
        timestamp(2026, 3, 1),
    ))
    .await;

    Mock::given(method("GET"))
        .and(path("/settings"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(GetSettingsResponse {
                schema_version: CURRENT_SCHEMA_VERSION,
                updated_at: timestamp(2026, 1, 1),
                settings: serde_json::to_value(CloudSettings::default()).unwrap(),
            }),
        )
        .expect(1)
        .mount(&h.server)
        .await;

    Mock::given(method("PUT"))
        .and(path("/settings"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(PutSettingsAcceptedResponse {
                schema_version: CURRENT_SCHEMA_VERSION,
                updated_at: timestamp(2026, 3, 2),
            }),
        )
        .expect(1)
        .mount(&h.server)
        .await;

    let status = h.engine.pull_now().await.expect("pull_now ok");
    let SyncStatus::Synced { at: pull_at } = status else {
        panic!("expected Synced after pull, got {status:?}");
    };

    // Subscribe *after* the pull lands so the watcher's `mark_unchanged`
    // discards the pull's Synced. From here on, every transition the
    // watcher sees is caused by the queued push being drained.
    let mut watcher = StatusWatcher::new(&h.engine);
    h.engine.start().await;
    watcher
        .wait_for(|s| matches!(s, SyncStatus::Synced { at } if *at > pull_at))
        .await;

    let cache = h.cache_clone().await;
    assert_eq!(cache.base_updated_at, Some(timestamp(2026, 3, 2)));
    h.engine.stop().await;
}

// --- PUT 409 → conflict; cache replaced -----------------------------------

#[tokio::test]
async fn put_409_replaces_cache_with_server_current() {
    let h = Harness::with_cache(cache_synced_at(
        ThemePreference::Dark,
        timestamp(2026, 1, 1),
    ))
    .await;

    let server_current = serde_json::json!({
        "shared": { "theme": "light", "dynamicAccent": false },
        "desktop": {
            "interfaceScale": 1.0,
            "textScale": 1.0,
            "telemetry": {
                "consentVersion": 1,
                "anonymousMetrics": false,
                "anonymousErrors": false,
                "nonAnonymousMetrics": false,
            },
        },
        "mobile": {},
        "web": {},
    });

    Mock::given(method("PUT"))
        .and(path("/settings"))
        .respond_with(
            ResponseTemplate::new(409).set_body_json(PutSettingsConflictResponse {
                schema_version: CURRENT_SCHEMA_VERSION,
                updated_at: timestamp(2026, 4, 1),
                current: server_current,
            }),
        )
        .expect(1)
        .mount(&h.server)
        .await;

    let mut watcher = StatusWatcher::new(&h.engine);
    h.engine.start().await;
    h.engine.request_push();
    watcher
        .wait_for(|s| matches!(s, SyncStatus::Conflict { .. }))
        .await;

    let cache = h.cache_clone().await;
    assert_eq!(cache.settings.shared.theme, ThemePreference::Light);
    assert_eq!(cache.base_updated_at, Some(timestamp(2026, 4, 1)));
    h.engine.stop().await;
}

// --- Coalescing -----------------------------------------------------------

#[tokio::test]
async fn rapid_request_pushes_coalesce() {
    let h = Harness::with_cache(cache_synced_at(
        ThemePreference::Dark,
        timestamp(2026, 1, 1),
    ))
    .await;

    // Respond to PUTs forever; we assert on the count at the end.
    let put_mock = Mock::given(method("PUT"))
        .and(path("/settings"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(PutSettingsAcceptedResponse {
                    schema_version: CURRENT_SCHEMA_VERSION,
                    updated_at: timestamp(2026, 1, 2),
                })
                // Hold the response briefly so the next ~9 requests
                // pile up in the coalescing queue rather than racing
                // their own worker pass.
                .set_delay(Duration::from_millis(50)),
        )
        .mount_as_scoped(&h.server)
        .await;

    h.engine.start().await;
    for _ in 0..10 {
        h.engine.request_push();
    }
    // Let the worker land any pending PUTs.
    tokio::time::sleep(Duration::from_millis(250)).await;
    h.engine.stop().await;

    let calls = put_mock.received_requests().await;
    assert!(
        calls.len() <= 2,
        "expected ≤ 2 PUTs from 10 rapid requests, got {}",
        calls.len()
    );
    assert!(!calls.is_empty(), "at least one PUT must have landed");
}

// --- Offline + recovery ---------------------------------------------------

#[tokio::test]
async fn push_retries_after_transient_failure() {
    let h = Harness::with_cache(cache_synced_at(
        ThemePreference::Dark,
        timestamp(2026, 1, 1),
    ))
    .await;

    // First attempt: 503. Worker classifies as Offline, retries via
    // the harness's millisecond-scale backoff.
    Mock::given(method("PUT"))
        .and(path("/settings"))
        .respond_with(ResponseTemplate::new(503).set_body_string("temporarily unavailable"))
        .up_to_n_times(1)
        .expect(1)
        .mount(&h.server)
        .await;

    // Second attempt: 200. Worker stamps the new updated_at and goes
    // back to Synced.
    Mock::given(method("PUT"))
        .and(path("/settings"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(PutSettingsAcceptedResponse {
                schema_version: CURRENT_SCHEMA_VERSION,
                updated_at: timestamp(2026, 5, 1),
            }),
        )
        .expect(1)
        .mount(&h.server)
        .await;

    let mut watcher = StatusWatcher::new(&h.engine);
    h.engine.start().await;
    h.engine.request_push();
    watcher
        .wait_for(|s| matches!(s, SyncStatus::Synced { .. }))
        .await;

    let cache = h.cache_clone().await;
    assert_eq!(cache.base_updated_at, Some(timestamp(2026, 5, 1)));
    h.engine.stop().await;
}

// --- start() idempotency --------------------------------------------------

#[tokio::test]
async fn start_is_idempotent() {
    let h = Harness::new().await;
    h.engine.start().await;
    h.engine.start().await;
    h.engine.start().await;
    // We can't assert directly on a single worker handle, but exiting
    // cleanly via stop() proves a second start() didn't dangle a
    // second task that would be aborted from the wrong slot.
    h.engine.stop().await;
}

// --- Extras preservation through pull-then-push ---------------------------

#[tokio::test]
async fn unknown_fields_round_trip_through_pull_then_push() {
    let h = Harness::with_cache(cache_synced_at(
        ThemePreference::Dark,
        timestamp(2026, 1, 1),
    ))
    .await;

    let server_blob = serde_json::json!({
        "shared": {
            "theme": "dark",
            "dynamicAccent": true,
            "futureSharedKnob": "x",
        },
        "desktop": {
            "interfaceScale": 1.0,
            "textScale": 1.0,
            "telemetry": {
                "consentVersion": 1,
                "anonymousMetrics": false,
                "anonymousErrors": false,
                "nonAnonymousMetrics": false,
                "futureTelemetryKnob": true,
            },
            "futureDesktopKnob": [1, 2, 3],
        },
        "mobile": { "futureMobileKnob": "y" },
        "web": { "futureWebKnob": "z" },
    });

    Mock::given(method("GET"))
        .and(path("/settings"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(GetSettingsResponse {
                schema_version: CURRENT_SCHEMA_VERSION,
                updated_at: timestamp(2026, 6, 1),
                settings: server_blob.clone(),
            }),
        )
        .expect(1)
        .mount(&h.server)
        .await;

    let put_mock = Mock::given(method("PUT"))
        .and(path("/settings"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(PutSettingsAcceptedResponse {
                schema_version: CURRENT_SCHEMA_VERSION,
                updated_at: timestamp(2026, 6, 2),
            }),
        )
        .mount_as_scoped(&h.server)
        .await;

    let status = h.engine.pull_now().await.expect("pull_now ok");
    let SyncStatus::Synced { at: pull_at } = status else {
        panic!("expected Synced after pull, got {status:?}");
    };

    let mut watcher = StatusWatcher::new(&h.engine);
    h.engine.start().await;
    h.engine.request_push();
    watcher
        .wait_for(|s| matches!(s, SyncStatus::Synced { at } if *at > pull_at))
        .await;
    h.engine.stop().await;

    let calls = put_mock.received_requests().await;
    assert!(!calls.is_empty(), "push must have landed");
    let body: serde_json::Value =
        serde_json::from_slice(&calls.last().unwrap().body).expect("PUT body JSON");

    assert_eq!(body["settings"]["shared"]["futureSharedKnob"], "x");
    assert_eq!(
        body["settings"]["desktop"]["telemetry"]["futureTelemetryKnob"],
        serde_json::json!(true)
    );
    assert_eq!(
        body["settings"]["desktop"]["futureDesktopKnob"],
        serde_json::json!([1, 2, 3])
    );
    assert_eq!(body["settings"]["mobile"]["futureMobileKnob"], "y");
    assert_eq!(body["settings"]["web"]["futureWebKnob"], "z");
}

// --- Deserialization clamping on pull --------------------------------------

#[tokio::test]
async fn pull_clamps_out_of_range_scales() {
    let h = Harness::new().await;

    let server_blob = serde_json::json!({
        "shared": { "theme": "system", "dynamicAccent": true },
        "desktop": {
            "interfaceScale": 9.0,
            "textScale": 0.1,
            "telemetry": {
                "consentVersion": 1,
                "anonymousMetrics": false,
                "anonymousErrors": false,
                "nonAnonymousMetrics": false,
            },
        },
        "mobile": {},
        "web": {},
    });
    Mock::given(method("GET"))
        .and(path("/settings"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(GetSettingsResponse {
                schema_version: CURRENT_SCHEMA_VERSION,
                updated_at: timestamp(2026, 7, 1),
                settings: server_blob,
            }),
        )
        .expect(1)
        .mount(&h.server)
        .await;

    h.engine.pull_now().await.expect("pull ok");
    let cache = h.cache_clone().await;
    assert_eq!(
        cache.settings.desktop.interface_scale,
        settings_core::InterfaceScale::MAX
    );
    assert_eq!(
        cache.settings.desktop.text_scale,
        settings_core::TextScale::MIN
    );
}

// --- Phase 6: auth identity + account isolation ---------------------------

#[tokio::test]
async fn first_run_upload_stamps_last_user_id() {
    // Fresh-install cache (no last_user_id, no baseline) + authenticated
    // user. GET 404 → first-run PUT → 200. After reconcile the cache
    // must carry the JWT subject and the server's updated_at.
    let uid = default_test_user();
    let h = Harness::with_cache_and_identity(
        CloudSettingsCache::default(),
        ScriptedIdentity::user(uid),
    )
    .await;

    Mock::given(method("GET"))
        .and(path("/settings"))
        .respond_with(ResponseTemplate::new(404))
        .expect(1)
        .mount(&h.server)
        .await;
    Mock::given(method("PUT"))
        .and(path("/settings"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(PutSettingsAcceptedResponse {
                schema_version: CURRENT_SCHEMA_VERSION,
                updated_at: timestamp(2026, 8, 1),
            }),
        )
        .expect(1)
        .mount(&h.server)
        .await;

    h.engine.pull_now().await.expect("pull_now ok");

    let cache = h.cache_clone().await;
    assert_eq!(cache.last_user_id, Some(uid));
    assert_eq!(cache.base_updated_at, Some(timestamp(2026, 8, 1)));

    // The stamp must also have hit disk so a restart sees the same
    // owner and the engine doesn't redo the first-run path.
    let reloaded = SettingsState::load_or_migrate(h.config_path()).unwrap();
    assert_eq!(reloaded.cache.last_user_id, Some(uid));
    assert_eq!(reloaded.cache.base_updated_at, Some(timestamp(2026, 8, 1)));
}

#[tokio::test]
async fn pull_200_replace_stamps_last_user_id() {
    // Cache previously owned by user A; user A is still logged in.
    // Server has fresher data; replace_cache must preserve / restamp
    // the owner so the round-trip doesn't accidentally clear it.
    let uid = default_test_user();
    let prior = CloudSettingsCache {
        last_user_id: Some(uid),
        base_updated_at: Some(timestamp(2026, 1, 1)),
        settings: CloudSettings::default(),
    };
    let h = Harness::with_cache_and_identity(prior, ScriptedIdentity::user(uid)).await;

    let server_blob = serde_json::json!({
        "shared": { "theme": "dark", "dynamicAccent": true },
        "desktop": {
            "interfaceScale": 1.0,
            "textScale": 1.0,
            "telemetry": {
                "consentVersion": 1,
                "anonymousMetrics": false,
                "anonymousErrors": false,
                "nonAnonymousMetrics": false,
            },
        },
        "mobile": {},
        "web": {},
    });
    Mock::given(method("GET"))
        .and(path("/settings"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(GetSettingsResponse {
                schema_version: CURRENT_SCHEMA_VERSION,
                updated_at: timestamp(2026, 9, 1),
                settings: server_blob,
            }),
        )
        .expect(1)
        .mount(&h.server)
        .await;

    h.engine.pull_now().await.expect("pull_now ok");

    let cache = h.cache_clone().await;
    assert_eq!(cache.last_user_id, Some(uid));
    assert_eq!(cache.settings.shared.theme, ThemePreference::Dark);
}

#[tokio::test]
async fn foreign_cache_is_discarded_before_any_io() {
    // Machine carries cache stamped for user A (theme: Dark). User B
    // logs in. Before any HTTP, the engine must in-memory replace
    // the cache with defaults stamped to B's id. The GET that follows
    // sees user B's blob and the resulting cache reflects B — A's
    // theme must not survive.
    let user_a = Uuid::parse_str("11111111-1111-4111-8111-111111111111").unwrap();
    let user_b = Uuid::parse_str("22222222-2222-4222-8222-222222222222").unwrap();

    let foreign = CloudSettingsCache {
        last_user_id: Some(user_a),
        base_updated_at: Some(timestamp(2025, 12, 1)),
        settings: {
            let mut s = CloudSettings::default();
            s.shared.theme = ThemePreference::Dark;
            s
        },
    };
    let h = Harness::with_cache_and_identity(foreign, ScriptedIdentity::user(user_b)).await;

    // Server-side row for user B uses Light theme.
    let server_blob = serde_json::json!({
        "shared": { "theme": "light", "dynamicAccent": false },
        "desktop": {
            "interfaceScale": 1.0,
            "textScale": 1.0,
            "telemetry": {
                "consentVersion": 1,
                "anonymousMetrics": false,
                "anonymousErrors": false,
                "nonAnonymousMetrics": false,
            },
        },
        "mobile": {},
        "web": {},
    });
    Mock::given(method("GET"))
        .and(path("/settings"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(GetSettingsResponse {
                schema_version: CURRENT_SCHEMA_VERSION,
                updated_at: timestamp(2026, 10, 1),
                settings: server_blob,
            }),
        )
        .expect(1)
        .mount(&h.server)
        .await;

    h.engine.pull_now().await.expect("pull_now ok");

    let cache = h.cache_clone().await;
    assert_eq!(cache.last_user_id, Some(user_b));
    assert_eq!(cache.settings.shared.theme, ThemePreference::Light);
    assert_eq!(cache.base_updated_at, Some(timestamp(2026, 10, 1)));

    // Disk reflects the reset + pulled state; the dark theme is gone.
    let reloaded = SettingsState::load_or_migrate(h.config_path()).unwrap();
    assert_eq!(reloaded.cache.settings.shared.theme, ThemePreference::Light);
    assert_eq!(reloaded.cache.last_user_id, Some(user_b));
}

#[tokio::test]
async fn foreign_cache_isolation_persists_before_network_failure() {
    // Even if the GET fails after isolation, the foreign-cache reset
    // must already have hit disk so a subsequent restart doesn't see
    // user A's theme through user B's session.
    let user_a = Uuid::parse_str("11111111-1111-4111-8111-111111111111").unwrap();
    let user_b = Uuid::parse_str("22222222-2222-4222-8222-222222222222").unwrap();

    let foreign = CloudSettingsCache {
        last_user_id: Some(user_a),
        base_updated_at: Some(timestamp(2025, 12, 1)),
        settings: {
            let mut s = CloudSettings::default();
            s.shared.theme = ThemePreference::Dark;
            s
        },
    };
    let h = Harness::with_cache_and_identity(foreign, ScriptedIdentity::user(user_b)).await;

    Mock::given(method("GET"))
        .and(path("/settings"))
        .respond_with(ResponseTemplate::new(500).set_body_string("upstream down"))
        .expect(1)
        .mount(&h.server)
        .await;

    let err = h.engine.pull_now().await.expect_err("pull must fail");
    assert!(matches!(err, SyncError::Server { .. }));

    // In-memory state and disk both reflect the isolation reset, not
    // user A's stale data.
    let cache = h.cache_clone().await;
    assert_eq!(cache.last_user_id, Some(user_b));
    assert_eq!(cache.settings.shared.theme, ThemePreference::default());
    assert!(cache.base_updated_at.is_none());

    let reloaded = SettingsState::load_or_migrate(h.config_path()).unwrap();
    assert_eq!(reloaded.cache.last_user_id, Some(user_b));
    assert_eq!(
        reloaded.cache.settings.shared.theme,
        ThemePreference::default()
    );
}

#[tokio::test]
async fn pull_skipped_when_unauthenticated() {
    // Logged-out boot must publish `LocalOnly` and issue zero HTTP
    // requests. The mock server records every incoming request, so
    // we can assert "nothing hit it."
    let h = Harness::with_identity(ScriptedIdentity::logged_out()).await;

    let status = h.engine.pull_now().await.expect("pull_now ok");
    assert_eq!(status, SyncStatus::LocalOnly);
    assert_eq!(h.engine.current_status(), SyncStatus::LocalOnly);

    let recorded = h.server.received_requests().await.unwrap_or_default();
    assert!(
        recorded.is_empty(),
        "unauthenticated pull must not hit the network; got {} request(s)",
        recorded.len()
    );

    // Cache untouched — no foreign isolation, no first-run upload.
    let cache = h.cache_clone().await;
    assert!(cache.last_user_id.is_none());
    assert!(cache.base_updated_at.is_none());
}

#[tokio::test]
async fn pull_propagates_transient_identity_error_as_offline() {
    // A transport-class failure resolving identity (auth refresh
    // crashes against a flaky server) must not be misclassified as a
    // logout. The engine surfaces it as a retryable Offline.
    let h = Harness::with_identity(ScriptedIdentity::transient()).await;

    let err = h
        .engine
        .pull_now()
        .await
        .expect_err("transient identity error propagates");
    assert!(err.is_retryable());
    assert!(matches!(err, SyncError::Internal(_)));
    assert!(matches!(
        h.engine.current_status(),
        SyncStatus::Offline { .. }
    ));

    // No requests issued — identity gate fired before any I/O.
    let recorded = h.server.received_requests().await.unwrap_or_default();
    assert!(recorded.is_empty());
}

#[tokio::test]
async fn reconcile_local_fresher_stamps_user_id_before_push() {
    // The "cache fresher than server" branch (`request_push`) must
    // also stamp `last_user_id` so a never-pushed cache transitions
    // out of the `None` state on first successful pull.
    let uid = default_test_user();
    let prior = CloudSettingsCache {
        last_user_id: None,
        base_updated_at: Some(timestamp(2026, 3, 1)),
        settings: {
            let mut s = CloudSettings::default();
            s.shared.theme = ThemePreference::Dark;
            s
        },
    };
    let h = Harness::with_cache_and_identity(prior, ScriptedIdentity::user(uid)).await;

    Mock::given(method("GET"))
        .and(path("/settings"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(GetSettingsResponse {
                schema_version: CURRENT_SCHEMA_VERSION,
                updated_at: timestamp(2026, 1, 1),
                settings: serde_json::to_value(CloudSettings::default()).unwrap(),
            }),
        )
        .expect(1)
        .mount(&h.server)
        .await;
    // The "cache fresher" branch queues a push; mount a PUT that
    // accepts it so the worker doesn't loop on retries.
    Mock::given(method("PUT"))
        .and(path("/settings"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(PutSettingsAcceptedResponse {
                schema_version: CURRENT_SCHEMA_VERSION,
                updated_at: timestamp(2026, 3, 2),
            }),
        )
        .mount(&h.server)
        .await;

    h.engine.pull_now().await.expect("pull_now ok");

    let cache = h.cache_clone().await;
    assert_eq!(
        cache.last_user_id,
        Some(uid),
        "pull must stamp owner even on the local-fresher branch"
    );
    // Theme stays Dark — the local edits weren't replaced.
    assert_eq!(cache.settings.shared.theme, ThemePreference::Dark);
}

// --- Decode failure is not retried ----------------------------------------

#[tokio::test]
async fn pull_with_garbage_body_classifies_as_decode_and_does_not_retry() {
    let h = Harness::new().await;

    // 200 with a body the client cannot parse as a `GetSettingsResponse`.
    // The transport must surface this as `SyncError::Decode`, which is
    // non-retryable; the engine must publish `Offline` and not loop.
    Mock::given(method("GET"))
        .and(path("/settings"))
        .respond_with(ResponseTemplate::new(200).set_body_string("not json {"))
        .expect(1)
        .mount(&h.server)
        .await;

    let err = h.engine.pull_now().await.expect_err("pull should fail");
    assert!(matches!(err, SyncError::Decode(_)), "got {err:?}");
    assert!(!err.is_retryable());
    assert!(matches!(
        h.engine.current_status(),
        SyncStatus::Offline { .. }
    ));
}

// --- Phase 8: pull serialisation across concurrent callers ----------------

#[tokio::test]
async fn pull_now_serialises_concurrent_invocations() {
    // With the `pull_lock`, N concurrent `pull_now` calls must run in
    // sequence rather than overlap. The mock injects a per-request
    // delay; serialised pulls take ≈ N × delay, parallel pulls take ≈
    // delay. We assert the serialised lower bound.
    let h = Harness::new().await;

    let get_delay = Duration::from_millis(80);
    let put_delay = Duration::from_millis(80);
    Mock::given(method("GET"))
        .and(path("/settings"))
        .respond_with(ResponseTemplate::new(404).set_delay(get_delay))
        .mount(&h.server)
        .await;
    Mock::given(method("PUT"))
        .and(path("/settings"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(PutSettingsAcceptedResponse {
                    schema_version: CURRENT_SCHEMA_VERSION,
                    updated_at: timestamp(2026, 1, 2),
                })
                .set_delay(put_delay),
        )
        .mount(&h.server)
        .await;

    let concurrency = 4;
    let start = tokio::time::Instant::now();
    let mut handles = Vec::with_capacity(concurrency);
    for _ in 0..concurrency {
        let engine = h.engine.clone();
        handles.push(tokio::spawn(async move {
            engine.pull_now().await.expect("pull_now ok")
        }));
    }
    for handle in handles {
        handle.await.expect("pull task panicked");
    }
    let elapsed = start.elapsed();

    // Sequential floor: each pull issues a GET *and* a PUT (the 404
    // path triggers a first-run upload). Allow some slack for runtime
    // scheduling overhead but require that we're nowhere near the
    // fully-parallel timing.
    let sequential_floor = (get_delay + put_delay) * concurrency as u32 - Duration::from_millis(40);
    assert!(
        elapsed >= sequential_floor,
        "expected serialised pulls (≥{sequential_floor:?}); got elapsed={elapsed:?}"
    );
}

// --- Phase 8: auth-event listener -----------------------------------------

#[tokio::test]
async fn auth_event_with_new_subject_triggers_pull() {
    // Fresh-install cache (no last_user_id) + listener wired to the
    // bus. An `AuthEvent { Some(claims) }` whose `sub` differs from
    // the (None) seed must drive a pull that stamps the cache.
    let uid = default_test_user();
    let (h, tx) =
        Harness::with_auth_events(CloudSettingsCache::default(), ScriptedIdentity::user(uid)).await;

    Mock::given(method("GET"))
        .and(path("/settings"))
        .respond_with(ResponseTemplate::new(404))
        .expect(1)
        .mount(&h.server)
        .await;
    Mock::given(method("PUT"))
        .and(path("/settings"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(PutSettingsAcceptedResponse {
                schema_version: CURRENT_SCHEMA_VERSION,
                updated_at: timestamp(2026, 8, 1),
            }),
        )
        .expect(1)
        .mount(&h.server)
        .await;

    h.engine.start().await;
    tx.send(AuthEvent {
        claims: Some(claims_for(uid)),
    })
    .expect("listener receiver alive");

    wait_for_cache(&h, |cache| cache.last_user_id == Some(uid)).await;
    let cache = h.cache_clone().await;
    assert_eq!(cache.base_updated_at, Some(timestamp(2026, 8, 1)));

    h.engine.stop().await;
}

#[tokio::test]
async fn auth_event_same_subject_does_not_trigger_pull() {
    // Cache already owned by `uid`; the listener seeds
    // `last_seen_subject = Some(uid)` from the cache, so a refresh
    // event carrying the same subject must be ignored. We assert
    // zero HTTP requests hit the mock.
    let uid = default_test_user();
    let prior = CloudSettingsCache {
        last_user_id: Some(uid),
        base_updated_at: Some(timestamp(2026, 1, 1)),
        settings: CloudSettings::default(),
    };
    let (h, tx) = Harness::with_auth_events(prior, ScriptedIdentity::user(uid)).await;

    h.engine.start().await;
    tx.send(AuthEvent {
        claims: Some(claims_for(uid)),
    })
    .expect("listener receiver alive");

    // The listener task needs a moment to consume and dedupe the
    // event. 150ms is well above the runtime's scheduling jitter and
    // well below anything the listener would do on a subject change
    // (which would entail an HTTP round-trip through wiremock).
    tokio::time::sleep(Duration::from_millis(150)).await;

    let recorded = h.server.received_requests().await.unwrap_or_default();
    assert!(
        recorded.is_empty(),
        "refresh-only AuthEvent must not trigger a network call; got {} request(s)",
        recorded.len()
    );

    h.engine.stop().await;
}

#[tokio::test]
async fn auth_event_logout_flips_status_to_local_only_without_io() {
    // A `None` claims event represents a logout. Per plan.md the
    // listener must publish `LocalOnly`, do no I/O, and keep the
    // cache intact so offline-after-logout reads still work.
    let uid = default_test_user();
    let prior = CloudSettingsCache {
        last_user_id: Some(uid),
        base_updated_at: Some(timestamp(2026, 1, 1)),
        settings: {
            let mut s = CloudSettings::default();
            s.shared.theme = ThemePreference::Dark;
            s
        },
    };
    let (h, tx) = Harness::with_auth_events(prior, ScriptedIdentity::user(uid)).await;

    h.engine.start().await;
    // Drive the engine off LocalOnly with a "server fresher" pull so
    // there's a distinct transition back to LocalOnly to observe.
    // Keeping the theme Dark in the server blob (matching the cache)
    // also lets us assert the cache survives the logout untouched.
    let server_blob = serde_json::json!({
        "shared": { "theme": "dark", "dynamicAccent": false },
        "desktop": {
            "interfaceScale": 1.0,
            "textScale": 1.0,
            "telemetry": {
                "consentVersion": 1,
                "anonymousMetrics": false,
                "anonymousErrors": false,
                "nonAnonymousMetrics": false,
            },
        },
        "mobile": {},
        "web": {},
    });
    Mock::given(method("GET"))
        .and(path("/settings"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(GetSettingsResponse {
                schema_version: CURRENT_SCHEMA_VERSION,
                // Strictly fresher than the cache's `base_updated_at`
                // so reconcile takes `replace_cache` and does not
                // queue a push that would race the baseline count.
                updated_at: timestamp(2026, 5, 1),
                settings: server_blob,
            }),
        )
        .expect(1)
        .mount(&h.server)
        .await;
    h.engine.pull_now().await.expect("initial pull ok");
    wait_for_status(&h.engine, |s| matches!(s, SyncStatus::Synced { .. })).await;
    let baseline_requests = h.server.received_requests().await.unwrap_or_default().len();

    tx.send(AuthEvent { claims: None })
        .expect("listener receiver alive");
    wait_for_status(&h.engine, |s| matches!(s, SyncStatus::LocalOnly)).await;

    // Cache still owned by `uid`; nothing wiped on logout.
    let cache = h.cache_clone().await;
    assert_eq!(cache.last_user_id, Some(uid));
    assert_eq!(cache.settings.shared.theme, ThemePreference::Dark);

    // No additional HTTP requests after the logout event.
    let after_logout = h.server.received_requests().await.unwrap_or_default().len();
    assert_eq!(
        after_logout, baseline_requests,
        "logout event must not trigger network I/O"
    );

    h.engine.stop().await;
}

#[tokio::test]
async fn auth_event_login_after_logout_re_triggers_pull() {
    // A `None` event clears `last_seen_subject`; a subsequent
    // `Some(uid)` — even for the same user — must re-trigger a pull
    // so a user who logs back in sees a fresh server snapshot.
    let uid = default_test_user();
    let prior = CloudSettingsCache {
        last_user_id: Some(uid),
        base_updated_at: Some(timestamp(2026, 1, 1)),
        settings: CloudSettings::default(),
    };
    let (h, tx) = Harness::with_auth_events(prior, ScriptedIdentity::user(uid)).await;

    let server_blob = serde_json::json!({
        "shared": { "theme": "dark", "dynamicAccent": true },
        "desktop": {
            "interfaceScale": 1.0,
            "textScale": 1.0,
            "telemetry": {
                "consentVersion": 1,
                "anonymousMetrics": false,
                "anonymousErrors": false,
                "nonAnonymousMetrics": false,
            },
        },
        "mobile": {},
        "web": {},
    });
    Mock::given(method("GET"))
        .and(path("/settings"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(GetSettingsResponse {
                schema_version: CURRENT_SCHEMA_VERSION,
                updated_at: timestamp(2026, 9, 1),
                settings: server_blob,
            }),
        )
        // Exactly one GET — only the post-logout login should hit the
        // network; the matching-subject seed must keep the listener
        // quiet on startup.
        .expect(1)
        .mount(&h.server)
        .await;

    h.engine.start().await;

    // Logout first to clear the listener's tracked subject.
    tx.send(AuthEvent { claims: None })
        .expect("listener receiver alive");
    wait_for_status(&h.engine, |s| matches!(s, SyncStatus::LocalOnly)).await;

    // Log back in as the same user. With `last_seen_subject == None`,
    // the listener must treat this as a transition and pull.
    tx.send(AuthEvent {
        claims: Some(claims_for(uid)),
    })
    .expect("listener receiver alive");

    wait_for_cache(&h, |cache| {
        cache.base_updated_at == Some(timestamp(2026, 9, 1))
    })
    .await;
    let cache = h.cache_clone().await;
    assert_eq!(cache.settings.shared.theme, ThemePreference::Dark);

    h.engine.stop().await;
}

#[tokio::test]
async fn auth_event_subject_change_replaces_cache() {
    // The cross-account scenario from plan.md: cache stamped for
    // user A; an `AuthEvent` for user B arrives. The listener pulls;
    // the pull's account-isolation step wipes A's cache before any
    // network call; the GET returns B's blob; the cache ends up
    // stamped for B with B's data.
    let user_a = Uuid::parse_str("11111111-1111-4111-8111-111111111111").unwrap();
    let user_b = Uuid::parse_str("22222222-2222-4222-8222-222222222222").unwrap();

    let prior = CloudSettingsCache {
        last_user_id: Some(user_a),
        base_updated_at: Some(timestamp(2026, 1, 1)),
        settings: {
            let mut s = CloudSettings::default();
            s.shared.theme = ThemePreference::Dark;
            s
        },
    };
    // Identity reports user B — i.e. the AuthManager has already
    // switched to B's session by the time the listener reacts.
    let (h, tx) = Harness::with_auth_events(prior, ScriptedIdentity::user(user_b)).await;

    let server_blob = serde_json::json!({
        "shared": { "theme": "light", "dynamicAccent": false },
        "desktop": {
            "interfaceScale": 1.0,
            "textScale": 1.0,
            "telemetry": {
                "consentVersion": 1,
                "anonymousMetrics": false,
                "anonymousErrors": false,
                "nonAnonymousMetrics": false,
            },
        },
        "mobile": {},
        "web": {},
    });
    Mock::given(method("GET"))
        .and(path("/settings"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(GetSettingsResponse {
                schema_version: CURRENT_SCHEMA_VERSION,
                updated_at: timestamp(2026, 10, 1),
                settings: server_blob,
            }),
        )
        .expect(1)
        .mount(&h.server)
        .await;

    h.engine.start().await;
    tx.send(AuthEvent {
        claims: Some(claims_for(user_b)),
    })
    .expect("listener receiver alive");

    // The wipe step stamps `last_user_id = user_b` *before* the network
    // pull replaces the cache contents (covered by
    // `foreign_cache_isolation_persists_before_network_failure`), so
    // waiting on just `last_user_id` races the pull. Pin both fields to
    // a post-pull state to land on the replaced blob.
    wait_for_cache(&h, |cache| {
        cache.last_user_id == Some(user_b) && cache.base_updated_at == Some(timestamp(2026, 10, 1))
    })
    .await;
    let cache = h.cache_clone().await;
    assert_eq!(cache.settings.shared.theme, ThemePreference::Light);
    assert_eq!(cache.base_updated_at, Some(timestamp(2026, 10, 1)));

    // The reset must have hit disk too — a restart under user B
    // must not observe A's Dark theme.
    let reloaded = SettingsState::load_or_migrate(h.config_path()).unwrap();
    assert_eq!(reloaded.cache.last_user_id, Some(user_b));
    assert_eq!(reloaded.cache.settings.shared.theme, ThemePreference::Light);

    h.engine.stop().await;
}

// --- ServerAhead: schema version newer than this build --------------------

#[tokio::test]
async fn pull_with_future_schema_version_refuses_to_replace_cache() {
    // Cache holds Dark + a known baseline. Server returns 200 with a
    // schema version newer than this build. The engine must NOT
    // adopt the server blob — that would require serialising back
    // from a partial-shape `CloudSettings`, which has no top-level
    // `extras` and would silently drop unknown sections. Instead it
    // raises ServerAhead, leaves the cache alone, and publishes the
    // matching status.
    let prior = cache_synced_at(ThemePreference::Dark, timestamp(2026, 1, 1));
    let h = Harness::with_cache(prior).await;

    Mock::given(method("GET"))
        .and(path("/settings"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(GetSettingsResponse {
                schema_version: CURRENT_SCHEMA_VERSION + 1,
                updated_at: timestamp(2026, 11, 1),
                settings: serde_json::json!({
                    "shared": { "theme": "light", "dynamicAccent": false },
                    "desktop": {
                        "interfaceScale": 1.0,
                        "textScale": 1.0,
                        "telemetry": {
                            "consentVersion": 1,
                            "anonymousMetrics": false,
                            "anonymousErrors": false,
                            "nonAnonymousMetrics": false,
                        },
                    },
                    "mobile": {},
                    "web": {},
                }),
            }),
        )
        .expect(1)
        .mount(&h.server)
        .await;

    let err = h
        .engine
        .pull_now()
        .await
        .expect_err("future schema must surface as ServerAhead");
    assert!(
        matches!(
            err,
            SyncError::ServerAhead { client, server }
                if client == CURRENT_SCHEMA_VERSION && server == CURRENT_SCHEMA_VERSION + 1
        ),
        "got {err:?}"
    );
    assert!(!err.is_retryable());
    assert_eq!(h.engine.current_status(), SyncStatus::ServerAhead);

    // Cache untouched: still Dark, still baselined to the original
    // timestamp the harness seeded. The server's row is *not*
    // adopted in any form — we cannot fully parse it.
    let cache = h.cache_clone().await;
    assert_eq!(cache.settings.shared.theme, ThemePreference::Dark);
    assert_eq!(cache.base_updated_at, Some(timestamp(2026, 1, 1)));
}

#[tokio::test]
async fn pushes_short_circuit_after_observing_future_schema_version() {
    // Once a pull has observed `schema_version > CURRENT`, the engine
    // latches an in-memory write-lock so the push worker stops issuing
    // PUTs. The intent is dropped (no retry) and the status reflects
    // ServerAhead. Without the latch, an older client could still
    // clobber a v(N+1) blob with a v(N) shape.
    let prior = cache_synced_at(ThemePreference::Dark, timestamp(2026, 1, 1));
    let h = Harness::with_cache(prior).await;

    // GET returns a future schema version.
    Mock::given(method("GET"))
        .and(path("/settings"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(GetSettingsResponse {
                schema_version: CURRENT_SCHEMA_VERSION + 1,
                updated_at: timestamp(2026, 11, 1),
                settings: serde_json::json!({
                    "shared": { "theme": "light", "dynamicAccent": false },
                    "desktop": {
                        "interfaceScale": 1.0,
                        "textScale": 1.0,
                        "telemetry": {
                            "consentVersion": 1,
                            "anonymousMetrics": false,
                            "anonymousErrors": false,
                            "nonAnonymousMetrics": false,
                        },
                    },
                    "mobile": {},
                    "web": {},
                }),
            }),
        )
        .mount(&h.server)
        .await;

    // The pull latches `schema_ahead`; ignore the error here, we only
    // care about the side effect.
    let _ = h.engine.pull_now().await;
    assert_eq!(h.engine.current_status(), SyncStatus::ServerAhead);

    // Track every PUT that lands. There must be zero.
    let put_mock = Mock::given(method("PUT"))
        .and(path("/settings"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(PutSettingsAcceptedResponse {
                schema_version: CURRENT_SCHEMA_VERSION,
                updated_at: timestamp(2026, 11, 2),
            }),
        )
        .mount_as_scoped(&h.server)
        .await;

    h.engine.start().await;
    h.engine.request_push();
    // Give the worker time to drain the queue. A PUT would land well
    // inside this window; the latch must short-circuit it before the
    // request is constructed.
    wait_for_status(&h.engine, |s| matches!(s, SyncStatus::ServerAhead)).await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    h.engine.stop().await;

    let calls = put_mock.received_requests().await;
    assert!(
        calls.is_empty(),
        "schema_ahead latch must suppress all PUTs; got {} request(s)",
        calls.len()
    );
}

#[tokio::test]
async fn put_409_with_future_schema_version_refuses_to_replace_cache() {
    // Server returns a 409 carrying a `current` row written under a
    // newer schema. The engine must refuse to adopt it (same reason
    // as the GET path) and surface ServerAhead. The push intent is
    // dropped; no retry — `is_retryable()` is false for ServerAhead.
    let prior = cache_synced_at(ThemePreference::Dark, timestamp(2026, 1, 1));
    let h = Harness::with_cache(prior).await;

    Mock::given(method("PUT"))
        .and(path("/settings"))
        .respond_with(
            ResponseTemplate::new(409).set_body_json(PutSettingsConflictResponse {
                schema_version: CURRENT_SCHEMA_VERSION + 1,
                updated_at: timestamp(2026, 11, 1),
                current: serde_json::json!({
                    "shared": { "theme": "light", "dynamicAccent": false },
                    "desktop": {
                        "interfaceScale": 1.0,
                        "textScale": 1.0,
                        "telemetry": {
                            "consentVersion": 1,
                            "anonymousMetrics": false,
                            "anonymousErrors": false,
                            "nonAnonymousMetrics": false,
                        },
                    },
                    "mobile": {},
                    "web": {},
                }),
            }),
        )
        .expect(1)
        .mount(&h.server)
        .await;

    h.engine.start().await;
    h.engine.request_push();
    wait_for_status(&h.engine, |s| matches!(s, SyncStatus::ServerAhead)).await;
    h.engine.stop().await;

    // Cache untouched: server's `current` row was not adopted.
    let cache = h.cache_clone().await;
    assert_eq!(cache.settings.shared.theme, ThemePreference::Dark);
    assert_eq!(cache.base_updated_at, Some(timestamp(2026, 1, 1)));
}

#[tokio::test]
async fn push_carries_current_schema_version_regardless_of_cache_state() {
    // The envelope's `schema_version` is unconditionally
    // `CURRENT_SCHEMA_VERSION` — there is no cache-side version to
    // round-trip back into the wire format any more. This test pins
    // the invariant by inspecting the request body that lands on the
    // mock.
    let prior = cache_synced_at(ThemePreference::Dark, timestamp(2026, 1, 1));
    let h = Harness::with_cache(prior).await;

    let put_mock = Mock::given(method("PUT"))
        .and(path("/settings"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(PutSettingsAcceptedResponse {
                schema_version: CURRENT_SCHEMA_VERSION,
                updated_at: timestamp(2026, 11, 1),
            }),
        )
        .expect(1)
        .mount_as_scoped(&h.server)
        .await;

    let mut watcher = StatusWatcher::new(&h.engine);
    h.engine.start().await;
    h.engine.request_push();
    watcher
        .wait_for(|s| matches!(s, SyncStatus::Synced { .. }))
        .await;
    h.engine.stop().await;

    let calls = put_mock.received_requests().await;
    let body: serde_json::Value =
        serde_json::from_slice(&calls.last().unwrap().body).expect("PUT body JSON");
    assert_eq!(body["schemaVersion"], CURRENT_SCHEMA_VERSION);
    // And the inner blob must not duplicate it.
    assert!(
        body["settings"].get("schemaVersion").is_none(),
        "schemaVersion must live on the envelope only, not inside settings"
    );
}
