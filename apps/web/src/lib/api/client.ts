/**
 * Thin typed HTTP client used by the web app to talk to the Eurora monolith.
 *
 * The wire types come from the workspace-level `*-core` crates via the
 * `pnpm specta:backend` codegen step (see `packages/shared/src/lib/bindings`).
 * This helper centralises Bearer-auth, JSON encoding, and uniform error
 * decoding so per-service callers stay declarative.
 */

import type { ConfigService } from '@eurora/shared/config/config-service';

/**
 * Common error envelope emitted by every backend HTTP service.
 *
 * Currently mirrors `auth-core`'s `AuthErrorResponse` shape; the activity /
 * thread / asset services use the same field names, so a single decoder works
 * across services. The fully typed per-service variants live in
 * `@eurora/shared/bindings/<service>` if a caller needs them.
 */
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

	/**
	 * Stable machine-readable kind, e.g. `"invalid_credentials"`. Falls back to
	 * `"http_<status>"` when the server didn't return a JSON envelope.
	 */
	get kind(): string {
		return this.body?.error ?? `http_${this.status}`;
	}
}

export interface ApiRequestInit<TBody> {
	method?: 'GET' | 'POST' | 'PUT' | 'PATCH' | 'DELETE';
	/** JSON-serialised request body. Pass `undefined` for bodyless requests. */
	body?: TBody;
	/** Query parameters appended to the URL. Skipped when nullish. */
	query?: Record<string, string | number | boolean | null | undefined>;
	/** Bearer token attached to the `Authorization` header. */
	bearerToken?: string | null;
	/** Extra headers; merged after the default `Content-Type` / `Accept`. */
	headers?: HeadersInit;
	signal?: AbortSignal;
}

export class ApiClient {
	readonly #config: ConfigService;

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
		if (init.bearerToken) headers.set('Authorization', `Bearer ${init.bearerToken}`);

		const res = await fetch(url, {
			method: init.method ?? (hasBody ? 'POST' : 'GET'),
			headers,
			body: hasBody ? JSON.stringify(init.body) : undefined,
			signal: init.signal,
		});

		if (!res.ok) {
			const body = (await res.json().catch(() => null)) as ApiErrorBody | null;
			const message = body?.message ?? `${res.status} ${res.statusText}`;
			throw new ApiError(res.status, body, message);
		}

		// 204 / no-content endpoints (e.g. logout) intentionally return nothing;
		// the caller declares `TResponse = void` and we resolve to `undefined`.
		if (res.status === 204) return undefined as TResponse;
		const text = await res.text();
		if (!text) return undefined as TResponse;
		return JSON.parse(text) as TResponse;
	}
}

function joinPath(base: string, path: string): string {
	if (!path) return base;
	if (/^https?:\/\//i.test(path)) return path;
	const trimmedBase = base.replace(/\/+$/, '');
	const trimmedPath = path.startsWith('/') ? path : `/${path}`;
	return `${trimmedBase}${trimmedPath}`;
}
