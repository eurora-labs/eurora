/**
 * `@eurora-labs/tauri-plugin-appauth` — Tauri 2 mobile bridge over AppAuth-iOS
 * and AppAuth-Android. PKCE, `state`/`nonce`, discovery, refresh, and
 * end-session are handled by AppAuth on each platform; this package is the
 * typed JS surface.
 *
 * Desktop targets reject every command with `UNSUPPORTED_PLATFORM` — desktop
 * OAuth has its own canonical plugin (`tauri-plugin-oauth`).
 */

export {
    authorize,
    authorizeBrowserOnly,
    discover,
    endSession,
    refresh,
    register,
} from './commands';

export { onAuthEvent } from './events';
export type { AuthEventHandler, Unsubscribe } from './events';

export { AppAuthError } from './types';
export type {
    AppAuthErrorCode,
    AuthEvent,
    AuthState,
    AuthorizeRequest,
    BrowserOnlyRequest,
    BrowserOnlyResponse,
    ConfigSource,
    DiscoverRequest,
    EndSessionRequest,
    EndSessionResponse,
    Prompt,
    RefreshRequest,
    RegisterRequest,
    RegistrationResponse,
    ServiceConfiguration,
} from './types';
