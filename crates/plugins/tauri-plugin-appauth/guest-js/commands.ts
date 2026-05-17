import { invoke } from '@tauri-apps/api/core';

import { AppAuthError } from './types';
import type {
    AuthorizeRequest,
    AuthState,
    BrowserOnlyRequest,
    BrowserOnlyResponse,
    DiscoverRequest,
    EndSessionRequest,
    EndSessionResponse,
    RefreshRequest,
    RegisterRequest,
    RegistrationResponse,
    ServiceConfiguration,
} from './types';

const PLUGIN = 'plugin:appauth';

async function call<T>(command: string, args: Record<string, unknown>): Promise<T> {
    try {
        return await invoke<T>(`${PLUGIN}|${command}`, args);
    } catch (error) {
        throw AppAuthError.from(error);
    }
}

/**
 * Resolve `<issuer>/.well-known/openid-configuration` (RFC 8414 / OIDC) into
 * a {@link ServiceConfiguration}. Cacheable.
 */
export function discover(request: DiscoverRequest): Promise<ServiceConfiguration> {
    return call('discover', { payload: request });
}

/**
 * RFC 7591 dynamic client registration. Only providers whose discovery
 * document advertises a `registration_endpoint` support this. Excluded from
 * the plugin's default permission set — host apps must opt in.
 */
export function register(request: RegisterRequest): Promise<RegistrationResponse> {
    return call('register', { payload: request });
}

/**
 * Open the platform browser, run PKCE, validate `state`/`nonce`, and
 * exchange the authorization code for tokens. Resolves with the full
 * post-exchange {@link AuthState}.
 */
export function authorize(request: AuthorizeRequest): Promise<AuthState> {
    return call('authorize', { payload: request });
}

/**
 * Open the browser at `authUrl`, capture the redirect to `redirectUri`, and
 * return the raw callback URL without performing a token exchange. Use this
 * when a backend mediates the code-for-token swap.
 */
export function authorizeBrowserOnly(request: BrowserOnlyRequest): Promise<BrowserOnlyResponse> {
    return call('authorize_browser_only', { payload: request });
}

/**
 * Trade a refresh token for a fresh access token via the issuer's token
 * endpoint.
 */
export function refresh(request: RefreshRequest): Promise<AuthState> {
    return call('refresh', { payload: request });
}

/**
 * RFC 8665 RP-initiated logout. Resolves once the post-logout redirect
 * fires.
 */
export function endSession(request: EndSessionRequest): Promise<EndSessionResponse> {
    return call('end_session', { payload: request });
}
