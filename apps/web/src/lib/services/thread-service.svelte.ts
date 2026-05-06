import { ApiClient } from '$lib/api/client.js';
import type { MessageNode } from '@eurora/chat/models/messages/index';
import type { MessageSearchResult, ThreadSearchResult } from '@eurora/chat/models/search.model';
import type { ChatServerMessage } from '@eurora/chat/models/streaming';
import type { Thread } from '@eurora/chat/models/thread.model';
import type {
	BranchDirection,
	ChatContext,
	IThreadService,
} from '@eurora/chat/services/thread/thread-service';
import type {
	ChatClientMessage,
	ChatSendRequest,
	CreateThreadRequest,
	CreateThreadResponse,
	GenerateThreadTitleRequest,
	GenerateThreadTitleResponse,
	GetMessagesResponse,
	ListThreadsResponse,
	SearchMessagesResponse,
	SearchThreadsResponse,
	SwitchBranchRequest,
} from '@eurora/shared/bindings/thread';
import type { ConfigService } from '@eurora/shared/config/config-service';

/**
 * SPA-side thread service. CRUD goes over REST (JSON, cookie auth, transparent
 * 401-refresh via [`ApiClient`]); chat streaming opens a WebSocket to
 * `/threads/{id}/chat` and yields the wire `ChatServerMessage` envelopes
 * unchanged.
 *
 * Same-origin/same-site cookies are sent automatically with the WebSocket
 * upgrade handshake, so no token plumbing is needed here. CSRF protection
 * applies to mutating HTTP requests only — the upgrade is a GET.
 */
export class ThreadService implements IThreadService {
	readonly #api: ApiClient;

	constructor(config: ConfigService) {
		this.#api = new ApiClient(config);
	}

	async listThreads(limit: number, offset: number): Promise<Thread[]> {
		const resp = await this.#api.fetch<ListThreadsResponse>('/threads', {
			query: { limit, offset },
		});
		return resp.threads;
	}

	async getMessages(
		threadId: string,
		limit: number,
		offset: number,
		allVariants: boolean,
	): Promise<MessageNode[]> {
		const resp = await this.#api.fetch<GetMessagesResponse>(`/threads/${threadId}/messages`, {
			query: { limit, offset, all_variants: allVariants },
		});
		return resp.messages;
	}

	async switchBranch(
		threadId: string,
		messageId: string,
		direction: BranchDirection,
	): Promise<MessageNode[]> {
		const body: SwitchBranchRequest = { message_id: messageId, direction };
		const resp = await this.#api.fetch<GetMessagesResponse, SwitchBranchRequest>(
			`/threads/${threadId}/messages/switch-branch`,
			{ body },
		);
		return resp.messages;
	}

	async deleteThread(threadId: string): Promise<void> {
		await this.#api.fetch<void>(`/threads/${threadId}`, { method: 'DELETE' });
	}

	async createThread(): Promise<Thread> {
		const resp = await this.#api.fetch<CreateThreadResponse, CreateThreadRequest>('/threads', {
			body: {},
		});
		return resp.thread;
	}

	async generateTitle(threadId: string): Promise<Thread> {
		const resp = await this.#api.fetch<GenerateThreadTitleResponse, GenerateThreadTitleRequest>(
			`/threads/${threadId}/title`,
			{ body: {} },
		);
		return resp.thread;
	}

	async searchThreads(
		query: string,
		limit: number,
		offset: number,
	): Promise<ThreadSearchResult[]> {
		const resp = await this.#api.fetch<SearchThreadsResponse>('/threads/search', {
			query: { q: query, limit, offset },
		});
		return resp.results;
	}

	async searchMessages(
		query: string,
		limit: number,
		offset: number,
	): Promise<MessageSearchResult[]> {
		const resp = await this.#api.fetch<SearchMessagesResponse>('/threads/messages/search', {
			query: { q: query, limit, offset },
		});
		return resp.results;
	}

	async collectContext(_threadId: string): Promise<ChatContext> {
		return { contentBlocks: [], assetChips: [] };
	}

	async *sendMessage(
		threadId: string,
		request: ChatSendRequest,
		signal?: AbortSignal,
	): AsyncIterable<ChatServerMessage> {
		const ws = new WebSocket(deriveWsUrl(this.#api.baseUrl, `/threads/${threadId}/chat`));

		const buffer: ChatServerMessage[] = [];
		let resolve: ((value: void) => void) | null = null;
		let finished = false;
		let error: unknown = null;

		function notify() {
			resolve?.();
			resolve = null;
		}

		ws.addEventListener('open', () => {
			const frame: ChatClientMessage = { type: 'send', ...request };
			ws.send(JSON.stringify(frame));
		});

		ws.addEventListener('message', (ev: MessageEvent<string>) => {
			try {
				buffer.push(JSON.parse(ev.data) as ChatServerMessage);
			} catch (e) {
				error = e;
				finished = true;
			}
			notify();
		});

		ws.addEventListener('error', () => {
			error = new Error('Chat WebSocket error');
			finished = true;
			notify();
		});

		ws.addEventListener('close', () => {
			finished = true;
			notify();
		});

		function onAbort() {
			if (ws.readyState === WebSocket.OPEN) {
				try {
					const cancel: ChatClientMessage = { type: 'cancel' };
					ws.send(JSON.stringify(cancel));
				} catch {
					// Already closing — nothing to do.
				}
			}
			try {
				ws.close();
			} catch {
				// Already closed.
			}
			notify();
		}
		signal?.addEventListener('abort', onAbort, { once: true });

		try {
			while (true) {
				while (buffer.length > 0) {
					if (signal?.aborted) return;
					yield buffer.shift()!;
				}
				if (finished) break;
				if (signal?.aborted) return;
				await new Promise<void>((r) => {
					resolve = r;
				});
			}

			while (buffer.length > 0) yield buffer.shift()!;
			if (error) throw error;
		} finally {
			signal?.removeEventListener('abort', onAbort);
			if (ws.readyState !== WebSocket.CLOSED && ws.readyState !== WebSocket.CLOSING) {
				try {
					ws.close();
				} catch {
					// Already closed.
				}
			}
		}
	}
}

function deriveWsUrl(apiUrl: string, path: string): string {
	const u = new URL(path, ensureTrailingSlash(apiUrl));
	u.protocol = u.protocol === 'https:' ? 'wss:' : 'ws:';
	return u.toString();
}

function ensureTrailingSlash(url: string): string {
	return url.endsWith('/') ? url : `${url}/`;
}
