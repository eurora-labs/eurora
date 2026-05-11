# tauri-plugin-appauth

A Tauri 2 mobile plugin that bridges OAuth 2.0 / OIDC flows to the OpenID
Foundation's [AppAuth-iOS](https://github.com/openid/AppAuth-iOS) and
[AppAuth-Android](https://github.com/openid/AppAuth-Android) reference
clients. Discovery, dynamic client registration, PKCE-secured authorization,
token refresh, and RP-initiated end-session are exposed as a single typed
TypeScript + Rust API.

## Platform support

| Platform | Status |
|---|---|
| iOS 15+ | Supported via AppAuth-iOS |
| Android 24+ | Supported via AppAuth-Android |
| Desktop | Returns `UNSUPPORTED_PLATFORM`; use [`tauri-plugin-oauth`](https://github.com/FabianLars/tauri-plugin-oauth) instead. |

## Install

Cargo:

```toml
[dependencies]
tauri-plugin-appauth = "0.2"
```

npm / bun:

```sh
bun add @eurora-labs/tauri-plugin-appauth
# or
npm install @eurora-labs/tauri-plugin-appauth
```

Register the plugin in your Tauri app's mobile entry point:

```rust
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_appauth::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

## Configure

### Android

Declare the OAuth redirect scheme as a manifest placeholder in your app's
`build.gradle.kts`. AppAuth's redirect-receiver activity is merged in
automatically; you do not need to register one yourself.

```kotlin
android {
    defaultConfig {
        manifestPlaceholders["appAuthRedirectScheme"] = "com.example.app"
    }
}
```

### iOS

Add your custom URL scheme to `Info.plist` if you use one, and ensure the
Associated Domains entitlement is set if you use Universal Links for the
redirect.

## Permissions

Add the plugin's default permission to your capabilities:

```json
{
  "permissions": ["appauth:default"]
}
```

The default permission set exposes `discover`, `authorize`,
`authorizeBrowserOnly`, `refresh`, and `endSession`. The more sensitive
`register` (RFC 7591 Dynamic Client Registration) ships with the plugin but
is **not** included in the default set — opt in explicitly with
`appauth:allow-register` when you need it.

## Quickstart

```ts
import { authorize, AppAuthError } from '@eurora-labs/tauri-plugin-appauth';

try {
  const auth = await authorize({
    config: { kind: 'discovery', issuer: 'https://accounts.google.com' },
    clientId: '...apps.googleusercontent.com',
    redirectUri: 'com.example.app:/oauth/callback',
    scopes: ['openid', 'email', 'profile'],
  });
  // auth.accessToken, auth.idToken, auth.refreshToken
} catch (e) {
  if (e instanceof AppAuthError && e.code === 'USER_CANCELED') return;
  throw e;
}
```

For backend-mediated flows that only need the browser leg, `authorizeBrowserOnly`
returns the redirect URL without performing the code-for-token exchange.

## Error reference

Errors are normalized to a stable enum derived from AppAuth's `OIDErrorCode`
(iOS) and `AuthorizationException` categories (Android):

| Code | Meaning |
|---|---|
| `USER_CANCELED` | The user dismissed the in-app browser. |
| `AUTHORIZATION_FAILED` | The authorization endpoint returned an error. |
| `TOKEN_EXCHANGE_FAILED` | Code-for-token exchange failed. |
| `NETWORK_ERROR` | Underlying transport failure. |
| `INVALID_REGISTRATION_RESPONSE` | DCR response was malformed. |
| `ID_TOKEN_VALIDATION_FAILED` | OIDC ID token signature/claims rejected. |
| `BROWSER_NOT_AVAILABLE` | No Custom Tabs / `ASWebAuthenticationSession` available. |
| `INVALID_REQUEST` | The plugin received malformed inputs. |
| `SERVER_ERROR` | Token endpoint returned a 5xx. |
| `UNSUPPORTED_PLATFORM` | Called on desktop. |
| `PLUGIN_INVOKE_FAILED` | IPC bridge failed before the native side ran. |

## Developing

This crate lives in the [eurora monorepo](https://github.com/eurora-labs/eurora)
at `crates/plugins/tauri-plugin-appauth/`. Standard workspace tooling:

```sh
# Rust
cargo check -p tauri-plugin-appauth
cargo test -p tauri-plugin-appauth
cargo clippy -p tauri-plugin-appauth

# TypeScript bindings (requires bun)
pnpm --filter @eurora-labs/tauri-plugin-appauth build
```

`cargo build` regenerates `android/.tauri/tauri-api/` from the Tauri version
pinned in the workspace `Cargo.toml`. That directory is gitignored.

`api-iife.js` and `dist-js/` are the npm publish artifacts; `dist-js/` is
gitignored and produced by `pnpm --filter @eurora-labs/tauri-plugin-appauth build`.
`api-iife.js` is checked in so the npm tarball stays self-contained without
forcing a build step on consumers.

## License

Apache-2.0. See [LICENSE](LICENSE).
