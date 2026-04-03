import { InjectionToken } from '@eurora/shared/context';
import type { Thread } from '$lib/models/thread.model.js';
import type { IThreadService } from '$lib/services/thread/thread-service.js';
import type { BaseMessageWithSibling } from '@eurora/shared/proto/agent_chain_pb.js';
import type {
	ListThreadsRequest,
	DeleteThreadRequest,
} from '@eurora/shared/proto/thread_service_pb.js';

const PAGE_SIZE = 20;
const MESSAGE_PAGE_SIZE = 50;

export class ThreadMessages {
	thread: Thread;
	messages: BaseMessageWithSibling[] = $state([]);
	loading = $state(false);
	hasMore = $state(true);
	offset = 0;
	streaming = $state(false);
	loaded = false;

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
	hasMoreThreads = $derived(this.activeThread ? this.activeThread.hasMore : false);

	private readonly threadClient: IThreadService;

	private offset = 0;
	private loadRetries = 0;
	private readonly unlisteners: ((() => void) | Promise<() => void>)[] = [];

	constructor(threadClient: IThreadService) {
		this.threadClient = threadClient;
	}

	async loadThreads(limit: number, offset: number) {
		try {
			this.threads = (
				await this.threadClient.listThreads({ limit, offset } as ListThreadsRequest)
			).map((thread) => new ThreadMessages(thread));
			this.offset = this.threads.length;
			this.hasMoreThreads = this.threads.length === PAGE_SIZE;
		} catch (error) {
			console.error('Failed to load threads:', error);
		} finally {
			this.loadingThreads = false;
		}
	}

	async loadMoreThreads() {
		if (this.loadingThreads || !this.hasMoreThreads) return;
		this.loadingThreads = true;
		try {
			const res = await this.threadClient.listThreads({
				limit: PAGE_SIZE,
				offset: this.offset,
			} as ListThreadsRequest);
			const newThreads = res.map((thread) => new ThreadMessages(thread));
			this.threads = [...this.threads, ...newThreads];
			this.offset += newThreads.length;
			this.hasMoreThreads = newThreads.length === PAGE_SIZE;
			this.loadRetries = 0;
		} catch (error) {
			console.error('Failed to load more threads:', error);
			this.loadRetries += 1;
		} finally {
			this.loadingThreads = false;
		}
	}

	async deleteThread(threadId: string) {
		await this.threadClient.deleteThread({ threadId } as DeleteThreadRequest);
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

	async switchBranch(threadId: string, messageId: string, direction: number): Promise<void> {
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
		this.loadRetries = 0;
		this.hasMoreThreads = true;
		this.loadingThreads = false;
		this.activeThreadId = null;
	}
}

export const CHAT_SERVICE = new InjectionToken<ChatService>('ChatService');
