import { browser } from '$app/environment';
import { AUTH_SERVICE, type AuthService, type TokenResponse } from '$lib/services/auth-service.js';
import { create } from '@bufbuild/protobuf';
import { inject } from '@eurora/shared/context';
import { RefreshTokenRequestSchema } from '@eurora/shared/proto/auth_service_pb.js';
import { writable, derived, get } from 'svelte/store';

export interface User {
	id: string;
	email: string;
	name?: string;
	avatar?: string;
}

export interface AuthState {
	isAuthenticated: boolean;
	user: User | null;
	accessToken: string | null;
	refreshToken: string | null;
	expiresAt: number | null;
}

const COOKIE_KEYS = {
	ACCESS_TOKEN: 'eurora_access_token',
	REFRESH_TOKEN: 'eurora_refresh_token',
	EXPIRES_AT: 'eurora_expires_at',
	USER: 'eurora_user',
} as const;

function getAuthService(): AuthService {
	return inject(AUTH_SERVICE);
}

function setCookie(name: string, value: string, maxAgeSec: number) {
	const secure = location.protocol === 'https:' ? '; secure' : '';
	document.cookie = `${name}=${value}; path=/; max-age=${maxAgeSec}; samesite=lax${secure}`;
}

function deleteCookie(name: string) {
	document.cookie = `${name}=; path=/; max-age=0`;
}

function initializeAuthState(): AuthState {
	if (!browser) {
		return {
			isAuthenticated: false,
			user: null,
			accessToken: null,
			refreshToken: null,
			expiresAt: null,
		};
	}

	try {
		const accessToken = getCookie(COOKIE_KEYS.ACCESS_TOKEN);
		const refreshToken = getCookie(COOKIE_KEYS.REFRESH_TOKEN);
		const expiresAtStr = getCookie(COOKIE_KEYS.EXPIRES_AT);
		const userStr = getCookie(COOKIE_KEYS.USER);

		if (accessToken && refreshToken && expiresAtStr && userStr) {
			const user = JSON.parse(decodeURIComponent(userStr)) as User;
			const expiresAt = parseInt(expiresAtStr, 10);
			const now = Date.now();
			const isValid = expiresAt > now + 5 * 60 * 1000;

			return {
				isAuthenticated: isValid,
				user: isValid ? user : null,
				accessToken: isValid ? accessToken : null,
				refreshToken,
				expiresAt,
			};
		}
	} catch (_error) {
		console.error('Error initializing auth state:', _error);
		clearTokens();
	}

	return {
		isAuthenticated: false,
		user: null,
		accessToken: null,
		refreshToken: null,
		expiresAt: null,
	};
}

function getCookie(name: string): string | null {
	const escaped = name.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
	const match = document.cookie.match(new RegExp(`(?:^|; )${escaped}=([^;]*)`));
	return match ? match[1] : null;
}

const authStore = writable<AuthState>(initializeAuthState());

function clearTokens() {
	if (!browser) return;
	deleteCookie(COOKIE_KEYS.ACCESS_TOKEN);
	deleteCookie(COOKIE_KEYS.REFRESH_TOKEN);
	deleteCookie(COOKIE_KEYS.EXPIRES_AT);
	deleteCookie(COOKIE_KEYS.USER);
}

function storeTokens(tokens: TokenResponse, user: User) {
	if (!browser) return;
	const expiresAt = Date.now() + Number(tokens.expiresIn) * 1000;
	const maxAgeSec = Number(tokens.expiresIn);

	setCookie(COOKIE_KEYS.ACCESS_TOKEN, tokens.accessToken, maxAgeSec);
	setCookie(COOKIE_KEYS.REFRESH_TOKEN, tokens.refreshToken, maxAgeSec * 10);
	setCookie(COOKIE_KEYS.EXPIRES_AT, expiresAt.toString(), maxAgeSec * 10);
	setCookie(COOKIE_KEYS.USER, encodeURIComponent(JSON.stringify(user)), maxAgeSec * 10);
}

function decodeJWTPayload(token: string): any {
	try {
		const base64Url = token.split('.')[1];
		const base64 = base64Url.replace(/-/g, '+').replace(/_/g, '/');
		const jsonPayload = decodeURIComponent(
			atob(base64)
				.split('')
				.map((c) => '%' + ('00' + c.charCodeAt(0).toString(16)).slice(-2))
				.join(''),
		);
		return JSON.parse(jsonPayload);
	} catch (_error) {
		console.error('Error decoding JWT:', _error);
		return null;
	}
}

export const auth = {
	subscribe: authStore.subscribe,

	login: (tokens: TokenResponse) => {
		try {
			const payload = decodeJWTPayload(tokens.accessToken);
			if (!payload) {
				throw new Error('Invalid access token');
			}

			const user: User = {
				id: payload.sub || payload.user_id || 'unknown',
				email: payload.email || 'unknown@example.com',
				name: payload.name || payload.username,
				avatar: payload.avatar || payload.picture,
			};

			storeTokens(tokens, user);

			authStore.set({
				isAuthenticated: true,
				user,
				accessToken: tokens.accessToken,
				refreshToken: tokens.refreshToken,
				expiresAt: Date.now() + Number(tokens.expiresIn) * 1000,
			});
		} catch (error) {
			console.error('Error during login:', error);
			throw error;
		}
	},

	logout: () => {
		clearTokens();
		authStore.set({
			isAuthenticated: false,
			user: null,
			accessToken: null,
			refreshToken: null,
			expiresAt: null,
		});
	},

	refreshToken: async () => {
		const currentState = get(authStore);

		if (!currentState.refreshToken) {
			throw new Error('No refresh token available');
		}

		try {
			const refreshRequest = create(RefreshTokenRequestSchema, {});
			const tokens = await getAuthService().refreshToken(refreshRequest);

			if (currentState.user) {
				storeTokens(tokens, currentState.user);
				authStore.update((state) => ({
					...state,
					accessToken: tokens.accessToken,
					refreshToken: tokens.refreshToken,
					expiresAt: Date.now() + Number(tokens.expiresIn) * 1000,
				}));
			}

			return tokens;
		} catch (error) {
			console.error('Token refresh failed:', error);
			auth.logout();
			throw error;
		}
	},

	ensureValidToken: async () => {
		const currentState = get(authStore);

		if (!currentState.isAuthenticated || !currentState.expiresAt) {
			return false;
		}

		const now = Date.now();
		const fiveMinutes = 5 * 60 * 1000;

		if (currentState.expiresAt <= now + fiveMinutes) {
			try {
				await auth.refreshToken();
				return true;
			} catch (_error) {
				return false;
			}
		}

		return true;
	},
};

export const isAuthenticated = derived(authStore, ($auth) => $auth.isAuthenticated);
export const currentUser = derived(authStore, ($auth) => $auth.user);
export const accessToken = derived(authStore, ($auth) => $auth.accessToken);
