import type { User } from '$lib/stores/auth.js';
import type { Cookies } from '@sveltejs/kit';

const COOKIE_KEYS = {
	ACCESS_TOKEN: 'eurora_access_token',
	REFRESH_TOKEN: 'eurora_refresh_token',
	EXPIRES_AT: 'eurora_expires_at',
	USER: 'eurora_user',
} as const;

export function getAuthFromCookies(cookies: Cookies): {
	user: User | null;
	accessToken: string | null;
	refreshToken: string | null;
	expiresAt: number | null;
} {
	const accessToken = cookies.get(COOKIE_KEYS.ACCESS_TOKEN) ?? null;
	const refreshToken = cookies.get(COOKIE_KEYS.REFRESH_TOKEN) ?? null;
	const expiresAtStr = cookies.get(COOKIE_KEYS.EXPIRES_AT) ?? null;
	const userStr = cookies.get(COOKIE_KEYS.USER) ?? null;

	if (!accessToken || !refreshToken || !expiresAtStr || !userStr) {
		return { user: null, accessToken: null, refreshToken: null, expiresAt: null };
	}

	const expiresAt = parseInt(expiresAtStr, 10);
	const now = Date.now();
	if (expiresAt <= now + 5 * 60 * 1000) {
		return { user: null, accessToken: null, refreshToken: null, expiresAt: null };
	}

	try {
		const user = JSON.parse(decodeURIComponent(userStr)) as User;
		return { user, accessToken, refreshToken, expiresAt };
	} catch {
		return { user: null, accessToken: null, refreshToken: null, expiresAt: null };
	}
}
