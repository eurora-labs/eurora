import type { ThemedToken } from 'shiki/core';

import type { ShikiHighlightResponse, ShikiWorkerRequest } from './shiki-worker.js';

export type HighlightResult = ThemedToken[][] | null;

interface PendingRequest {
	code: string;
	lang: string;
	theme: string;
	resolve: (result: HighlightResult) => void;
}

interface KeyState {
	inflightId: number | null;
	queued: PendingRequest | null;
}

/**
 * Singleton bridge between Svelte components and the Shiki Web Worker.
 *
 * Coalescing: at most one inflight + one queued request per `key`. New
 * requests for a key replace the queued one — the dropped queued request
 * resolves with `null` so callers can ignore it. This bounds worker queue
 * depth regardless of how fast chunks arrive.
 */
class ShikiWorkerClient {
	#worker: Worker | null = null;
	#nextRequestId = 1;
	#inflightCallbacks = new Map<number, (tokens: ThemedToken[][]) => void>();
	#stateByKey = new Map<string, KeyState>();

	request(key: string, code: string, lang: string, theme: string): Promise<HighlightResult> {
		return new Promise<HighlightResult>((resolve) => {
			const state = this.#stateByKey.get(key) ?? { inflightId: null, queued: null };
			this.#stateByKey.set(key, state);

			const pending: PendingRequest = { code, lang, theme, resolve };

			if (state.inflightId !== null) {
				// Drop any older queued request for this key — only the most
				// recent snapshot is worth highlighting.
				state.queued?.resolve(null);
				state.queued = pending;
				return;
			}

			this.#dispatch(key, pending);
		});
	}

	/**
	 * Forget a key entirely. Any queued request resolves with `null`; an
	 * inflight request continues to completion (the worker has no abort
	 * primitive) but its result is discarded.
	 */
	release(key: string): void {
		const state = this.#stateByKey.get(key);
		if (!state) return;
		state.queued?.resolve(null);
		this.#stateByKey.delete(key);
	}

	/**
	 * Eagerly boot the worker and load the listed languages. Fire-and-forget
	 * — there is no reply. Call this from app bootstrap so the first real
	 * code block doesn't pay the cold-start cost.
	 */
	warmup(langs: string[]): void {
		const worker = this.#ensureWorker();
		const message: ShikiWorkerRequest = { type: 'warmup', langs };
		worker.postMessage(message);
	}

	#dispatch(key: string, req: PendingRequest): void {
		const worker = this.#ensureWorker();
		const id = this.#nextRequestId++;

		const state = this.#stateByKey.get(key);
		if (state) state.inflightId = id;

		this.#inflightCallbacks.set(id, (tokens) => {
			const cur = this.#stateByKey.get(key);
			// If the key has been released, just drop the result.
			if (!cur || cur.inflightId !== id) {
				req.resolve(null);
				return;
			}

			cur.inflightId = null;
			req.resolve(tokens);

			if (cur.queued) {
				const next = cur.queued;
				cur.queued = null;
				this.#dispatch(key, next);
			}
		});

		const message: ShikiWorkerRequest = {
			type: 'highlight',
			id,
			code: req.code,
			lang: req.lang,
			theme: req.theme,
		};
		worker.postMessage(message);
	}

	#ensureWorker(): Worker {
		if (this.#worker) return this.#worker;

		const worker = new Worker(new URL('./shiki-worker.js', import.meta.url), {
			type: 'module',
			name: 'shiki-highlighter',
		});

		worker.addEventListener('message', (event: MessageEvent<ShikiHighlightResponse>) => {
			const res = event.data;
			if (!res || res.type !== 'highlight' || typeof res.id !== 'number') return;
			const cb = this.#inflightCallbacks.get(res.id);
			if (!cb) return;
			this.#inflightCallbacks.delete(res.id);
			cb(res.tokens);
		});

		worker.addEventListener('error', (event) => {
			console.error('[shiki-worker-client] worker error:', event.message);
		});

		this.#worker = worker;
		return worker;
	}
}

let singleton: ShikiWorkerClient | null = null;

export function getShikiWorkerClient(): ShikiWorkerClient {
	if (typeof Worker === 'undefined') {
		throw new Error('Shiki worker client requires a Worker-capable environment');
	}
	if (!singleton) singleton = new ShikiWorkerClient();
	return singleton;
}

export type { ShikiWorkerClient };
