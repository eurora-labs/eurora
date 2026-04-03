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
	activeThreadId: string | undefined = $state(undefined);
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
			this.activeThreadId = undefined;
		}
	}

	updateThread(thread: Thread) {
		const entry = this.threads.find((t) => t.thread.id === thread.id);
		if (entry) {
			entry.thread = { ...entry.thread, ...thread };
		}
	}

	getThreadData(threadId: string | undefined): ThreadMessages | undefined {
		if (!threadId) return undefined;
		return this.threads.find((t) => t.thread.id === threadId);
	}

	async loadMessages(threadId: string): Promise<void> {
		const entry = this.threads.find((t) => t.thread.id === threadId);
		if (!entry || entry.loading || entry.loaded || entry.streamingMessageId) return;

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

	async sendMessage(text: string): Promise<void> {
		if (!text.trim()) return;

		let threadId = this.activeThreadId;

		if (!threadId) {
			const thread = await this.threadClient.createThread();
			const entry = new ThreadMessages(thread);
			this.threads = [entry, ...this.threads];
			this.activeThreadId = thread.id;
			threadId = thread.id;
			this.newThread = thread;
		}

		const entry = this.getThreadData(threadId);
		if (!entry) return;

		this.appendPlaceholders(entry, text);
		await this.consumeStream(entry, threadId, text);
	}

	async editMessage(messageId: string, text: string): Promise<void> {
		const threadId = this.activeThreadId;
		if (!threadId) return;

		const entry = this.getThreadData(threadId);
		if (!entry) return;

		const nodeIndex = entry.messages.findIndex((n) => n.message.id === messageId);
		if (nodeIndex < 0) return;

		const parentId = entry.messages[nodeIndex].parentId || null;

		entry.messages = entry.messages.slice(0, nodeIndex);
		this.appendPlaceholders(entry, text);
		await this.consumeStream(entry, threadId, text, parentId);
	}

	private async consumeStream(
		entry: ThreadMessages,
		threadId: string,
		text: string,
		parentMessageId?: string | null,
	): Promise<void> {
		const humanNode = entry.messages.at(-2)!;
		const aiNode = entry.messages.at(-1)!;
		const aiMessage = aiNode.message;
		if (aiMessage.type === 'remove') return;

		let hasReceivedContent = false;
		let pendingWhitespace = '';

		try {
			for await (const chunk of this.threadClient.sendMessage(
				threadId,
				text,
				parentMessageId,
			)) {
				if (chunk.chunkPosition === 'last') {
					const kwargs = chunk.additionalKwargs
						? JSON.parse(chunk.additionalKwargs)
						: null;
					if (kwargs?.human_message_id) {
						humanNode.message.id = kwargs.human_message_id;
					}
					if (kwargs?.ai_message_id) {
						aiMessage.id = kwargs.ai_message_id;
					}
					entry.loaded = true;
					break;
				}

				for (const block of chunk.content) {
					if (block.type === 'text') {
						let textContent = block.text;
						if (!hasReceivedContent) {
							if (textContent.trim().length === 0) {
								pendingWhitespace += textContent;
								continue;
							}
							hasReceivedContent = true;
							textContent = pendingWhitespace + textContent;
							pendingWhitespace = '';
						}
						const existing = aiMessage.content.find((b) => b.type === 'text');
						if (existing && existing.type === 'text') {
							existing.text += textContent;
						} else {
							aiMessage.content.push({ ...block, text: textContent });
						}
					} else if (block.type === 'reasoning') {
						const existing = aiMessage.content.find((b) => b.type === 'reasoning');
						if (existing && existing.type === 'reasoning') {
							existing.reasoning =
								(existing.reasoning ?? '') + (block.reasoning ?? '');
						} else {
							aiMessage.content.push({ ...block });
						}
					} else {
						aiMessage.content.push(block);
					}
				}
			}
		} finally {
			entry.streamingMessageId = null;
		}
	}

	private appendPlaceholders(entry: ThreadMessages, text: string): void {
		const now = Date.now();
		const humanId = `temp-${now}-human`;
		const aiId = `temp-${now}-ai`;

		entry.messages = [
			...entry.messages,
			{
				parentId: '',
				message: {
					type: 'human',
					content: [
						{
							type: 'text',
							id: null,
							text,
							annotations: [],
							index: null,
							extras: null,
						},
					],
					id: humanId,
					name: null,
					additionalKwargs: null,
					responseMetadata: null,
				},
				children: [],
				siblingIndex: 0,
				depth: 0,
			},
			{
				parentId: humanId,
				message: {
					type: 'ai',
					content: [],
					id: aiId,
					name: null,
					toolCalls: [],
					invalidToolCalls: [],
					usageMetadata: null,
					additionalKwargs: null,
					responseMetadata: null,
				},
				children: [],
				siblingIndex: 0,
				depth: 0,
			},
		];
		entry.streamingMessageId = aiId;
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
		this.activeThreadId = undefined;
	}
}

export const CHAT_SERVICE = new InjectionToken<ChatService>('ChatService');
