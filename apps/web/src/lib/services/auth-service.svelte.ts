import { browser } from '$app/environment';
import { create } from '@bufbuild/protobuf';
import { createClient, type Client } from '@connectrpc/connect';
import { createGrpcWebTransport } from '@connectrpc/connect-web';
import { InjectionToken } from '@eurora/shared/context';
import {
	AssociateLoginTokenRequestSchema,
	CheckEmailRequestSchema,
	LoginRequestSchema,
	ProtoAuthService,
	Provider,
	RefreshTokenRequestSchema,
	RegisterRequestSchema,
	VerifyEmailRequestSchema,
	type TokenResponse,
} from '@eurora/shared/proto/auth_service_pb.js';
import * as Sentry from '@sentry/sveltekit';
import type { ConfigService } from '@eurora/shared/config/config-service';

export type OAuthProvider = 'google' | 'github';

export interface User {
	id: string;
	email: string;
	name?: string;
	avatar?: string;
}

export type CheckEmailResult =
	| { status: 'oauth'; provider: OAuthProvider }
	| { status: 'not_found' }
	| { status: 'exists' };

const COOKIE_KEYS = {
	ACCESS_TOKEN: 'eurora_access_token',
	REFRESH_TOKEN: 'eurora_refresh_token',
	EXPIRES_AT: 'eurora_expires_at',
	USER: 'eurora_user',
} as const;

const REFRESH_LEEWAY_MS = 5 * 60 * 1000;

export class AuthService {
	isAuthenticated = $state(false);
	user = $state<User | null>(null);
	accessToken = $state<string | null>(null);

	readonly #config: ConfigService;
	#refreshTokenValue: string | null = null;
	#expiresAt: number | null = null;
	#client: Client<typeof ProtoAuthService> | null = null;
	#refreshInflight: Promise<void> | null = null;

	constructor(config: ConfigService) {
		this.#config = config;
		if (browser) this.#hydrateFromCookies();
	}

	async login(email: string, password: string): Promise<void> {
		const tokens = await this.#grpc.login(
			create(LoginRequestSchema, {
				credential: {
					case: 'emailPassword',
					value: { login: email, password },
				},
			}),
		);
		this.#setSession(tokens);
	}

	async register(email: string, password: string, displayName?: string): Promise<void> {
		const tokens = await this.#grpc.register(
			create(RegisterRequestSchema, { email, password, displayName }),
		);
		this.#setSession(tokens);
	}

	async loginWithOAuth(
		provider: OAuthProvider,
		code: string,
		state: string,
		opts?: { loginToken?: string; challengeMethod?: string },
	): Promise<void> {
		const tokens = await this.#grpc.login(
			create(LoginRequestSchema, {
				credential: {
					case: 'thirdParty',
					value: {
						provider: toProtoProvider(provider),
						code,
						state,
						loginToken: opts?.loginToken,
						challengeMethod: opts?.challengeMethod,
					},
				},
			}),
		);
		this.#setSession(tokens);
	}

	async verifyEmail(token: string): Promise<void> {
		const tokens = await this.#grpc.verifyEmail(create(VerifyEmailRequestSchema, { token }));
		this.#setSession(tokens);
	}

	async checkEmail(email: string): Promise<CheckEmailResult> {
		const resp = await this.#grpc.checkEmail(create(CheckEmailRequestSchema, { email }));
		if (resp.status === 'oauth') {
			const provider = fromProtoProvider(resp.provider);
			if (!provider) {
				throw new Error('Unknown OAuth provider returned for email check');
			}
			return { status: 'oauth', provider };
		}
		if (resp.status === 'not_found') return { status: 'not_found' };
		return { status: 'exists' };
	}

	async getOAuthRedirectUrl(provider: OAuthProvider): Promise<string> {
		const resp = await this.#grpc.getThirdPartyAuthUrl({ provider: toProtoProvider(provider) });
		return resp.url;
	}

	async ensureValidToken(): Promise<boolean> {
		if (!this.isAuthenticated || !this.#expiresAt) return false;
		if (this.#expiresAt > Date.now() + REFRESH_LEEWAY_MS) return true;

		this.#refreshInflight ??= this.#refresh().finally(() => {
			this.#refreshInflight = null;
		});
		try {
			await this.#refreshInflight;
			return true;
		} catch {
			return false;
		}
	}

	async associateDesktopLogin(loginToken: string): Promise<void> {
		const accessToken = this.accessToken;
		if (!accessToken) {
			throw new Error('Cannot associate desktop login without an active session');
		}
		await this.#grpc.associateLoginToken(
			create(AssociateLoginTokenRequestSchema, { codeChallenge: loginToken }),
			{ headers: new Headers({ authorization: `Bearer ${accessToken}` }) },
		);
		if (browser) {
			sessionStorage.removeItem('loginToken');
			sessionStorage.removeItem('challengeMethod');
		}
	}

	async associateDesktopLoginIfPending(
		opts: { consumeRedirect?: boolean } = {},
	): Promise<boolean> {
		if (!browser) return false;
		const loginToken = sessionStorage.getItem('loginToken');
		if (!loginToken) return false;

		try {
			await this.associateDesktopLogin(loginToken);
		} catch (err) {
			Sentry.captureException(err, { tags: { area: 'auth.associate-desktop' } });
			return false;
		}

		if (opts.consumeRedirect) {
			const redirectUri = sessionStorage.getItem('deviceRedirectUri');
			if (redirectUri) {
				sessionStorage.removeItem('deviceRedirectUri');
				window.location.href = redirectUri;
			}
		}
		return true;
	}

	logout(): void {
		this.#clearCookies();
		Sentry.setUser(null);
		this.isAuthenticated = false;
		this.user = null;
		this.accessToken = null;
		this.#refreshTokenValue = null;
		this.#expiresAt = null;
	}

	get #grpc(): Client<typeof ProtoAuthService> {
		this.#client ??= createClient(
			ProtoAuthService,
			createGrpcWebTransport({
				baseUrl: this.#config.grpcApiUrl,
				useBinaryFormat: true,
			}),
		);
		return this.#client;
	}

	async #refresh(): Promise<void> {
		if (!this.#refreshTokenValue) {
			throw new Error('No refresh token available');
		}
		try {
			const tokens = await this.#grpc.refreshToken(create(RefreshTokenRequestSchema, {}));
			if (!this.user) {
				throw new Error('Cannot refresh session without a user');
			}
			this.#writeCookies(tokens, this.user);
			this.accessToken = tokens.accessToken;
			this.#refreshTokenValue = tokens.refreshToken;
			this.#expiresAt = Date.now() + Number(tokens.expiresIn) * 1000;
		} catch (err) {
			Sentry.captureException(err, { tags: { area: 'auth.refresh' } });
			this.logout();
			throw err;
		}
	}

	#setSession(tokens: TokenResponse): void {
		const claims = decodeJwtPayload(tokens.accessToken);
		if (!claims) {
			throw new Error('Invalid access token');
		}
		const user: User = {
			id: claims.sub ?? claims.user_id ?? 'unknown',
			email: claims.email ?? 'unknown@example.com',
			name: claims.name ?? claims.email,
			avatar: claims.avatar ?? claims.picture,
		};
		const expiresAt = Date.now() + Number(tokens.expiresIn) * 1000;

		this.#writeCookies(tokens, user);
		Sentry.setUser({ id: user.id });

		this.isAuthenticated = true;
		this.user = user;
		this.accessToken = tokens.accessToken;
		this.#refreshTokenValue = tokens.refreshToken;
		this.#expiresAt = expiresAt;
	}

	#hydrateFromCookies(): void {
		try {
			const accessToken = readCookie(COOKIE_KEYS.ACCESS_TOKEN);
			const refreshToken = readCookie(COOKIE_KEYS.REFRESH_TOKEN);
			const expiresAtStr = readCookie(COOKIE_KEYS.EXPIRES_AT);
			const userStr = readCookie(COOKIE_KEYS.USER);

			if (!refreshToken || !expiresAtStr || !userStr) return;

			const user = JSON.parse(decodeURIComponent(userStr)) as User;
			const expiresAt = Number.parseInt(expiresAtStr, 10);
			const accessFresh = !!accessToken && expiresAt > Date.now() + REFRESH_LEEWAY_MS;

			Sentry.setUser({ id: user.id });

			this.isAuthenticated = true;
			this.user = user;
			this.accessToken = accessFresh ? accessToken : null;
			this.#refreshTokenValue = refreshToken;
			this.#expiresAt = expiresAt;
		} catch (err) {
			Sentry.captureException(err, { tags: { area: 'auth.init' } });
		}
	}

	#writeCookies(tokens: TokenResponse, user: User): void {
		if (!browser) return;
		const expiresAt = Date.now() + Number(tokens.expiresIn) * 1000;
		const accessMaxAge = Number(tokens.expiresIn);
		const sessionMaxAge = accessMaxAge * 10;

		writeCookie(COOKIE_KEYS.ACCESS_TOKEN, tokens.accessToken, accessMaxAge);
		writeCookie(COOKIE_KEYS.REFRESH_TOKEN, tokens.refreshToken, sessionMaxAge);
		writeCookie(COOKIE_KEYS.EXPIRES_AT, expiresAt.toString(), sessionMaxAge);
		writeCookie(COOKIE_KEYS.USER, encodeURIComponent(JSON.stringify(user)), sessionMaxAge);
	}

	#clearCookies(): void {
		if (!browser) return;
		for (const name of Object.values(COOKIE_KEYS)) deleteCookie(name);
	}
}

export const AUTH_SERVICE = new InjectionToken<AuthService>('AuthService');

function toProtoProvider(provider: OAuthProvider): Provider {
	return provider === 'google' ? Provider.GOOGLE : Provider.GITHUB;
}

function fromProtoProvider(provider: Provider | undefined): OAuthProvider | undefined {
	if (provider === Provider.GOOGLE) return 'google';
	if (provider === Provider.GITHUB) return 'github';
	return undefined;
}

function writeCookie(name: string, value: string, maxAgeSec: number): void {
	const secure = location.protocol === 'https:' ? '; secure' : '';
	document.cookie = `${name}=${value}; path=/; max-age=${maxAgeSec}; samesite=lax${secure}`;
}

function deleteCookie(name: string): void {
	document.cookie = `${name}=; path=/; max-age=0`;
}

function readCookie(name: string): string | null {
	const escaped = name.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
	const match = document.cookie.match(new RegExp(`(?:^|; )${escaped}=([^;]*)`));
	return match ? match[1] : null;
}

function decodeJwtPayload(token: string): Record<string, string> | null {
	try {
		const base64Url = token.split('.')[1];
		const base64 = base64Url.replace(/-/g, '+').replace(/_/g, '/');
		const json = decodeURIComponent(
			atob(base64)
				.split('')
				.map((c) => '%' + ('00' + c.charCodeAt(0).toString(16)).slice(-2))
				.join(''),
		);
		return JSON.parse(json);
	} catch (err) {
		Sentry.captureException(err, { tags: { area: 'auth.jwt-decode' } });
		return null;
	}
}
