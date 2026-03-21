import { getAuthFromCookies } from '$lib/server/auth.js';
import type { Handle } from '@sveltejs/kit';

export async function handle({ event, resolve }: Parameters<Handle>[0]) {
	const { user, accessToken, refreshToken, expiresAt } = getAuthFromCookies(event.cookies);
	event.locals.user = user;
	event.locals.accessToken = accessToken;
	event.locals.refreshToken = refreshToken;
	event.locals.expiresAt = expiresAt;

	return await resolve(event);
}
