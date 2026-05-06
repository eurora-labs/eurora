/**
 * Thin typed HTTP client used by the SPA to talk to the Eurora monolith.
 *
 * Every request carries `credentials: 'include'` so the browser sends
 * the HttpOnly `eu_access` (and, for `/auth` calls, `eu_refresh`)
 * cookies set by the backend. The SPA holds no tokens in JS.
 *
 * The wire types come from the workspace-level `*-core` crates via the
 * `pnpm specta:backend` codegen step (see `packages/shared/src/lib/bindings`).
 *
 * CSRF protection is enforced by the backend through an Origin
 * allowlist plus `SameSite=Lax` on the session cookies — the SPA does
 * not need to attach any CSRF token header.
 *
 * On a 401 the client transparently calls `POST /auth/refresh` once and
 * replays the original request. Concurrent 401s share a single refresh
 * promise so we don't stampede the auth service.
 */
import type { ConfigService } from '@eurora/shared/config/config-service';

export interface ApiErrorBody {
	error: string;
	message: string;
	details?: string | null;
}

export class ApiError extends Error {
	readonly status: number;
	readonly body: ApiErrorBody | null;

	constructor(status: number, body: ApiErrorBody | null, message: string) {
		super(message);
		this.name = 'ApiError';
		this.status = status;
		this.body = body;
	}

	get kind(): string {
		return this.body?.error ?? `http_${this.status}`;
	}
}

export interface ApiRequestInit<TBody> {
	method?: 'GET' | 'POST' | 'PUT' | 'PATCH' | 'DELETE';
	body?: TBody;
	query?: Record<string, string | number | boolean | null | undefined>;
	headers?: HeadersInit;
	signal?: AbortSignal;
	/**
	 * Skip the transparent 401-refresh dance. Used by `/auth/refresh`
	 * itself (would loop) and `/auth/me` during boot (a 401 there is
	 * the legitimate "you're logged out" signal).
	 */
	skipAuthRefresh?: boolean;
}

const REFRESH_PATH = '/auth/refresh';

/**
 * Optional callback the SPA hooks up to `AuthService.logout()` so a
 * failed transparent refresh tears down the in-memory user state in
 * sync with the backend's cookie clearing.
 */
let onSessionExpired: (() => void) | null = null;

export function setSessionExpiredHandler(handler: (() => void) | null): void {
	onSessionExpired = handler;
}

export class ApiClient {
	readonly #config: ConfigService;
	#refreshInflight: Promise<boolean> | null = null;

	constructor(config: ConfigService) {
		this.#config = config;
	}

	get baseUrl(): string {
		return this.#config.apiUrl;
	}

	async fetch<TResponse, TBody = undefined>(
		path: string,
		init: ApiRequestInit<TBody> = {},
	): Promise<TResponse> {
		const res = await this.#send(path, init);

		if (res.status === 401 && !init.skipAuthRefresh && path !== REFRESH_PATH) {
			const refreshed = await this.#tryRefresh();
			if (refreshed) {
				const replay = await this.#send(path, init);
				return await this.#parse<TResponse>(replay);
			}
			onSessionExpired?.();
		}

		return await this.#parse<TResponse>(res);
	}

	async #send<TBody>(path: string, init: ApiRequestInit<TBody>): Promise<Response> {
		const url = new URL(joinPath(this.#config.apiUrl, path));
		if (init.query) {
			for (const [key, value] of Object.entries(init.query)) {
				if (value === null || value === undefined) continue;
				url.searchParams.set(key, String(value));
			}
		}

		const headers = new Headers(init.headers);
		headers.set('Accept', 'application/json');
		const hasBody = init.body !== undefined;
		if (hasBody) headers.set('Content-Type', 'application/json');

		const method = init.method ?? (hasBody ? 'POST' : 'GET');

		return await fetch(url, {
			method,
			headers,
			credentials: 'include',
			body: hasBody ? JSON.stringify(init.body) : undefined,
			signal: init.signal,
		});
	}

	async #parse<TResponse>(res: Response): Promise<TResponse> {
		if (!res.ok) {
			const body = (await res.json().catch(() => null)) as ApiErrorBody | null;
			const message = body?.message ?? `${res.status} ${res.statusText}`;
			throw new ApiError(res.status, body, message);
		}
		if (res.status === 204) return undefined as TResponse;
		const text = await res.text();
		if (!text) return undefined as TResponse;
		return JSON.parse(text) as TResponse;
	}

	/**
	 * Issue a single in-flight refresh request, deduplicating
	 * concurrent callers. Returns `true` on success.
	 */
	async #tryRefresh(): Promise<boolean> {
		this.#refreshInflight ??= this.#send(REFRESH_PATH, {
			method: 'POST',
			skipAuthRefresh: true,
		})
			.then((res) => res.ok)
			.catch(() => false)
			.finally(() => {
				this.#refreshInflight = null;
			});
		return await this.#refreshInflight;
	}
}

function joinPath(base: string, path: string): string {
	if (!path) return base;
	if (/^https?:\/\//i.test(path)) return path;
	const trimmedBase = base.replace(/\/+$/, '');
	const trimmedPath = path.startsWith('/') ? path : `/${path}`;
	return `${trimmedBase}${trimmedPath}`;
}
