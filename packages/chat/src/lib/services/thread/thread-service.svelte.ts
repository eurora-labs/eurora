import { InjectionToken } from '@eurora/shared/context';
import type { ThreadClient } from '$lib/services/thread/thread-client.svelte.js';
import type { Thread } from '@eurora/shared/proto/thread_service_pb.js';

const PAGE_SIZE = 20;
const MAX_LOAD_RETRIES = 3;

export class ThreadService {
	threads: Thread[] = $state([]);
	loading = $state(true);
	loadingMore = $state(false);
	hasMore = $state(true);
	activeThreadId: string | null = $state(null);

	private offset = 0;
	private loadRetries = 0;
	private readonly client: ThreadClient;
	private readonly unlisteners: ((() => void) | Promise<() => void>)[] = [];

	constructor(client: ThreadClient) {
		this.client = client;
	}

	async init() {
		try {
			const res = await this.client.list(PAGE_SIZE, 0);
			this.threads = res;
			this.offset = res.length;
			this.hasMore = res.length === PAGE_SIZE;
		} catch (error) {
			console.error('Failed to load threads:', error);
		} finally {
			this.loading = false;
		}

		this.unlisteners.push(
			this.client.newThread$.subscribe((thread) => {
				if (thread && !this.threads.some((t) => t.id === thread.id)) {
					this.threads = [thread, ...this.threads];
					this.offset += 1;
					this.activeThreadId = thread.id;
				}
			}),
			this.client.titleChanged$.subscribe((thread) => {
				this.threads = this.threads.map((t) =>
					t.id === thread.id ? { ...t, title: thread.title } : t,
				);
			}),
			this.client.currentThreadChanged$.subscribe((thread) => {
				this.activeThreadId = thread.id;
			}),
		);
	}

	addThread(thread: Thread) {
		if (!this.threads.some((t) => t.id === thread.id)) {
			this.threads = [thread, ...this.threads];
			this.offset += 1;
		}
		this.activeThreadId = thread.id;
	}

	updateThread(thread: Thread) {
		this.threads = this.threads.map((t) =>
			t.id === thread.id ? { ...t, title: thread.title } : t,
		);
	}

	async deleteThread(threadId: string) {
		await this.client.delete(threadId);
		this.threads = this.threads.filter((t) => t.id !== threadId);
		this.offset = Math.max(0, this.offset - 1);
		if (this.activeThreadId === threadId) {
			this.activeThreadId = null;
		}
	}

	async loadMore() {
		if (this.loadingMore || !this.hasMore) return;
		this.loadingMore = true;
		try {
			const res = await this.client.list(PAGE_SIZE, this.offset);
			this.threads = [...this.threads, ...res];
			this.offset += res.length;
			this.hasMore = res.length === PAGE_SIZE;
			this.loadRetries = 0;
		} catch (error) {
			console.error('Failed to load more threads:', error);
			this.loadRetries += 1;
			if (this.loadRetries >= MAX_LOAD_RETRIES) {
				this.hasMore = false;
			}
		} finally {
			this.loadingMore = false;
		}
	}

	destroy() {
		for (const p of this.unlisteners) {
			if (p instanceof Promise) {
				p.then((unlisten) => unlisten());
			} else {
				p();
			}
		}
		this.unlisteners.length = 0;
		this.threads = [];
		this.offset = 0;
		this.loadRetries = 0;
		this.hasMore = true;
		this.loading = true;
		this.loadingMore = false;
		this.activeThreadId = null;
	}
}

export const THREAD_SERVICE = new InjectionToken<ThreadService>('ThreadService');
