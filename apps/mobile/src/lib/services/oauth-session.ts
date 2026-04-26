import { invoke } from '@tauri-apps/api/core';

export type OAuthSessionErrorCode =
	| 'USER_CANCELED'
	| 'PRESENTATION_FAILED'
	| 'SESSION_FAILED'
	| 'SESSION_BUSY'
	| 'INVALID_AUTH_URL'
	| 'INVALID_CALLBACK_SCHEME'
	| 'UNSUPPORTED_PLATFORM'
	| 'PLUGIN_INVOKE_FAILED';

export class OAuthSessionError extends Error {
	readonly code: OAuthSessionErrorCode;

	constructor(code: OAuthSessionErrorCode, message: string) {
		super(message);
		this.code = code;
		this.name = 'OAuthSessionError';
	}
}

interface AuthenticateOptions {
	authUrl: string;
	callbackScheme: string;
	prefersEphemeralSession?: boolean;
}

interface AuthenticateResponse {
	url: string;
}

/**
 * Open an Apple-managed in-app browser via ASWebAuthenticationSession (iOS) and
 * resolve with the redirect URL the system intercepted from `callbackScheme`.
 *
 * The session is ephemeral by default — no cookies are shared with Safari and
 * iOS does not show the "wants to use X to sign in" consent prompt.
 */
export async function authenticateOAuthSession(
	options: AuthenticateOptions,
): Promise<AuthenticateResponse> {
	try {
		return await invoke<AuthenticateResponse>('plugin:oauth-session|authenticate', {
			options,
		});
	} catch (raw) {
		throw normalizeError(raw);
	}
}

function normalizeError(raw: unknown): OAuthSessionError {
	if (raw && typeof raw === 'object') {
		const obj = raw as { code?: unknown; message?: unknown };
		const code =
			typeof obj.code === 'string' ? (obj.code as OAuthSessionErrorCode) : 'SESSION_FAILED';
		const message = typeof obj.message === 'string' ? obj.message : String(raw);
		return new OAuthSessionError(code, message);
	}
	return new OAuthSessionError('SESSION_FAILED', String(raw));
}
