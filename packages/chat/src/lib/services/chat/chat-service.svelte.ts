import { InjectionToken } from '@eurora/shared/context';
import type { MessageNode } from '$lib/models/messages/index.js';
import type { Thread } from '$lib/models/thread.model.js';
import type { BranchDirection, IThreadService } from '$lib/services/thread/thread-service.js';

const PAGE_SIZE = 20;
const MESSAGE_PAGE_SIZE = 50;

export class ThreadMessages {
	thread: Thread;
	messages: MessageNode[] = $state([]);
	loading = $state(false);
	hasMore = $state(true);
	offset = $state(0);
	streamingMessageId: string | null = $state(null);
	loaded = $state(false);

	constructor(thread: Thread) {
		this.thread = thread;
	}
}

export class ChatService {
	newThread: Thread | undefined = $state();
	titleChanged: Thread | undefined = $state();

	threads: ThreadMessages[] = $state([]);
	activeThreadId: string | null = $state(null);
	activeThread = $derived(
		this.activeThreadId ? this.getThreadData(this.activeThreadId) : undefined,
	);
	loadingThreads = $state(false);
	loadingMoreThreads = $state(false);
	hasMoreThreads = $state(true);

	private readonly threadClient: IThreadService;

	private offset = 0;
	private readonly unlisteners: ((() => void) | Promise<() => void>)[] = [];

	constructor(threadClient: IThreadService) {
		this.threadClient = threadClient;
	}

	async loadThreads(limit: number, offset: number) {
		try {
			const fresh = await this.threadClient.listThreads(limit, offset);
			const existing = new Map(this.threads.map((t) => [t.thread.id, t]));
			this.threads = fresh.map(
				(thread) => existing.get(thread.id) ?? new ThreadMessages(thread),
			);
			this.offset = this.threads.length;
			this.hasMoreThreads = this.threads.length === PAGE_SIZE;
		} catch (error) {
			console.error('Failed to load threads:', error);
		} finally {
			this.loadingThreads = false;
		}
	}

	async loadMoreThreads() {
		if (this.loadingMoreThreads || !this.hasMoreThreads) return;
		this.loadingMoreThreads = true;
		try {
			const res = await this.threadClient.listThreads(PAGE_SIZE, this.offset);
			const newThreads = res.map((thread) => new ThreadMessages(thread));
			this.threads = [...this.threads, ...newThreads];
			this.offset += newThreads.length;
			this.hasMoreThreads = newThreads.length === PAGE_SIZE;
		} catch (error) {
			console.error('Failed to load more threads:', error);
		} finally {
			this.loadingMoreThreads = false;
		}
	}

	async deleteThread(threadId: string) {
		await this.threadClient.deleteThread(threadId);
		this.threads = this.threads.filter((t) => t.thread.id !== threadId);
		this.offset = Math.max(0, this.offset - 1);
		if (this.activeThreadId === threadId) {
			this.activeThreadId = null;
		}
	}

	updateThread(thread: Thread) {
		const entry = this.threads.find((t) => t.thread.id === thread.id);
		if (entry) {
			entry.thread = { ...entry.thread, ...thread };
		}
	}

	getThreadData(threadId: string): ThreadMessages | undefined {
		return this.threads.find((t) => t.thread.id === threadId);
	}

	async loadMessages(threadId: string): Promise<void> {
		const entry = this.threads.find((t) => t.thread.id === threadId);
		if (!entry || entry.loading || entry.loaded) return;

		entry.loading = true;
		try {
			const messages = await this.threadClient.getMessages(threadId, MESSAGE_PAGE_SIZE, 0);
			entry.messages = messages;
			entry.offset = messages.length;
			entry.hasMore = messages.length === MESSAGE_PAGE_SIZE;
			entry.loaded = true;
		} catch (error) {
			console.error(`Failed to load messages for thread ${threadId}:`, error);
		} finally {
			entry.loading = false;
		}
	}

	async switchBranch(
		threadId: string,
		messageId: string,
		direction: BranchDirection,
	): Promise<void> {
		const entry = this.threads.find((t) => t.thread.id === threadId);
		if (!entry) return;

		const messages = await this.threadClient.switchBranch(threadId, messageId, direction);
		entry.messages = messages;
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
		this.hasMoreThreads = true;
		this.loadingThreads = false;
		this.activeThreadId = null;
	}
}

export const CHAT_SERVICE = new InjectionToken<ChatService>('ChatService');
