import { type IThreadService } from '@eurora/chat/services/thread/thread-service';
import { InjectionToken } from '@eurora/shared/context';
import type { TaurpcService } from '$lib/bindings/taurpcService.js';
import type { Thread } from '@eurora/chat/models/thread.model';
import type { BaseMessageWithSibling } from '@eurora/shared/proto/agent_chain_pb.js';
import type {
	DeleteThreadRequest,
	ListThreadsRequest,
} from '@eurora/shared/proto/thread_service_pb.js';

const PAGE_SIZE = 20;
const MAX_LOAD_RETRIES = 3;

export class ThreadService implements IThreadService {
	taurpc: TaurpcService;
	constructor(taurpc: TaurpcService) {
		this.taurpc = taurpc;
	}

	async listThreads(request: ListThreadsRequest): Promise<Thread[]> {
		return (await this.taurpc.thread.list(request.limit, request.offset)).map(
			(thread) =>
				({
					id: thread.id,
					title: thread.title,
					createdAt: thread.created_at,
					updatedAt: thread.updated_at,
				}) as Thread,
		);
	}

	async loadMoreMessages(threadId: string): Promise<void> {
		const entry = this.cache.get(threadId);
		if (!entry || entry.loading || !entry.hasMore) return;

		entry.loading = true;
		try {
			const messages = await this.taurpc.thread.get_messages(
				threadId,
				PAGE_SIZE,
				entry.offset,
			);
			const insertOffset = entry.messages.length;
			entry.messages = [...entry.messages, ...messages];
			entry.offset += messages.length;
			entry.hasMore = messages.length === PAGE_SIZE;
			this.extractReasoning(entry, messages, insertOffset);
		} catch (error) {
			console.error(`Failed to load more messages for thread ${threadId}:`, error);
		} finally {
			entry.loading = false;
		}
	}

	deleteThread(request: DeleteThreadRequest): Promise<void> {
		throw new Error('Method not implemented.');
	}
}

// export class ThreadService {
// 	threads: Thread[] = $state([]);
// 	loading = $state(true);
// 	loadingMore = $state(false);
// 	hasMore = $state(true);
// 	activeThreadId: string | null = $state(null);

// 	private offset = 0;
// 	private loadRetries = 0;
// 	private readonly taurpc: TaurpcService;
// 	private readonly unlisteners: Promise<() => void>[] = [];

// 	constructor(taurpc: TaurpcService) {
// 		this.taurpc = taurpc;
// 	}

// 	async init() {
// 		try {
// 			const res = await this.taurpc.thread.list(PAGE_SIZE, 0);
// 			this.threads = res;
// 			this.offset = res.length;
// 			this.hasMore = res.length === PAGE_SIZE;
// 		} catch (error) {
// 			console.error('Failed to load threads:', error);
// 		} finally {
// 			this.loading = false;
// 		}

// 		this.unlisteners.push(
// 			this.taurpc.thread.new_thread_added.on((thread) => {
// 				if (!this.threads.some((t) => t.id === thread.id)) {
// 					this.threads = [thread, ...this.threads];
// 					this.offset += 1;
// 				}
// 				this.activeThreadId = thread.id;
// 			}),
// 			this.taurpc.thread.thread_title_changed.on((thread) => {
// 				this.threads = this.threads.map((t) =>
// 					t.id === thread.id ? { ...t, title: thread.title } : t,
// 				);
// 			}),
// 			this.taurpc.thread.current_thread_changed.on((thread) => {
// 				this.activeThreadId = thread.id;
// 			}),
// 		);
// 	}

// 	addThread(thread: Thread) {
// 		if (!this.threads.some((t) => t.id === thread.id)) {
// 			this.threads = [thread, ...this.threads];
// 			this.offset += 1;
// 		}
// 		this.activeThreadId = thread.id;
// 	}

// 	updateThread(thread: Thread) {
// 		this.threads = this.threads.map((t) =>
// 			t.id === thread.id ? { ...t, title: thread.title } : t,
// 		);
// 	}

// 	async deleteThread(threadId: string) {
// 		await this.taurpc.thread.delete(threadId);
// 		this.threads = this.threads.filter((t) => t.id !== threadId);
// 		this.offset = Math.max(0, this.offset - 1);
// 		if (this.activeThreadId === threadId) {
// 			this.activeThreadId = null;
// 		}
// 	}

// 	async loadMore() {
// 		if (this.loadingMore || !this.hasMore) return;
// 		this.loadingMore = true;
// 		try {
// 			const res = await this.taurpc.thread.list(PAGE_SIZE, this.offset);
// 			this.threads = [...this.threads, ...res];
// 			this.offset += res.length;
// 			this.hasMore = res.length === PAGE_SIZE;
// 			this.loadRetries = 0;
// 		} catch (error) {
// 			console.error('Failed to load more threads:', error);
// 			this.loadRetries += 1;
// 			if (this.loadRetries >= MAX_LOAD_RETRIES) {
// 				this.hasMore = false;
// 			}
// 		} finally {
// 			this.loadingMore = false;
// 		}
// 	}

// 	destroy() {
// 		for (const p of this.unlisteners) {
// 			p.then((unlisten) => unlisten());
// 		}
// 		this.unlisteners.length = 0;
// 		this.threads = [];
// 		this.offset = 0;
// 		this.loadRetries = 0;
// 		this.hasMore = true;
// 		this.loading = true;
// 		this.loadingMore = false;
// 		this.activeThreadId = null;
// 	}
// }

export const THREAD_SERVICE = new InjectionToken<ThreadService>('ThreadService');
