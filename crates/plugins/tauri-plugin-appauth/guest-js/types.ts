/**
 * Public type surface for `@eurora-labs/tauri-plugin-appauth`.
 *
 * Mirrors the Rust models in `src/models.rs` 1:1. `serde(rename_all =
 * "camelCase")` on the Rust side means every wire field is camelCase here.
 */

/**
 * Where to source the authorization server's endpoints.
 *
 * `discovery` hits `<issuer>/.well-known/openid-configuration` (RFC 8414 /
 * OIDC); `explicit` skips discovery for providers that don't publish a
 * document or for tests that want full control.
 */
export type ConfigSource =
    | { kind: 'discovery'; issuer: string }
    | {
          kind: 'explicit';
          authorizationEndpoint: string;
          tokenEndpoint: string;
          endSessionEndpoint?: string;
          registrationEndpoint?: string;
      };

export interface ServiceConfiguration {
    authorizationEndpoint: string;
    tokenEndpoint: string;
    endSessionEndpoint?: string;
    registrationEndpoint?: string;
    issuer?: string;
    additionalParameters?: Record<string, unknown>;
}

export interface DiscoverRequest {
    issuer: string;
}

/** OIDC `prompt` parameter values (RFC 6749 / OIDC Core §3.1.2.1). */
export type Prompt = 'login' | 'consent' | 'select_account' | 'none';

export interface AuthorizeRequest {
    config: ConfigSource;
    clientId: string;
    /**
     * Custom-scheme URI (e.g. `com.example.app:/oauth/callback`) or HTTPS
     * Universal Link / App Link. AppAuth validates that the redirect handler
     * is registered with the OS before opening the browser.
     *
     * Must include either a host or a path beginning with `/`. Bare schemes
     * (e.g. `com.example:`) are rejected with `INVALID_REQUEST`.
     */
    redirectUri: string;
    scopes?: string[];
    additionalParameters?: Record<string, string>;
    prompt?: Prompt;
    loginHint?: string;
    uiLocales?: string[];
    /**
     * iOS-only hint forwarded to `ASWebAuthenticationSession`. Ignored on
     * Android (Custom Tabs always shares cookies with the user's default
     * browser). Defaults to `true`.
     */
    prefersEphemeralSession?: boolean;
    /**
     * Whether AppAuth should generate and validate an OIDC `nonce`. Defaults
     * to `true` on every platform; set to `false` to opt out for non-OIDC
     * providers that reject the parameter. OIDC requires the nonce defense for
     * the `code` flow, so the default is `true` regardless of the requested
     * scopes.
     */
    useNonce?: boolean;
}

export interface AuthState {
    accessToken?: string;
    /** Unix seconds at which `accessToken` expires. */
    accessTokenExpiresAt?: number;
    idToken?: string;
    refreshToken?: string;
    scope?: string;
    tokenType?: string;
    /** Surfaced for backend-mediated flows that exchange the code themselves. */
    authorizationCode?: string;
    additionalParameters?: Record<string, unknown>;
}

export interface BrowserOnlyRequest {
    /**
     * Fully-built authorization URL. The plugin opens the browser at this URL
     * and waits for the OS to intercept `redirectUri`.
     */
    authUrl: string;
    /**
     * Custom-scheme URI (e.g. `com.example.app:/oauth/callback`) or HTTPS
     * Universal Link.
     *
     * On iOS, HTTPS redirects are routed through
     * `ASWebAuthenticationSession`'s Universal Link callback, which requires
     * **iOS 17.4 or later**. Older iOS versions reject HTTPS redirects with
     * `INVALID_REQUEST` — fall back to a custom scheme to support them.
     *
     * Must include either a host or a path beginning with `/`.
     */
    redirectUri: string;
    prefersEphemeralSession?: boolean;
}

export interface BrowserOnlyResponse {
    /**
     * Full callback URL the system intercepted, with all query parameters
     * from the authorization server intact.
     */
    url: string;
}

export interface RefreshRequest {
    config: ConfigSource;
    clientId: string;
    refreshToken: string;
    /** Optionally narrow the requested scopes (RFC 6749 §6). */
    scopes?: string[];
    additionalParameters?: Record<string, string>;
}

export interface RegisterRequest {
    config: ConfigSource;
    redirectUris: string[];
    clientName?: string;
    responseTypes?: string[];
    grantTypes?: string[];
    /**
     * OIDC `subject_types` the client supports.
     *
     * The OIDC discovery field is plural at the metadata level
     * (`subject_types_supported`), but a registration commits to exactly one
     * value — and the underlying AppAuth-iOS / AppAuth-Android APIs expose it
     * as a single string. Pass at most one entry; the plugin rejects payloads
     * with more than one value as `INVALID_REQUEST` rather than silently
     * dropping the rest.
     */
    subjectTypes?: string[];
    tokenEndpointAuthMethod?: string;
    additionalParameters?: Record<string, unknown>;
}

export interface RegistrationResponse {
    clientId: string;
    clientIdIssuedAt?: number;
    clientSecret?: string;
    clientSecretExpiresAt?: number;
    registrationAccessToken?: string;
    registrationClientUri?: string;
    tokenEndpointAuthMethod?: string;
    additionalParameters?: Record<string, unknown>;
}

export interface EndSessionRequest {
    config: ConfigSource;
    idTokenHint: string;
    postLogoutRedirectUri: string;
    state?: string;
    additionalParameters?: Record<string, string>;
    prefersEphemeralSession?: boolean;
}

export interface EndSessionResponse {
    url: string;
    state?: string;
}

/**
 * Diagnostic events emitted by the native AppAuth runtime as a flow
 * progresses. Subscribe via {@link onAuthEvent} to receive them.
 */
export type AuthEvent =
    | { kind: 'browserOpened' }
    | { kind: 'redirectIntercepted' }
    | { kind: 'tokenExchangeStarted' }
    | { kind: 'tokenExchangeCompleted' };

/**
 * Stable error codes mirroring AppAuth's iOS `OIDErrorCode` and Android
 * `AuthorizationException` categories. Switch on these instead of parsing
 * free-form messages.
 */
export type AppAuthErrorCode =
    | 'USER_CANCELED'
    | 'AUTHORIZATION_FAILED'
    | 'TOKEN_EXCHANGE_FAILED'
    | 'NETWORK_ERROR'
    | 'INVALID_REGISTRATION_RESPONSE'
    | 'ID_TOKEN_VALIDATION_FAILED'
    | 'BROWSER_NOT_AVAILABLE'
    | 'INVALID_REQUEST'
    | 'SERVER_ERROR'
    | 'UNSUPPORTED_PLATFORM'
    | 'PLUGIN_INVOKE_FAILED';

interface AppAuthErrorOptions {
    oauthError?: string;
    oauthErrorDescription?: string;
    cause?: unknown;
}

/**
 * Thrown by every command in this package when the underlying `invoke`
 * call rejects. Inspect {@link AppAuthError.code} to branch on the failure
 * mode; the optional {@link AppAuthError.oauthError} carries the OAuth
 * error name (e.g. `invalid_grant`) when the issuer returned one.
 */
export class AppAuthError extends Error {
    readonly code: AppAuthErrorCode;
    readonly oauthError?: string;
    readonly oauthErrorDescription?: string;

    constructor(code: AppAuthErrorCode, message: string, options: AppAuthErrorOptions = {}) {
        super(message, options.cause !== undefined ? { cause: options.cause } : undefined);
        this.name = 'AppAuthError';
        this.code = code;
        this.oauthError = options.oauthError;
        this.oauthErrorDescription = options.oauthErrorDescription;
        // Preserve `instanceof AppAuthError` after Babel/tsc downlevel transforms.
        Object.setPrototypeOf(this, AppAuthError.prototype);
    }

    /**
     * Normalise an unknown rejection from `invoke` into an `AppAuthError`.
     * Accepts the plugin's structured error payload, plain `Error`s, and
     * strings — falls back to `PLUGIN_INVOKE_FAILED` when nothing matches.
     */
    static from(value: unknown): AppAuthError {
        if (value instanceof AppAuthError) {
            return value;
        }

        if (typeof value === 'object' && value !== null) {
            const raw = value as {
                code?: unknown;
                message?: unknown;
                oauth_error?: unknown;
                oauthError?: unknown;
                oauth_error_description?: unknown;
                oauthErrorDescription?: unknown;
            };
            if (typeof raw.code === 'string' && typeof raw.message === 'string') {
                const oauthError = pickString(raw.oauth_error, raw.oauthError);
                const oauthErrorDescription = pickString(
                    raw.oauth_error_description,
                    raw.oauthErrorDescription,
                );
                return new AppAuthError(raw.code as AppAuthErrorCode, raw.message, {
                    oauthError,
                    oauthErrorDescription,
                    cause: value,
                });
            }
        }

        if (value instanceof Error) {
            return new AppAuthError('PLUGIN_INVOKE_FAILED', value.message, { cause: value });
        }

        if (typeof value === 'string') {
            return new AppAuthError('PLUGIN_INVOKE_FAILED', value);
        }

        return new AppAuthError('PLUGIN_INVOKE_FAILED', 'Unknown plugin error', { cause: value });
    }
}

function pickString(...candidates: unknown[]): string | undefined {
    for (const candidate of candidates) {
        if (typeof candidate === 'string') {
            return candidate;
        }
    }
    return undefined;
}
