import { InjectionToken } from '@eurora/shared/context';
import type { ThreadView } from '$lib/bindings/bindings.js';
import type { TaurpcService } from '$lib/bindings/taurpcService.js';

const PAGE_SIZE = 20;
const MAX_LOAD_RETRIES = 3;

export class ThreadService {
	threads: ThreadView[] = $state([]);
	loading = $state(true);
	loadingMore = $state(false);
	hasMore = $state(true);
	activeThreadId: string | null = $state(null);

	private offset = 0;
	private loadRetries = 0;
	private readonly taurpc: TaurpcService;
	private readonly unlisteners: Promise<() => void>[] = [];

	constructor(taurpc: TaurpcService) {
		this.taurpc = taurpc;
	}

	async init() {
		try {
			const res = await this.taurpc.thread.list(PAGE_SIZE, 0);
			this.threads = res;
			this.offset = res.length;
			this.hasMore = res.length === PAGE_SIZE;
		} catch (error) {
			console.error('Failed to load threads:', error);
		} finally {
			this.loading = false;
		}

		this.unlisteners.push(
			this.taurpc.thread.new_thread_added.on((thread) => {
				if (!this.threads.some((t) => t.id === thread.id)) {
					this.threads = [thread, ...this.threads];
					this.offset += 1;
				}
				this.activeThreadId = thread.id;
			}),
			this.taurpc.thread.thread_title_changed.on((thread) => {
				this.threads = this.threads.map((t) =>
					t.id === thread.id ? { ...t, title: thread.title } : t,
				);
			}),
			this.taurpc.thread.current_thread_changed.on((thread) => {
				this.activeThreadId = thread.id;
			}),
		);
	}

	createThread() {
		this.activeThreadId = null;
	}

	async loadMore() {
		if (this.loadingMore || !this.hasMore) return;
		this.loadingMore = true;
		try {
			const res = await this.taurpc.thread.list(PAGE_SIZE, this.offset);
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
			p.then((unlisten) => unlisten());
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
