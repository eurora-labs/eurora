import { browser } from '$app/environment';
import { ApiClient, ApiError, setSessionExpiredHandler } from '$lib/api/client.js';
import { consumeAppRedirectUri } from '$lib/auth/redirect-uri';
import { InjectionToken } from '@eurora/shared/context';
import * as Sentry from '@sentry/sveltekit';
import type {
	AssociateLoginTokenRequest,
	CheckEmailRequest,
	CheckEmailResponse,
	LoginRequest,
	Provider,
	RegisterRequest,
	ThirdPartyAuthUrlRequest,
	ThirdPartyAuthUrlResponse,
	UserInfo,
	UserResponse,
	VerifyEmailRequest,
} from '@eurora/shared/bindings/auth';
import type { ConfigService } from '@eurora/shared/config/config-service';

export type OAuthProvider = Provider;

export type User = UserInfo;

export type CheckEmailResult =
	| { status: 'oauth'; provider: OAuthProvider }
	| { status: 'not_found' }
	| { status: 'password' };

/**
 * Browser-side auth state.
 *
 * The session lives entirely in `HttpOnly` cookies set by the backend
 * (`eu_access`, `eu_refresh`); this class never touches token values
 * directly. The SPA only needs to know "who is logged in" — that
 * comes from `GET /auth/me` on boot and from the `UserResponse` body
 * returned by the session-minting endpoints.
 */
export class AuthService {
	isAuthenticated = $state(false);
	user = $state<User | null>(null);
	/** Resolves once the initial `/auth/me` probe has settled. */
	ready = $state<Promise<void>>(Promise.resolve());

	readonly #api: ApiClient;

	constructor(config: ConfigService) {
		this.#api = new ApiClient(config);
		setSessionExpiredHandler(() => this.#clearLocal());
		if (browser) this.ready = this.#hydrate();
	}

	async login(email: string, password: string): Promise<void> {
		const body: LoginRequest = { kind: 'email_password', login: email, password };
		const resp = await this.#api.fetch<UserResponse, LoginRequest>('/auth/login', { body });
		this.#setSession(resp.user);
	}

	async register(email: string, password: string, displayName?: string): Promise<void> {
		const body: RegisterRequest = {
			email,
			password,
			display_name: displayName ?? null,
		};
		const resp = await this.#api.fetch<UserResponse, RegisterRequest>('/auth/register', {
			body,
		});
		this.#setSession(resp.user);
	}

	async loginWithOAuth(
		provider: OAuthProvider,
		code: string,
		state: string,
		opts?: { loginToken?: string },
	): Promise<void> {
		const body: LoginRequest = {
			kind: 'third_party',
			provider,
			code,
			state,
			login_token: opts?.loginToken ?? null,
		};
		const resp = await this.#api.fetch<UserResponse, LoginRequest>('/auth/login', { body });
		this.#setSession(resp.user);
	}

	async verifyEmail(token: string): Promise<void> {
		const body: VerifyEmailRequest = { token };
		const resp = await this.#api.fetch<UserResponse, VerifyEmailRequest>('/auth/email/verify', {
			body,
		});
		this.#setSession(resp.user);
	}

	async resendVerificationEmail(): Promise<void> {
		await this.#api.fetch<void>('/auth/email/resend-verification', { method: 'POST' });
	}

	async checkEmail(email: string): Promise<CheckEmailResult> {
		const body: CheckEmailRequest = { email };
		const resp = await this.#api.fetch<CheckEmailResponse, CheckEmailRequest>(
			'/auth/email/check',
			{ body },
		);
		switch (resp.status) {
			case 'oauth': {
				if (!resp.provider) {
					throw new Error('Auth service returned `oauth` status without a provider');
				}
				return { status: 'oauth', provider: resp.provider };
			}
			case 'not_found':
				return { status: 'not_found' };
			case 'password':
				return { status: 'password' };
		}
	}

	async getOAuthRedirectUrl(provider: OAuthProvider): Promise<string> {
		const body: ThirdPartyAuthUrlRequest = { provider };
		const resp = await this.#api.fetch<ThirdPartyAuthUrlResponse, ThirdPartyAuthUrlRequest>(
			'/auth/oauth/url',
			{ body },
		);
		return resp.url;
	}

	async associateAppLogin(loginToken: string): Promise<void> {
		const body: AssociateLoginTokenRequest = { code_challenge: loginToken };
		await this.#api.fetch<void, AssociateLoginTokenRequest>('/auth/login-token/associate', {
			body,
		});
		if (browser) {
			sessionStorage.removeItem('loginToken');
			sessionStorage.removeItem('challengeMethod');
		}
	}

	async associateAppLoginIfPending(opts: { consumeRedirect?: boolean } = {}): Promise<boolean> {
		if (!browser) return false;
		const loginToken = sessionStorage.getItem('loginToken');
		if (!loginToken) return false;

		try {
			await this.associateAppLogin(loginToken);
		} catch (err) {
			// Capture to Sentry plus a console error: the native swallow
			// here used to make a backend-rejected associate look like a
			// silent no-op in the desktop polling loop, which sent us
			// debugging the wrong layer for an afternoon.
			console.error('[auth.associate-app] associate failed', err);
			Sentry.captureException(err, { tags: { area: 'auth.associate-app' } });
			return false;
		}

		if (opts.consumeRedirect) {
			const redirectUri = consumeAppRedirectUri();
			if (redirectUri) {
				window.location.href = redirectUri;
			}
		}
		return true;
	}

	/**
	 * Sign the user out of this device. Calls the backend so the
	 * refresh token is revoked server-side and `Set-Cookie` headers
	 * clear the auth cookies; falls through to clearing local state
	 * even if the network call fails so the UI never gets stuck.
	 */
	async logout(): Promise<void> {
		try {
			await this.#api.fetch<void>('/auth/logout', { method: 'POST', skipAuthRefresh: true });
		} catch (err) {
			if (!(err instanceof ApiError) || err.status !== 401) {
				Sentry.captureException(err, { tags: { area: 'auth.logout' } });
			}
		} finally {
			this.#clearLocal();
		}
	}

	async #hydrate(): Promise<void> {
		try {
			const resp = await this.#api.fetch<UserResponse>('/auth/me', {
				skipAuthRefresh: false,
			});
			this.#setSession(resp.user);
		} catch (err) {
			if (err instanceof ApiError && err.status === 401) {
				this.#clearLocal();
				return;
			}
			Sentry.captureException(err, { tags: { area: 'auth.init' } });
			this.#clearLocal();
		}
	}

	#setSession(user: User): void {
		Sentry.setUser({ id: user.id });
		this.user = user;
		this.isAuthenticated = true;
	}

	#clearLocal(): void {
		Sentry.setUser(null);
		this.user = null;
		this.isAuthenticated = false;
	}
}

export const AUTH_SERVICE = new InjectionToken<AuthService>('AuthService');
