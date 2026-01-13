import { browser } from '$app/environment';
import { create } from '@bufbuild/protobuf';
import { RefreshTokenRequestSchema } from '@eurora/shared/proto/auth_service_pb.js';
import { authService } from '@eurora/shared/services/auth-service';
import { writable, derived, get } from 'svelte/store';
import type { TokenResponse } from '@eurora/shared/services/auth-service';

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

// Initialize auth state from localStorage if available
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

			// Check if token is still valid (with 5 minute buffer)
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
		// Clear corrupted data
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

// Create the auth store
const authStore = writable<AuthState>(initializeAuthState());

// Helper function to clear stored tokens
function clearStoredTokens() {
	if (!browser) return;

	localStorage.removeItem(STORAGE_KEYS.ACCESS_TOKEN);
	localStorage.removeItem(STORAGE_KEYS.REFRESH_TOKEN);
	localStorage.removeItem(STORAGE_KEYS.EXPIRES_AT);
	localStorage.removeItem(STORAGE_KEYS.USER);
}

// Helper function to store tokens
function storeTokens(tokens: TokenResponse, user: User) {
	if (!browser) return;

	const expiresAt = Date.now() + Number(tokens.expiresIn) * 1000;

	localStorage.setItem(STORAGE_KEYS.ACCESS_TOKEN, tokens.accessToken);
	localStorage.setItem(STORAGE_KEYS.REFRESH_TOKEN, tokens.refreshToken);
	localStorage.setItem(STORAGE_KEYS.EXPIRES_AT, expiresAt.toString());
	localStorage.setItem(STORAGE_KEYS.USER, JSON.stringify(user));
}

// Helper function to decode JWT payload (basic implementation)
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

// Auth actions
export const auth = {
	// Subscribe to auth state
	subscribe: authStore.subscribe,

	// Login with tokens
	login: (tokens: TokenResponse) => {
		try {
			// Decode user info from access token
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

			// Store tokens in localStorage
			storeTokens(tokens, user);

			// Update store
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

	// Logout
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

	// Refresh token
	refreshToken: async () => {
		const currentState = get(authStore);

		if (!currentState.refreshToken) {
			throw new Error('No refresh token available');
		}

		try {
			const refreshRequest = create(RefreshTokenRequestSchema, {});
			const tokens = await authService.refreshToken(refreshRequest);

			// Update tokens while keeping user info
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
			// If refresh fails, logout user
			auth.logout();
			throw error;
		}
	},

	// Check if token needs refresh and refresh if necessary
	ensureValidToken: async () => {
		const currentState = get(authStore);

		if (!currentState.isAuthenticated || !currentState.expiresAt) {
			return false;
		}

		// Check if token expires in the next 5 minutes
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

// Derived stores for convenience
export const isAuthenticated = derived(authStore, ($auth) => $auth.isAuthenticated);
export const currentUser = derived(authStore, ($auth) => $auth.user);
export const accessToken = derived(authStore, ($auth) => $auth.accessToken);

// Auto-refresh token on app load if needed
if (browser) {
	auth.ensureValidToken().catch((error) => {
		console.error('Failed to ensure valid token on app load:', error);
	});
}
