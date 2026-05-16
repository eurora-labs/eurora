use auth_core::{AppleNativeUser, Claims, Provider};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use euro_auth::AuthManager;
use euro_auth::tauri::auth_manager;
use euro_secret::ExposeSecret;
use rand::Rng;
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, Manager};
use tauri_plugin_appauth::{AppAuthExt, BrowserOnlyRequest};
use tauri_plugin_apple_auth::{
    AppleAuthExt, AppleSignInOutcome, SignInRequest as AppleSignInRequest,
};
use tauri_plugin_google_auth::{GoogleAuthExt, SignInRequest};
use url::Url;

use crate::error::ResultExt;
use crate::shared_types::SharedSettingsState;

fn resolve_auth_manager(app_handle: &AppHandle) -> Result<AuthManager, String> {
    auth_manager(app_handle).ok_or_else(|| "auth manager not available".to_string())
}

/// Custom URL scheme the in-app browser session is bound to. iOS doesn't
/// need this in `Info.plist` — `ASWebAuthenticationSession`'s
/// `callbackURLScheme:` constructor binds it directly. Android registers
/// it via the `tauriBrowserRedirectScheme` manifest placeholder
/// (`gen/android/app/build.gradle.kts`). The backend's mobile OAuth
/// callback handler 302s here once a third-party login completes;
/// `tauri-plugin-appauth` captures the redirect and resolves the
/// awaited future.
const REDIRECT_URI: &str = "eurora://mobile/callback";

#[derive(Clone, Debug, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum LoginOutcome {
    Success,
    Canceled,
    /// Provider or backend refused the sign-in for a non-cancellation
    /// reason. `reason` is the underlying error category — useful for
    /// surfacing diagnostics, not for branching on the UI side. The
    /// frontend should treat every `rejected` outcome the same.
    Rejected {
        reason: String,
    },
    /// Native sign-in is not available on this device (e.g. Android
    /// without Play Services). The frontend should retry via the
    /// in-app browser flow.
    NativeUnavailable,
}

impl LoginOutcome {
    /// Build a `Rejected` outcome from any `Display`-able value.
    /// Centralises the conversion so we never accidentally serialise
    /// `Debug` formatting (which would leak Rust type names) or
    /// truncate a context-rich error to a one-word category.
    fn rejected(reason: impl std::fmt::Display) -> Self {
        Self::Rejected {
            reason: reason.to_string(),
        }
    }
}

async fn save_settings(app_handle: &AppHandle) -> Result<(), String> {
    let state = app_handle.state::<SharedSettingsState>();
    let settings = state.lock().await;
    settings
        .save_local_to_default_path()
        .ctx("Failed to save local settings")?;
    settings
        .save_cache_to_default_path()
        .ctx("Failed to save cloud cache")
}

/// Inspect the redirect URL captured by the in-app browser. The backend
/// callback always lands on `eurora://mobile/callback`, with
/// `?status=ok` on success or `?status=error&error=<kind>` on failure.
/// We surface the latter as a `Rejected` outcome rather than completing
/// the login-token exchange (which would just fail with `InvalidToken`).
fn parse_callback_status(url: &str) -> Result<(), String> {
    let parsed = Url::parse(url).ctx("backend returned malformed callback URL")?;
    let status = parsed
        .query_pairs()
        .find_map(|(k, v)| (k == "status").then(|| v.into_owned()));
    match status.as_deref() {
        Some("ok") | None => Ok(()),
        Some("error") => {
            let kind = parsed
                .query_pairs()
                .find_map(|(k, v)| (k == "error").then(|| v.into_owned()))
                .unwrap_or_else(|| "unknown".into());
            Err(kind)
        }
        Some(other) => Err(format!("unknown status `{other}`")),
    }
}

/// Open the chosen provider's authorisation page in an in-app browser,
/// complete sign-in, and exchange the device's PKCE verifier for
/// session tokens once the redirect fires.
///
/// **Flow shape.** The mobile crate generates a fresh PKCE pair locally
/// and POSTs the challenge to the backend's `/auth/oauth/mobile/url`
/// endpoint. The backend stamps that challenge as the OAuth `state`
/// (so it round-trips through Google / GitHub) and returns the provider
/// authorisation URL. After the user completes sign-in, the provider
/// 302s to the backend's `/auth/oauth/{provider}/mobile-callback`,
/// which atomically completes login and 302s to
/// `eurora://mobile/callback?status=ok|error`. The in-app browser
/// captures the redirect; we then redeem the verifier (still in this
/// awaiting frame) for our own access / refresh tokens.
///
/// **Browser-session sharing.** We deliberately do *not* set
/// `prefers_ephemeral_session: true`. Sharing cookies with the system
/// browser is what enables the "I'm already signed in" UX users expect:
///
/// - "Sign in with Google" recognises the user's existing Google
///   session in the system browser and shows the account picker
///   instead of prompting for email / password / 2FA;
/// - iOS Keychain / Android Autofill and password managers can fill
///   saved credentials for our domain;
/// - the user's existing session on our own domain (if any) is
///   honoured.
///
/// The trade-off is that the in-app browser sees whatever is currently
/// signed in to the user's system browser — which is exactly what users
/// mean when they tap "Sign in with Google". For workflows that
/// explicitly need a clean session, the user can sign out in the system
/// browser first.
#[tauri::command]
#[specta::specta]
pub async fn auth_start_login(
    app_handle: AppHandle,
    provider: Provider,
) -> Result<LoginOutcome, String> {
    let auth_manager = resolve_auth_manager(&app_handle)?;

    let (code_verifier, code_challenge) = auth_manager
        .get_login_tokens()
        .await
        .ctx("Failed to get login tokens")?;

    let auth_url = auth_manager
        .mobile_third_party_auth_url(provider, code_challenge)
        .await
        .ctx("Failed to start mobile OAuth")?;

    // Run the in-app browser session via appauth. The verifier lives
    // only in this awaiting frame — never touches disk — and is
    // consumed by the backend exchange below. If the app is killed
    // mid-flow the OS tears down the browser session anyway, so
    // persistence wouldn't recover us.
    let session = app_handle
        .appauth()
        .authorize_browser_only(BrowserOnlyRequest {
            auth_url,
            redirect_uri: REDIRECT_URI.to_string(),
            prefers_ephemeral_session: false,
        })
        .await;

    let callback = match session {
        Ok(resp) => resp,
        Err(err) => {
            let code = err.code();
            if code == "USER_CANCELED" {
                return Ok(LoginOutcome::Canceled);
            }
            return Err(format!("[{code}] {err}"));
        }
    };

    if let Err(kind) = parse_callback_status(&callback.url) {
        tracing::warn!(error = %kind, "mobile OAuth callback returned error");
        return Ok(LoginOutcome::rejected(kind));
    }

    match auth_manager.login_by_login_token(code_verifier).await {
        Ok(_) => {
            save_settings(&app_handle).await?;
            Ok(LoginOutcome::Success)
        }
        Err(e) => {
            tracing::error!("Login by login token failed: {e}");
            Ok(LoginOutcome::rejected(e))
        }
    }
}

/// Native Google sign-in via `tauri-plugin-google-auth`. On iOS this
/// drives the GoogleSignIn SDK (account picker, no browser); on Android
/// the Credential Manager API (system bottom sheet). Returns
/// [`LoginOutcome::NativeUnavailable`] when the device can't service
/// the request — Android without Play Services, or any non-mobile
/// platform — so the frontend can fall back to [`auth_start_login`].
///
/// We pass the platform-appropriate Google client ID:
/// - iOS uses the iOS OAuth client (`GOOGLE_CLIENT_ID_IOS`); the
///   resulting JWT carries `aud == GOOGLE_CLIENT_ID_IOS`, which the
///   backend accepts via its `accepted_audiences` list.
/// - Android Credential Manager only takes the *server* client ID
///   (`GOOGLE_CLIENT_ID`); the resulting JWT carries `aud ==
///   GOOGLE_CLIENT_ID` so the backend's primary verifier accepts it.
#[tauri::command]
#[specta::specta]
pub async fn auth_start_login_google_native(app_handle: AppHandle) -> Result<LoginOutcome, String> {
    let Some(client_id) = native_google_client_id() else {
        return Ok(LoginOutcome::NativeUnavailable);
    };

    let auth_manager = resolve_auth_manager(&app_handle)?;

    let response = match app_handle.google_auth().sign_in(SignInRequest {
        client_id,
        client_secret: None,
        scopes: Some(vec![
            "openid".to_string(),
            "email".to_string(),
            "profile".to_string(),
        ]),
        hosted_domain: None,
        login_hint: None,
        redirect_uri: None,
        success_html_response: None,
        // Android: prefer the native Credential Manager UI over the
        // older web flow. iOS ignores this field.
        flow_type: Some(tauri_plugin_google_auth::FlowType::Native),
    }) {
        Ok(resp) => resp,
        Err(e) => return Ok(classify_native_google_error(&e)),
    };

    let Some(id_token) = response.id_token else {
        tracing::warn!("native Google sign-in returned no id_token");
        return Ok(LoginOutcome::rejected("missing_id_token"));
    };

    match auth_manager.login_by_google_id_token(id_token, None).await {
        Ok(_) => {
            save_settings(&app_handle).await?;
            Ok(LoginOutcome::Success)
        }
        Err(e) => {
            tracing::error!("Google ID-token login failed: {e}");
            Ok(LoginOutcome::rejected(e))
        }
    }
}

/// Pick the Google OAuth client ID the native SDK should advertise on
/// this platform. Returns `None` on unsupported platforms (desktop) or
/// when no client ID is configured for the current target — both
/// surface to the caller as `NativeUnavailable`.
fn native_google_client_id() -> Option<String> {
    if cfg!(target_os = "ios") {
        std::env::var("GOOGLE_CLIENT_ID_IOS")
            .ok()
            .filter(|s| !s.is_empty())
    } else if cfg!(target_os = "android") {
        // Android Credential Manager only ever takes the *server*
        // client ID (the same one the backend verifies against). Apps
        // are bound to the project via signing-cert SHA registered in
        // Google Cloud Console — the Android client ID is never
        // surfaced to the SDK.
        std::env::var("GOOGLE_CLIENT_ID")
            .ok()
            .filter(|s| !s.is_empty())
    } else {
        None
    }
}

fn classify_native_google_error(err: &tauri_plugin_google_auth::Error) -> LoginOutcome {
    use tauri_plugin_google_auth::Error;
    match err {
        Error::UserCancelled => LoginOutcome::Canceled,
        Error::ConfigurationError(_) | Error::InvalidClientId => LoginOutcome::NativeUnavailable,
        other => {
            tracing::warn!(error = %other, "native Google sign-in failed");
            LoginOutcome::rejected(other)
        }
    }
}

/// Bytes of OS randomness in the raw Apple nonce. 32 bytes
/// (256 bits, base64url-encoded) matches what we use for OAuth state
/// elsewhere and exceeds Apple's recommended floor.
const APPLE_RAW_NONCE_BYTES: usize = 32;

/// Generate a fresh, opaque raw nonce for an Apple sign-in attempt.
///
/// The plugin SHA-256-hashes this before assigning to
/// `ASAuthorizationAppleIDRequest.nonce`; the backend re-derives the
/// same hash to match against the ID token's `nonce` claim. Returning
/// the raw value here means the unhashed nonce lives only in this
/// awaiting frame and is consumed by the backend exchange below — it
/// never touches disk.
fn generate_apple_raw_nonce() -> String {
    let mut bytes = [0u8; APPLE_RAW_NONCE_BYTES];
    rand::rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

/// Native Apple sign-in via `tauri-plugin-apple-auth`. On iOS this
/// drives `ASAuthorizationController` (the system Apple sheet, FaceID /
/// TouchID, no browser); on every other target the plugin returns
/// [`AppleSignInOutcome::NativeUnavailable`] and we surface that to the
/// frontend so it can fall back to [`auth_start_login`].
///
/// Unlike Google, the iOS flow does not need a client ID — the
/// `ASAuthorizationAppleIDProvider` request is bound to the iOS Bundle
/// ID via the provisioning profile's Sign in with Apple capability.
/// The backend's dual-audience verifier accepts both
/// `APPLE_SERVICE_ID` (web) and `APPLE_BUNDLE_ID` (native iOS) audiences,
/// so the resulting `(provider, sub)` pair lines up with web sign-ins
/// of the same Apple ID.
#[tauri::command]
#[specta::specta]
pub async fn auth_start_login_apple_native(app_handle: AppHandle) -> Result<LoginOutcome, String> {
    // Short-circuit on non-iOS: the plugin's `stub` bridge would
    // return `NativeUnavailable` anyway, but skipping the call keeps
    // the cfg-gated path free of plugin-bridge serialisation work on
    // every Android tap.
    if !cfg!(target_os = "ios") {
        return Ok(LoginOutcome::NativeUnavailable);
    }

    let auth_manager = resolve_auth_manager(&app_handle)?;
    let raw_nonce = generate_apple_raw_nonce();

    let outcome = match app_handle
        .apple_auth()
        .sign_in_with_apple(AppleSignInRequest {
            raw_nonce: raw_nonce.clone(),
        })
        .await
    {
        Ok(o) => o,
        Err(e) => {
            tracing::warn!(code = %e.code(), error = %e, "Apple native sign-in bridge failed");
            return Ok(LoginOutcome::rejected(e.code()));
        }
    };

    let response = match outcome {
        AppleSignInOutcome::Success(r) => r,
        AppleSignInOutcome::Cancelled => return Ok(LoginOutcome::Canceled),
        AppleSignInOutcome::Rejected(reason) => {
            tracing::warn!(error = %reason, "Apple native sign-in rejected");
            return Ok(LoginOutcome::rejected(reason));
        }
        AppleSignInOutcome::NativeUnavailable => return Ok(LoginOutcome::NativeUnavailable),
    };

    let user = response.user.map(|u| AppleNativeUser {
        first_name: u.first_name,
        last_name: u.last_name,
    });

    match auth_manager
        .login_by_apple_id_token(response.id_token, raw_nonce, user)
        .await
    {
        Ok(_) => {
            save_settings(&app_handle).await?;
            Ok(LoginOutcome::Success)
        }
        Err(e) => {
            tracing::error!("Apple ID-token login failed: {e}");
            Ok(LoginOutcome::rejected(e))
        }
    }
}

#[tauri::command]
#[specta::specta]
pub async fn auth_login(
    app_handle: AppHandle,
    login: String,
    password: String,
) -> Result<(), String> {
    let auth_manager = resolve_auth_manager(&app_handle)?;

    auth_manager
        .login(&login, &password)
        .await
        .ctx("Login failed")?;

    save_settings(&app_handle).await
}

#[tauri::command]
#[specta::specta]
pub async fn auth_register(
    app_handle: AppHandle,
    email: String,
    password: String,
) -> Result<(), String> {
    let auth_manager = resolve_auth_manager(&app_handle)?;

    auth_manager
        .register(&email, &password)
        .await
        .ctx("Registration failed")?;

    save_settings(&app_handle).await
}

#[tauri::command]
#[specta::specta]
pub async fn auth_logout(app_handle: AppHandle) -> Result<(), String> {
    let auth_manager = resolve_auth_manager(&app_handle)?;
    auth_manager.logout().await;
    save_settings(&app_handle).await
}

#[tauri::command]
#[specta::specta]
pub async fn auth_is_authenticated(app_handle: AppHandle) -> Result<bool, String> {
    use backon::{ConstantBuilder, Retryable};

    // Mobile fires IPC calls as soon as the WebView paints, which can
    // race the `install` step in `setup`. Retry the state lookup for a
    // few seconds before giving up — same shape as the desktop
    // procedure, just relevant here because the cold-launch window is
    // larger on mobile.
    let manager = (|| async { auth_manager(&app_handle).ok_or("auth manager not yet available") })
        .retry(
            ConstantBuilder::default()
                .with_delay(std::time::Duration::from_millis(100))
                .with_max_times(50),
        )
        .sleep(tokio::time::sleep)
        .await;

    let Ok(auth_manager) = manager else {
        return Ok(false);
    };

    match auth_manager.get_or_refresh_access_token().await {
        Ok(token) => Ok(!token.expose_secret().is_empty()),
        // Definitively logged out — surface as `false` so the frontend
        // shows the login screen.
        Err(e) if e.is_logged_out() => Ok(false),
        // Transient failure (server unreachable etc.) — local credentials
        // are intact. Don't log the user out on connectivity blips; trust
        // the last-known state if we have any token stored.
        Err(e) => {
            tracing::warn!(
                "is_authenticated: transient auth error, assuming last-known state: {e}"
            );
            Ok(auth_manager.current_claims().is_some())
        }
    }
}

#[tauri::command]
#[specta::specta]
pub async fn auth_get_access_token_payload(app_handle: AppHandle) -> Result<Claims, String> {
    let auth_manager = resolve_auth_manager(&app_handle)?;
    auth_manager
        .get_or_refresh_access_token()
        .await
        .ctx("Failed to get access token")?;
    auth_manager
        .get_access_token_payload()
        .ctx("Failed to get access token payload")
}

#[tauri::command]
#[specta::specta]
pub async fn auth_refresh_session(app_handle: AppHandle) -> Result<(), String> {
    let auth_manager = resolve_auth_manager(&app_handle)?;
    auth_manager
        .refresh_tokens()
        .await
        .ctx("Failed to refresh session")?;

    Ok(())
}
