import { browser } from '$app/environment';
import { authService, type TokenResponse } from '$lib/services/auth-service';
import { create } from '@bufbuild/protobuf';
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

const STORAGE_KEYS = {
	ACCESS_TOKEN: 'eurora_access_token',
	REFRESH_TOKEN: 'eurora_refresh_token',
	EXPIRES_AT: 'eurora_expires_at',
	USER: 'eurora_user',
} as const;

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
		const accessToken = localStorage.getItem(STORAGE_KEYS.ACCESS_TOKEN);
		const refreshToken = localStorage.getItem(STORAGE_KEYS.REFRESH_TOKEN);
		const expiresAt = localStorage.getItem(STORAGE_KEYS.EXPIRES_AT);
		const userStr = localStorage.getItem(STORAGE_KEYS.USER);

		if (accessToken && refreshToken && expiresAt && userStr) {
			const user = JSON.parse(userStr) as User;
			const expiresAtNum = parseInt(expiresAt, 10);
			const now = Date.now();

			const isValid = expiresAtNum > now + 5 * 60 * 1000;

			return {
				isAuthenticated: isValid,
				user: isValid ? user : null,
				accessToken: isValid ? accessToken : null,
				refreshToken,
				expiresAt: expiresAtNum,
			};
		}
	} catch (_error) {
		console.error('Error initializing auth state:', _error);
		clearStoredTokens();
	}

	return {
		isAuthenticated: false,
		user: null,
		accessToken: null,
		refreshToken: null,
		expiresAt: null,
	};
}

const authStore = writable<AuthState>(initializeAuthState());

function clearStoredTokens() {
	if (!browser) return;

	localStorage.removeItem(STORAGE_KEYS.ACCESS_TOKEN);
	localStorage.removeItem(STORAGE_KEYS.REFRESH_TOKEN);
	localStorage.removeItem(STORAGE_KEYS.EXPIRES_AT);
	localStorage.removeItem(STORAGE_KEYS.USER);
}

function storeTokens(tokens: TokenResponse, user: User) {
	if (!browser) return;

	const expiresAt = Date.now() + Number(tokens.expiresIn) * 1000;

	localStorage.setItem(STORAGE_KEYS.ACCESS_TOKEN, tokens.accessToken);
	localStorage.setItem(STORAGE_KEYS.REFRESH_TOKEN, tokens.refreshToken);
	localStorage.setItem(STORAGE_KEYS.EXPIRES_AT, expiresAt.toString());
	localStorage.setItem(STORAGE_KEYS.USER, JSON.stringify(user));
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
		clearStoredTokens();
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
			const tokens = await authService.refreshToken(refreshRequest);

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

if (browser) {
	auth.ensureValidToken().catch((error) => {
		console.error('Failed to ensure valid token on app load:', error);
	});
}
